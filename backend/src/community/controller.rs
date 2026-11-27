use std::collections::HashMap;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use serde_json::{Value, json};

use super::access::{MANAGER_ROLES, Repos, require_community_role};
use super::models::{CommunityCategory, CommunityVisibility};
use super::usecases::{
    CreateCommunityCommand, JoinCommunityCommand, LeaveCommunityCommand, ListCommunitiesQueryInput,
    ReviewMemberCommand, approve_member_command, create_community_command, get_community_query,
    join_community_command, leave_community_command, list_communities_query,
    list_members_read_query, reject_member_command,
};
use crate::app::AppState;
use crate::shared::auth::{optional_account, require_account};
use crate::shared::error::{ApiError, ApiResult};
use crate::shared::time::to_iso;
use crate::shared::validation::{
    json_object, optional_nullable_string, page_params, required_enum, required_string, uuid_param,
};

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ============================================================
// コミュニティ
// ============================================================

/// POST /communities - コミュニティ作成
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<Value>)> {
    // 検証（400）→ 認証（401）の順（旧 OpenAPI バリデータ → requireAuth と同じ）
    let obj = json_object(&body)?;
    let name = required_string(&obj, "name", 1, 100)?;
    let description = optional_nullable_string(&obj, "description", 1000)?;
    let category = required_enum(&obj, "category", CommunityCategory::ALL)?;
    let visibility = required_enum(&obj, "visibility", CommunityVisibility::ALL)?;
    let account_id = require_account(&headers, &state.jwt_secret)?;

    let now = chrono::Utc::now();
    let repos = Repos::new(&state);
    let (community, _owner) = create_community_command(
        &repos.ctx(&state),
        CreateCommunityCommand {
            id: new_id(),
            owner_member_id: new_id(),
            account_id,
            name,
            description,
            category: CommunityCategory::parse(category).unwrap_or(CommunityCategory::Tech),
            visibility: CommunityVisibility::parse(visibility)
                .unwrap_or(CommunityVisibility::Public),
            created_at: now,
            updated_at: now,
        },
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "community": community.to_response() })),
    ))
}

/// GET /communities - コミュニティ一覧取得
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let (limit, offset) = page_params(&query)?;
    let category = match query.get("category") {
        Some(c) if CommunityCategory::parse(c).is_some() => Some(c.as_str()),
        Some(_) => {
            return Err(ApiError::validation(
                "request/query/category must be equal to one of the allowed values: TECH, BUSINESS, HOBBY",
            ));
        }
        None => None,
    };
    let member_me = match query.get("member").map(String::as_str) {
        Some("me") => true,
        Some(_) => {
            return Err(ApiError::validation(
                "request/query/member must be equal to one of the allowed values: me",
            ));
        }
        None => false,
    };

    let account_id = optional_account(&headers, &state.jwt_secret);
    // ?member=me は認証が必要
    if member_me && account_id.is_none() {
        return Err(ApiError::unauthorized());
    }

    let repos = Repos::new(&state);
    let (communities, total) = list_communities_query(
        &repos.ctx(&state),
        ListCommunitiesQueryInput {
            category,
            member_account_id: if member_me {
                account_id.as_deref()
            } else {
                None
            },
            limit,
            offset,
        },
    )
    .await?;
    Ok(Json(json!({
        "communities": communities.iter().map(|c| c.to_response()).collect::<Vec<_>>(),
        "total": total,
    })))
}

/// GET /communities/:id - コミュニティ取得
pub async fn get_one(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id = uuid_param("id", &id)?;
    let account_id = optional_account(&headers, &state.jwt_secret);
    let repos = Repos::new(&state);
    let community = get_community_query(&repos.ctx(&state), &id, account_id.as_deref()).await?;
    Ok(Json(json!({ "community": community.to_response() })))
}

// ============================================================
// メンバー
// ============================================================

/// POST /communities/:id/members - コミュニティ参加
pub async fn join(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let id = uuid_param("id", &id)?;
    let account_id = require_account(&headers, &state.jwt_secret)?;
    let repos = Repos::new(&state);
    let member = join_community_command(
        &repos.ctx(&state),
        JoinCommunityCommand {
            community_id: id,
            account_id,
            member_id: new_id(),
            occurred_at: chrono::Utc::now(),
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(member.to_response())))
}

async fn leave(
    state: &AppState,
    headers: &HeaderMap,
    community_id: String,
    member_id: Option<String>,
) -> ApiResult<Json<Value>> {
    let account_id = require_account(headers, &state.jwt_secret)?;
    let repos = Repos::new(state);
    leave_community_command(
        &repos.ctx(state),
        LeaveCommunityCommand {
            community_id,
            account_id,
            member_id,
            occurred_at: chrono::Utc::now(),
        },
    )
    .await?;
    Ok(Json(json!({})))
}

/// DELETE /communities/:id/members/me - コミュニティ脱退
pub async fn leave_me(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id = uuid_param("id", &id)?;
    leave(&state, &headers, id, None).await
}

/// DELETE /communities/:id/members/:memberId - メンバー ID 指定で脱退
pub async fn leave_by_member_id(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, member_id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let id = uuid_param("id", &id)?;
    let member_id = uuid_param("memberId", &member_id)?;
    leave(&state, &headers, id, Some(member_id)).await
}

/// GET /communities/:id/members - メンバー一覧（Read モデル）
pub async fn list_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let id = uuid_param("id", &id)?;
    let (limit, offset) = page_params(&query)?;
    let account_id = optional_account(&headers, &state.jwt_secret);
    let repos = Repos::new(&state);
    let (items, total) = list_members_read_query(
        &repos.ctx(&state),
        &id,
        account_id.as_deref(),
        limit,
        offset,
    )
    .await?;

    let members: Vec<Value> = items
        .iter()
        .map(|item| {
            let r = &item.row;
            let mut v = json!({
                "id": r.id,
                "communityId": r.community_id,
                "accountName": r.account_name,
                "role": r.role.as_str(),
                "status": r.status.as_str(),
                "createdAt": to_iso(&r.created_at),
            });
            if item.expose_account_id {
                v["accountId"] = json!(r.account_id);
            }
            v
        })
        .collect();
    Ok(Json(json!({ "members": members, "total": total })))
}

async fn review_preconditions(
    state: &AppState,
    headers: &HeaderMap,
    id: &str,
    member_id: &str,
) -> ApiResult<(Repos, ReviewMemberCommand)> {
    let account_id = require_account(headers, &state.jwt_secret)?;
    let repos = Repos::new(state);
    require_community_role(&repos, id, &account_id, MANAGER_ROLES).await?;
    let command = ReviewMemberCommand {
        community_id: id.to_string(),
        target_member_id: member_id.to_string(),
        occurred_at: chrono::Utc::now(),
    };
    Ok((repos, command))
}

/// PATCH /communities/:id/members/:memberId/approve - メンバー承認
pub async fn approve(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, member_id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let id = uuid_param("id", &id)?;
    let member_id = uuid_param("memberId", &member_id)?;
    let (repos, command) = review_preconditions(&state, &headers, &id, &member_id).await?;
    let member = approve_member_command(&repos.ctx(&state), command).await?;
    Ok(Json(member.to_response()))
}

/// PATCH /communities/:id/members/:memberId/reject - メンバー拒否
pub async fn reject(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, member_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let id = uuid_param("id", &id)?;
    let member_id = uuid_param("memberId", &member_id)?;
    let (repos, command) = review_preconditions(&state, &headers, &id, &member_id).await?;
    reject_member_command(&repos.ctx(&state), command).await?;
    Ok(StatusCode::NO_CONTENT)
}
