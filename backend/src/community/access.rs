//! コミュニティ単位のアクセス制御と UseCase 依存性の組み立て。
//! controller から独立させ、他コンテキスト（event など）が community の controller に依存しないようにする。

use super::models::MemberRole;
use super::repository::{CommunityMemberRepository, CommunityRepository};
use super::usecases::Ctx;
use crate::app::AppState;
use crate::shared::error::{ApiError, ApiResult, log_internal};

/// ハンドラごとに UseCase の依存性（リポジトリ）を組み立てる。
pub struct Repos {
    communities: CommunityRepository,
    members: CommunityMemberRepository,
}

impl Repos {
    pub fn new(state: &AppState) -> Self {
        Self {
            communities: CommunityRepository::new(state.pool.clone()),
            members: CommunityMemberRepository::new(state.pool.clone()),
        }
    }

    pub fn communities(&self) -> &CommunityRepository {
        &self.communities
    }

    pub fn members(&self) -> &CommunityMemberRepository {
        &self.members
    }

    pub fn ctx<'a>(&'a self, state: &'a AppState) -> Ctx<'a> {
        Ctx {
            communities: &self.communities,
            members: &self.members,
            bus: &state.community_events,
        }
    }
}

/// ロール要求: OWNER / ADMIN のみ通過（それ以外は 403）。コミュニティの存在有無も漏らさない。
pub async fn require_community_role(
    repos: &Repos,
    community_id: &str,
    account_id: &str,
    allowed: &[MemberRole],
) -> ApiResult<()> {
    let member = repos
        .members
        .find_by_ids(community_id, account_id)
        .await
        .map_err(|e| {
            log_internal("require_community_role", &e);
            ApiError::internal()
        })?;
    match member {
        Some(m) if allowed.contains(&m.role) => Ok(()),
        _ => Err(ApiError::forbidden()),
    }
}

pub const MANAGER_ROLES: &[MemberRole] = &[MemberRole::Owner, MemberRole::Admin];
