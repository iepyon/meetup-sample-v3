use std::collections::HashMap;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use serde_json::{Value, json};

use super::models::{CreateEventInput, EventFormat};
use super::repository::EventRepository;
use super::usecases::{Ctx, create_event_command, list_events_query};
use crate::app::AppState;
use crate::community::access::{MANAGER_ROLES, Repos, require_community_role};
use crate::shared::auth::{optional_account, require_account};
use crate::shared::error::ApiResult;
use crate::shared::time::to_iso;
use crate::shared::validation::{
    json_object, optional_nullable_string, page_params, required_datetime, required_enum,
    required_int, required_string, uuid_param,
};

/// GET /communities/:id/events - イベント一覧（DRAFT は作成者のみ）
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let id = uuid_param("id", &id)?;
    let (limit, offset) = page_params(&query)?;
    let account_id = optional_account(&headers, &state.jwt_secret);

    let repos = Repos::new(&state);
    let communities = repos.communities();
    let events = EventRepository::new(state.pool.clone());
    let ctx = Ctx {
        communities,
        events: &events,
        bus: &state.event_events,
        members: repos.members(),
    };
    let (rows, total) = list_events_query(&ctx, &id, account_id.as_deref(), limit, offset).await?;

    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "communityId": r.community_id,
                "title": r.title,
                "status": r.status.as_str(),
                "startsAt": to_iso(&r.starts_at),
                "endsAt": to_iso(&r.ends_at),
                "format": r.format.as_str(),
                "capacity": r.capacity,
            })
        })
        .collect();
    Ok(Json(json!({ "events": items, "total": total })))
}

/// POST /communities/:id/events - イベント作成（OWNER / ADMIN のみ）
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let id = uuid_param("id", &id)?;
    let obj = json_object(&body)?;
    let title = required_string(&obj, "title", 1, 100)?;
    let description = optional_nullable_string(&obj, "description", 1000)?;
    let starts_at = required_datetime(&obj, "startsAt")?;
    let ends_at = required_datetime(&obj, "endsAt")?;
    let format = required_enum(&obj, "format", EventFormat::ALL)?;
    let capacity = required_int(&obj, "capacity", 1, 1000)?;

    let account_id = require_account(&headers, &state.jwt_secret)?;
    let repos = Repos::new(&state);
    require_community_role(&repos, &id, &account_id, MANAGER_ROLES).await?;

    let now = chrono::Utc::now();
    let events = EventRepository::new(state.pool.clone());
    let ctx = Ctx {
        communities: repos.communities(),
        events: &events,
        bus: &state.event_events,
        members: repos.members(),
    };
    let event = create_event_command(
        &ctx,
        CreateEventInput {
            id: uuid::Uuid::new_v4().to_string(),
            community_id: id,
            created_by: account_id,
            title,
            description,
            starts_at,
            ends_at,
            format: EventFormat::parse(format).unwrap_or(EventFormat::Online),
            capacity,
            now,
            created_at: now,
            updated_at: now,
        },
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "event": event.to_response() })),
    ))
}
