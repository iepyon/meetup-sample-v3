use chrono::{DateTime, Utc};

use crate::shared::error::log_internal;

// ============================================================
// ドメインエラー（判別共用体）
// ============================================================

#[derive(Debug)]
pub enum CreateCommunityError {
    DuplicateCommunityName,
    TooManyCommunities,
    Internal,
}

#[derive(Debug)]
pub enum GetCommunityError {
    CommunityNotFound,
    Internal,
}

#[derive(Debug)]
pub enum JoinCommunityError {
    CommunityNotFound,
    AlreadyMember,
    Internal,
}

#[derive(Debug)]
pub enum LeaveCommunityError {
    CommunityNotFound,
    MemberNotFound,
    OwnerCannotLeave,
    Internal,
}

#[derive(Debug)]
pub enum ApproveMemberError {
    CommunityNotFound,
    MemberNotFound,
    MemberAlreadyActive,
    Internal,
}

#[derive(Debug)]
pub enum RejectMemberError {
    CommunityNotFound,
    MemberNotFound,
    CannotRejectOwner,
    MemberNotPending,
    Internal,
}

#[derive(Debug)]
pub enum ListMembersError {
    CommunityNotFound,
    Internal,
}

#[derive(Debug)]
pub enum ListCommunitiesError {
    Internal,
}

/// sqlx のエラーは `?` で `Internal` に変換する（種別のみログ出力）。
macro_rules! internal_from_sqlx {
    ($($error:ident),+) => {$(
        impl From<sqlx::Error> for $error {
            fn from(e: sqlx::Error) -> Self {
                log_internal(stringify!($error), &e);
                Self::Internal
            }
        }
    )+};
}

internal_from_sqlx!(
    CreateCommunityError,
    GetCommunityError,
    JoinCommunityError,
    LeaveCommunityError,
    ApproveMemberError,
    RejectMemberError,
    ListMembersError,
    ListCommunitiesError
);

// ============================================================
// ドメインイベント
// ============================================================

#[derive(Debug, Clone)]
pub enum CommunityDomainEvent {
    CommunityCreated {
        community_id: String,
        account_id: String,
        name: String,
        occurred_at: DateTime<Utc>,
    },
    MemberJoined {
        community_id: String,
        account_id: String,
        member_id: String,
        occurred_at: DateTime<Utc>,
    },
    MemberApplicationSubmitted {
        community_id: String,
        account_id: String,
        member_id: String,
        occurred_at: DateTime<Utc>,
    },
    MemberLeft {
        community_id: String,
        account_id: String,
        member_id: String,
        occurred_at: DateTime<Utc>,
    },
    MemberApproved {
        community_id: String,
        account_id: String,
        member_id: String,
        occurred_at: DateTime<Utc>,
    },
    MemberRejected {
        community_id: String,
        account_id: String,
        member_id: String,
        occurred_at: DateTime<Utc>,
    },
}
