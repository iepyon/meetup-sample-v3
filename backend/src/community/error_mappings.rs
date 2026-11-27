use axum::http::StatusCode;

use super::errors::*;
use crate::shared::error::ApiError;

// ============================================================
// エラー → HTTP レスポンス マッピング
// ============================================================

fn community_not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "COMMUNITY_NOT_FOUND",
        "コミュニティが見つかりません",
    )
}

fn member_not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "MEMBER_NOT_FOUND",
        "メンバーが見つかりません",
    )
}

impl From<CreateCommunityError> for ApiError {
    fn from(error: CreateCommunityError) -> Self {
        match error {
            CreateCommunityError::DuplicateCommunityName { name } => ApiError::new(
                StatusCode::CONFLICT,
                "DUPLICATE_COMMUNITY_NAME",
                format!("このコミュニティ名は既に使用されています: {name}"),
            ),
            CreateCommunityError::TooManyCommunities => ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "TOO_MANY_COMMUNITIES",
                "コミュニティの作成数が上限に達しています",
            ),
            CreateCommunityError::Internal => ApiError::internal(),
        }
    }
}

impl From<GetCommunityError> for ApiError {
    fn from(error: GetCommunityError) -> Self {
        match error {
            GetCommunityError::CommunityNotFound => community_not_found(),
            GetCommunityError::Internal => ApiError::internal(),
        }
    }
}

impl From<JoinCommunityError> for ApiError {
    fn from(error: JoinCommunityError) -> Self {
        match error {
            JoinCommunityError::CommunityNotFound => community_not_found(),
            JoinCommunityError::AlreadyMember => ApiError::new(
                StatusCode::CONFLICT,
                "ALREADY_MEMBER",
                "既にこのコミュニティのメンバーです",
            ),
            JoinCommunityError::Internal => ApiError::internal(),
        }
    }
}

impl From<LeaveCommunityError> for ApiError {
    fn from(error: LeaveCommunityError) -> Self {
        match error {
            LeaveCommunityError::CommunityNotFound => community_not_found(),
            LeaveCommunityError::MemberNotFound => member_not_found(),
            LeaveCommunityError::OwnerCannotLeave => ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "OWNER_CANNOT_LEAVE",
                "オーナーはコミュニティを脱退できません",
            ),
            LeaveCommunityError::Internal => ApiError::internal(),
        }
    }
}

impl From<ApproveMemberError> for ApiError {
    fn from(error: ApproveMemberError) -> Self {
        match error {
            ApproveMemberError::CommunityNotFound => community_not_found(),
            ApproveMemberError::MemberNotFound => member_not_found(),
            ApproveMemberError::MemberAlreadyActive => ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "MEMBER_ALREADY_ACTIVE",
                "メンバーは既にアクティブです",
            ),
            ApproveMemberError::Internal => ApiError::internal(),
        }
    }
}

impl From<RejectMemberError> for ApiError {
    fn from(error: RejectMemberError) -> Self {
        match error {
            RejectMemberError::CommunityNotFound => community_not_found(),
            RejectMemberError::MemberNotFound => member_not_found(),
            RejectMemberError::CannotRejectOwner => ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "CANNOT_REJECT_OWNER",
                "オーナーは拒否できません",
            ),
            RejectMemberError::Internal => ApiError::internal(),
        }
    }
}

impl From<ListMembersError> for ApiError {
    fn from(error: ListMembersError) -> Self {
        match error {
            ListMembersError::CommunityNotFound => community_not_found(),
            ListMembersError::Internal => ApiError::internal(),
        }
    }
}

impl From<ListCommunitiesError> for ApiError {
    fn from(_: ListCommunitiesError) -> Self {
        ApiError::internal()
    }
}
