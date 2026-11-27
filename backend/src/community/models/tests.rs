//! `community/models.rs` の単体テスト

use super::*;
use meetup_test_macros::describe;

fn community(visibility: CommunityVisibility) -> Community {
    Community {
        id: "c1".into(),
        name: "n".into(),
        description: None,
        category: CommunityCategory::Tech,
        visibility,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn member(role: MemberRole, status: MemberStatus) -> CommunityMember {
    CommunityMember {
        id: "m1".into(),
        community_id: "c1".into(),
        account_id: "a1".into(),
        role,
        status,
        created_at: Utc::now(),
    }
}

describe!(
    "コミュニティとメンバーのドメインモデル",
    {
        test!("公開コミュニティへの参加は ACTIVE になる", {
            let m = join_community(
                &community(CommunityVisibility::Public),
                "a".into(),
                "m".into(),
                Utc::now(),
            );
            assert_eq!(m.status, MemberStatus::Active);
            assert_eq!(m.role, MemberRole::Member);
        });

        test!(
            "非公開コミュニティへの参加は PENDING になる",
            {
                let m = join_community(
                    &community(CommunityVisibility::Private),
                    "a".into(),
                    "m".into(),
                    Utc::now(),
                );
                assert_eq!(m.status, MemberStatus::Pending);
            }
        );

        test!("オーナーは脱退できない", {
            assert!(matches!(
                leave_community(&member(MemberRole::Owner, MemberStatus::Active)),
                Err(LeaveCommunityError::OwnerCannotLeave)
            ));
            assert!(leave_community(&member(MemberRole::Member, MemberStatus::Active)).is_ok());
        });

        test!(
            "PENDING のメンバーを承認すると ACTIVE になる",
            {
                let approved =
                    approve_member(&member(MemberRole::Member, MemberStatus::Pending)).unwrap();
                assert_eq!(approved.status, MemberStatus::Active);
            }
        );

        test!("既に ACTIVE のメンバーは承認できない", {
            assert!(matches!(
                approve_member(&member(MemberRole::Member, MemberStatus::Active)),
                Err(ApproveMemberError::MemberAlreadyActive)
            ));
        });

        test!("オーナーは拒否できない", {
            assert!(matches!(
                reject_member(&member(MemberRole::Owner, MemberStatus::Active)),
                Err(RejectMemberError::CannotRejectOwner)
            ));
            assert!(reject_member(&member(MemberRole::Member, MemberStatus::Pending)).is_ok());
        });

        test!("ACTIVE なメンバーは拒否できない", {
            for role in [MemberRole::Member, MemberRole::Admin] {
                assert!(matches!(
                    reject_member(&member(role, MemberStatus::Active)),
                    Err(RejectMemberError::MemberNotPending)
                ));
            }
        });

        test!(
            "PENDING のメンバーは指定ロールを持っていても権限を持たない",
            {
                assert!(
                    member(MemberRole::Admin, MemberStatus::Active).has_active_role(MANAGER_ROLES)
                );
                assert!(
                    !member(MemberRole::Admin, MemberStatus::Pending)
                        .has_active_role(MANAGER_ROLES)
                );
                assert!(
                    !member(MemberRole::Member, MemberStatus::Active)
                        .has_active_role(MANAGER_ROLES)
                );
            }
        );

        test!(
            "ACTIVE なオーナーと管理者は承認待ちの申請を審査できる",
            {
                assert!(member(MemberRole::Owner, MemberStatus::Active).can_review_applications());
                assert!(member(MemberRole::Admin, MemberStatus::Active).can_review_applications());
            }
        );

        test!(
            "一般メンバーと PENDING のメンバーは承認待ちの申請を審査できない",
            {
                assert!(
                    !member(MemberRole::Member, MemberStatus::Active).can_review_applications()
                );
                assert!(
                    !member(MemberRole::Member, MemberStatus::Pending).can_review_applications()
                );
                assert!(
                    !member(MemberRole::Admin, MemberStatus::Pending).can_review_applications()
                );
            }
        );

        test!(
            "コミュニティ作成でオーナーが ACTIVE になる",
            {
                let now = Utc::now();
                let (c, owner) = create_community(CreateCommunityInput {
                    id: "c".into(),
                    name: "n".into(),
                    description: None,
                    category: CommunityCategory::Hobby,
                    visibility: CommunityVisibility::Public,
                    created_at: now,
                    updated_at: now,
                    owner_account_id: "a".into(),
                    owner_member_id: "m".into(),
                });
                assert_eq!(owner.community_id, c.id);
                assert_eq!(owner.role, MemberRole::Owner);
                assert_eq!(owner.status, MemberStatus::Active);
            }
        );
    }
);
