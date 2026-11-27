use chrono::{DateTime, Utc};

use super::errors::*;
use super::models::{
    Community, CommunityCategory, CommunityMember, CommunityVisibility,
    CreateCommunityInput as CreateInput, MemberStatus, approve_member, create_community,
    join_community, leave_community, reject_member,
};
use super::repository::{
    CommunityMemberRepository, CommunityRepository, FindAllOptions, MemberReadRow,
};
use crate::shared::error::is_unique_violation;
use crate::shared::event_bus::EventBus;

/// 1 ユーザーあたりの最大コミュニティ作成数
const MAX_COMMUNITIES_PER_USER: i64 = 10;

/// UseCase が使う依存性（Composition Root から注入される）。
/// UseCase は調整のみを担い、ドメインロジックは `models` に委譲する。
pub struct Ctx<'a> {
    pub communities: &'a CommunityRepository,
    pub members: &'a CommunityMemberRepository,
    pub bus: &'a EventBus<CommunityDomainEvent>,
}

// ============================================================
// コミュニティ作成
// ============================================================

pub struct CreateCommunityCommand {
    pub id: String,
    pub owner_member_id: String,
    pub account_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: CommunityCategory,
    pub visibility: CommunityVisibility,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn create_community_command(
    ctx: &Ctx<'_>,
    command: CreateCommunityCommand,
) -> Result<(Community, CommunityMember), CreateCommunityError> {
    let duplicate = || CreateCommunityError::DuplicateCommunityName {
        name: command.name.clone(),
    };

    if ctx.communities.find_by_name(&command.name).await?.is_some() {
        return Err(duplicate());
    }
    let count = ctx
        .communities
        .count_by_owner_account_id(&command.account_id)
        .await?;
    if count >= MAX_COMMUNITIES_PER_USER {
        return Err(CreateCommunityError::TooManyCommunities);
    }

    let (community, owner) = create_community(CreateInput {
        id: command.id.clone(),
        name: command.name.clone(),
        description: command.description.clone(),
        category: command.category,
        visibility: command.visibility,
        created_at: command.created_at,
        updated_at: command.updated_at,
        owner_account_id: command.account_id.clone(),
        owner_member_id: command.owner_member_id.clone(),
    });

    ctx.communities
        .save_with_owner(&community, &owner)
        .await
        .map_err(|e| {
            if is_unique_violation(&e) {
                duplicate()
            } else {
                CreateCommunityError::from(e)
            }
        })?;

    ctx.bus.publish(&CommunityDomainEvent::CommunityCreated {
        community_id: community.id.clone(),
        account_id: command.account_id,
        name: community.name.clone(),
        occurred_at: command.created_at,
    });
    Ok((community, owner))
}

// ============================================================
// コミュニティ取得・一覧
// ============================================================

/// PUBLIC は誰でも閲覧可能。PRIVATE はメンバーのみ（存在を漏らさないため 404 扱い）。
pub async fn get_community_query(
    ctx: &Ctx<'_>,
    community_id: &str,
    requesting_account_id: Option<&str>,
) -> Result<Community, GetCommunityError> {
    let community = ctx
        .communities
        .find_by_id(community_id)
        .await?
        .ok_or(GetCommunityError::CommunityNotFound)?;
    if community.visibility == CommunityVisibility::Public {
        return Ok(community);
    }
    let Some(account_id) = requesting_account_id else {
        return Err(GetCommunityError::CommunityNotFound);
    };
    // 承認待ち（PENDING）の申請者には見せない（メンバー一覧と同じ ACTIVE 条件）
    ctx.members
        .find_by_ids(&community.id, account_id)
        .await?
        .filter(|m| m.status == MemberStatus::Active)
        .ok_or(GetCommunityError::CommunityNotFound)?;
    Ok(community)
}

pub struct ListCommunitiesQueryInput<'a> {
    pub category: Option<&'a str>,
    pub member_account_id: Option<&'a str>,
    pub limit: i64,
    pub offset: i64,
}

/// `member_account_id` 指定時はそのユーザーが所属するコミュニティ、未指定時は PUBLIC のみ。
pub async fn list_communities_query(
    ctx: &Ctx<'_>,
    input: ListCommunitiesQueryInput<'_>,
) -> Result<(Vec<Community>, i64), ListCommunitiesError> {
    let visibility = if input.member_account_id.is_some() {
        None
    } else {
        Some(CommunityVisibility::Public)
    };
    Ok(ctx
        .communities
        .find_all(FindAllOptions {
            category: input.category,
            member_account_id: input.member_account_id,
            visibility,
            limit: input.limit,
            offset: input.offset,
        })
        .await?)
}

// ============================================================
// メンバーシップ
// ============================================================

pub struct JoinCommunityCommand {
    pub community_id: String,
    pub account_id: String,
    pub member_id: String,
    pub occurred_at: DateTime<Utc>,
}

/// PUBLIC → ACTIVE（MemberJoined）、PRIVATE → PENDING（MemberApplicationSubmitted）
pub async fn join_community_command(
    ctx: &Ctx<'_>,
    command: JoinCommunityCommand,
) -> Result<CommunityMember, JoinCommunityError> {
    let community = ctx
        .communities
        .find_by_id(&command.community_id)
        .await?
        .ok_or(JoinCommunityError::CommunityNotFound)?;
    if ctx
        .members
        .find_by_ids(&command.community_id, &command.account_id)
        .await?
        .is_some()
    {
        return Err(JoinCommunityError::AlreadyMember);
    }

    let member = join_community(
        &community,
        command.account_id,
        command.member_id,
        command.occurred_at,
    );
    ctx.members.save(&member).await.map_err(|e| {
        if is_unique_violation(&e) {
            JoinCommunityError::AlreadyMember
        } else {
            JoinCommunityError::from(e)
        }
    })?;

    let (community_id, account_id, member_id, occurred_at) = (
        member.community_id.clone(),
        member.account_id.clone(),
        member.id.clone(),
        command.occurred_at,
    );
    ctx.bus.publish(&match community.visibility {
        CommunityVisibility::Public => CommunityDomainEvent::MemberJoined {
            community_id,
            account_id,
            member_id,
            occurred_at,
        },
        CommunityVisibility::Private => CommunityDomainEvent::MemberApplicationSubmitted {
            community_id,
            account_id,
            member_id,
            occurred_at,
        },
    });
    Ok(member)
}

