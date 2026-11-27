use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use super::errors::{ApproveMemberError, LeaveCommunityError, RejectMemberError};
use crate::shared::time::to_iso;

// ============================================================
// 列挙型（スキーマ由来。文字列リテラルの直書きを避ける）
// ============================================================

crate::string_enum!(CommunityCategory { Tech => "TECH", Business => "BUSINESS", Hobby => "HOBBY" });
crate::string_enum!(CommunityVisibility { Public => "PUBLIC", Private => "PRIVATE" });
crate::string_enum!(MemberRole { Owner => "OWNER", Admin => "ADMIN", Member => "MEMBER" });
crate::string_enum!(MemberStatus { Pending => "PENDING", Active => "ACTIVE" });

/// メンバー管理（承認・拒否・イベント作成）ができるロール
pub const MANAGER_ROLES: &[MemberRole] = &[MemberRole::Owner, MemberRole::Admin];

// ============================================================
// コミュニティ
// ============================================================

#[derive(Debug, Clone)]
pub struct Community {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: CommunityCategory,
    pub visibility: CommunityVisibility,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Community {
    pub fn to_response(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "category": self.category.as_str(),
            "visibility": self.visibility.as_str(),
            "createdAt": to_iso(&self.created_at),
            "updatedAt": to_iso(&self.updated_at),
        })
    }
}

#[derive(Debug, Clone)]
pub struct CommunityMember {
    pub id: String,
    pub community_id: String,
    pub account_id: String,
    pub role: MemberRole,
    pub status: MemberStatus,
    pub created_at: DateTime<Utc>,
}

impl CommunityMember {
    pub fn to_response(&self) -> Value {
        json!({
            "id": self.id,
            "communityId": self.community_id,
            "accountId": self.account_id,
            "role": self.role.as_str(),
            "status": self.status.as_str(),
            "createdAt": to_iso(&self.created_at),
        })
    }

    /// 指定ロールのいずれかを持つ ACTIVE メンバーか。承認待ち（PENDING）はロールにかかわらず権限を持たない
    pub fn has_active_role(&self, allowed: &[MemberRole]) -> bool {
        self.status == MemberStatus::Active && allowed.contains(&self.role)
    }

    /// 承認待ち（PENDING）の参加申請を閲覧・審査できるか（ACTIVE な OWNER / ADMIN）
    pub fn can_review_applications(&self) -> bool {
        self.has_active_role(MANAGER_ROLES)
    }
}

// ============================================================
// コミュニティ作成（ファクトリ）
// ============================================================

pub struct CreateCommunityInput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: CommunityCategory,
    pub visibility: CommunityVisibility,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub owner_account_id: String,
    pub owner_member_id: String,
}

/// コミュニティとオーナーメンバー（ACTIVE）を同時に生成する。
/// 名前重複・上限数のような集約外の不変条件は UseCase で事前チェックする。
pub fn create_community(input: CreateCommunityInput) -> (Community, CommunityMember) {
    let community = Community {
        id: input.id.clone(),
        name: input.name,
        description: input.description,
        category: input.category,
        visibility: input.visibility,
        created_at: input.created_at,
        updated_at: input.updated_at,
    };
    let owner = CommunityMember {
        id: input.owner_member_id,
        community_id: input.id,
        account_id: input.owner_account_id,
        role: MemberRole::Owner,
        status: MemberStatus::Active,
        created_at: input.created_at,
    };
    (community, owner)
}

// ============================================================
// メンバーシップ遷移
// ============================================================

/// 参加: PUBLIC → ACTIVE、PRIVATE → PENDING
pub fn join_community(
    community: &Community,
    account_id: String,
    member_id: String,
    now: DateTime<Utc>,
) -> CommunityMember {
    let status = match community.visibility {
        CommunityVisibility::Public => MemberStatus::Active,
        CommunityVisibility::Private => MemberStatus::Pending,
    };
    CommunityMember {
        id: member_id,
        community_id: community.id.clone(),
        account_id,
        role: MemberRole::Member,
        status,
        created_at: now,
    }
}

/// 脱退: オーナーは不可
pub fn leave_community(member: &CommunityMember) -> Result<(), LeaveCommunityError> {
    if member.role == MemberRole::Owner {
        return Err(LeaveCommunityError::OwnerCannotLeave);
    }
    Ok(())
}

/// 承認: PENDING → ACTIVE。既に ACTIVE ならエラー
pub fn approve_member(member: &CommunityMember) -> Result<CommunityMember, ApproveMemberError> {
    if member.status == MemberStatus::Active {
        return Err(ApproveMemberError::MemberAlreadyActive);
    }
    Ok(CommunityMember {
        status: MemberStatus::Active,
        ..member.clone()
    })
}

/// 拒否: オーナーは拒否（削除）できない（コミュニティが無主化するため）
/// 参加申請を拒否する。対象は承認待ち（PENDING）のみ（ACTIVE メンバーの除名は別の操作として扱う）。
pub fn reject_member(member: &CommunityMember) -> Result<(), RejectMemberError> {
    if member.role == MemberRole::Owner {
        return Err(RejectMemberError::CannotRejectOwner);
    }
    if member.status != MemberStatus::Pending {
        return Err(RejectMemberError::MemberNotPending);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