pub struct LeaveCommunityCommand {
    pub community_id: String,
    pub account_id: String,
    /// メンバー ID 指定で脱退する場合（所有者確認を行う）
    pub member_id: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

pub async fn leave_community_command(
    ctx: &Ctx<'_>,
    command: LeaveCommunityCommand,
) -> Result<(), LeaveCommunityError> {
    ctx.communities
        .find_by_id(&command.community_id)
        .await?
        .ok_or(LeaveCommunityError::CommunityNotFound)?;

    if let Some(member_id) = &command.member_id {
        let by_id = ctx.members.find_by_id(member_id).await?;
        if by_id.is_none_or(|m| {
            m.account_id != command.account_id || m.community_id != command.community_id
        }) {
            return Err(LeaveCommunityError::MemberNotFound);
        }
    }

    let member = ctx
        .members
        .find_by_ids(&command.community_id, &command.account_id)
        .await?
        .ok_or(LeaveCommunityError::MemberNotFound)?;
    leave_community(&member)?;

    ctx.members.delete(&member.id).await?;
    ctx.bus.publish(&CommunityDomainEvent::MemberLeft {
        community_id: member.community_id,
        account_id: member.account_id,
        member_id: member.id,
        occurred_at: command.occurred_at,
    });
    Ok(())
}

pub struct ReviewMemberCommand {
    pub community_id: String,
    pub target_member_id: String,
    pub occurred_at: DateTime<Utc>,
}

/// 権限チェックはコントローラ層（ロール要求）で実施済み。
/// 対象が当該コミュニティに所属することを照合し、越権操作を防ぐ。
pub async fn approve_member_command(
    ctx: &Ctx<'_>,
    command: ReviewMemberCommand,
) -> Result<CommunityMember, ApproveMemberError> {
    ctx.communities
        .find_by_id(&command.community_id)
        .await?
        .ok_or(ApproveMemberError::CommunityNotFound)?;
    let target = ctx
        .members
        .find_by_id(&command.target_member_id)
        .await?
        .filter(|m| m.community_id == command.community_id)
        .ok_or(ApproveMemberError::MemberNotFound)?;

    let approved = approve_member(&target)?;
    ctx.members.save(&approved).await?;
    ctx.bus.publish(&CommunityDomainEvent::MemberApproved {
        community_id: approved.community_id.clone(),
        account_id: approved.account_id.clone(),
        member_id: approved.id.clone(),
        occurred_at: command.occurred_at,
    });
    Ok(approved)
}

pub async fn reject_member_command(
    ctx: &Ctx<'_>,
    command: ReviewMemberCommand,
) -> Result<(), RejectMemberError> {
    ctx.communities
        .find_by_id(&command.community_id)
        .await?
        .ok_or(RejectMemberError::CommunityNotFound)?;
    let target = ctx
        .members
        .find_by_id(&command.target_member_id)
        .await?
        .filter(|m| m.community_id == command.community_id)
        .ok_or(RejectMemberError::MemberNotFound)?;
    reject_member(&target)?;

    ctx.members.delete(&target.id).await?;
    ctx.bus.publish(&CommunityDomainEvent::MemberRejected {
        community_id: target.community_id,
        account_id: target.account_id,
        member_id: target.id,
        occurred_at: command.occurred_at,
    });
    Ok(())
}

// ============================================================
// メンバー一覧（Read モデル）
// ============================================================

pub struct MemberListItem {
    pub row: MemberReadRow,
    /// requester が ACTIVE メンバーの場合のみ accountId を公開する（列挙による相関分析を防ぐ）
    pub expose_account_id: bool,
}

/// PUBLIC は誰でも、PRIVATE は ACTIVE メンバーのみ閲覧可能（存在を漏らさず 404）。
/// 承認待ち（PENDING）は申請を審査できる閲覧者（ACTIVE な OWNER / ADMIN）にだけ含める。
pub async fn list_members_read_query(
    ctx: &Ctx<'_>,
    community_id: &str,
    requesting_account_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<MemberListItem>, i64), ListMembersError> {
    let community = ctx
        .communities
        .find_by_id(community_id)
        .await?
        .ok_or(ListMembersError::CommunityNotFound)?;

    let requester = match requesting_account_id {
        Some(account_id) => ctx.members.find_by_ids(community_id, account_id).await?,
        None => None,
    };
    let is_active_member = requester
        .as_ref()
        .is_some_and(|m| m.status == MemberStatus::Active);
    if community.visibility == CommunityVisibility::Private && !is_active_member {
        return Err(ListMembersError::CommunityNotFound);
    }

    // 承認待ちの申請者は審査できるメンバーにだけ見せる
    let include_pending = requester
        .as_ref()
        .is_some_and(|m| m.can_review_applications());
    let (rows, total) = ctx
        .members
        .list_with_names(community_id, include_pending, limit, offset)
        .await?;
    let items = rows
        .into_iter()
        .map(|row| MemberListItem {
            row,
            expose_account_id: is_active_member,
        })
        .collect();
    Ok((items, total))
}
