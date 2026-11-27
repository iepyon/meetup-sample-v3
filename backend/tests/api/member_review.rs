//! 参加申請の審査（承認・拒否）
use crate::common::*;
use meetup_test_macros::describe;

describe!("参加申請の審査", {
    test!(
        "非公開コミュニティは承認制で、承認と拒否ができる",
        async {
            let app = app().await;
            let owner = register(&app, "ap-owner").await;
            let u1 = register(&app, "ap-u1").await;
            let u2 = register(&app, "ap-u2").await;
            let id = create_community(&app, &owner, "approval", "PRIVATE").await;
            let members = format!("/communities/{id}/members");

            let (_, j1) = call(&app, "POST", &members, Some(&u1.token), None).await;
            let (_, j2) = call(&app, "POST", &members, Some(&u2.token), None).await;
            assert_eq!(j1["status"], "PENDING");
            let m1 = j1["id"].as_str().unwrap();
            let m2 = j2["id"].as_str().unwrap();

            // 一般メンバーは承認できない（403）
            let (status, body) = call(
                &app,
                "PATCH",
                &format!("{members}/{m1}/approve"),
                Some(&u2.token),
                None,
            )
            .await;
            assert_eq!(status, StatusCode::FORBIDDEN);
            assert_eq!(body["code"], "FORBIDDEN");

            let (status, body) = call(
                &app,
                "PATCH",
                &format!("{members}/{m1}/approve"),
                Some(&owner.token),
                None,
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body["status"], "ACTIVE");

            let (status, body) = call(
                &app,
                "PATCH",
                &format!("{members}/{m1}/approve"),
                Some(&owner.token),
                None,
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], "MEMBER_ALREADY_ACTIVE");

            let (status, _) = call(
                &app,
                "PATCH",
                &format!("{members}/{m2}/reject"),
                Some(&owner.token),
                None,
            )
            .await;
            assert_eq!(status, StatusCode::NO_CONTENT);
        }
    );

    test!(
        "別コミュニティのメンバーは承認できない",
        async {
            let app = app().await;
            let owner_a = register(&app, "x-owner-a").await;
            let owner_b = register(&app, "x-owner-b").await;
            let user = register(&app, "x-user").await;
            let a = create_community(&app, &owner_a, "comm-a", "PRIVATE").await;
            let b = create_community(&app, &owner_b, "comm-b", "PRIVATE").await;
            let (_, joined) = call(
                &app,
                "POST",
                &format!("/communities/{b}/members"),
                Some(&user.token),
                None,
            )
            .await;
            let member_id = joined["id"].as_str().unwrap();

            let (status, body) = call(
                &app,
                "PATCH",
                &format!("/communities/{a}/members/{member_id}/approve"),
                Some(&owner_a.token),
                None,
            )
            .await;
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert_eq!(body["code"], "MEMBER_NOT_FOUND");
        }
    );

    test!(
        "拒否したメンバーは一覧から消え、再参加できる",
        async {
            let app = app().await;
            let owner = register(&app, "rj-owner").await;
            let user = register(&app, "rj-user").await;
            let id = create_community(&app, &owner, "reject-flow", "PRIVATE").await;
            let members = format!("/communities/{id}/members");
            let (_, joined) = call(&app, "POST", &members, Some(&user.token), None).await;
            let member_id = joined["id"].as_str().unwrap();
            let (status, _) = call(
                &app,
                "PATCH",
                &format!("{members}/{member_id}/reject"),
                Some(&owner.token),
                None,
            )
            .await;
            assert_eq!(status, StatusCode::NO_CONTENT);
            let (status, _) = call(&app, "POST", &members, Some(&user.token), None).await;
            assert_eq!(status, StatusCode::CREATED, "拒否後は再申請できる");
        }
    );

    test!("オーナーのメンバーは拒否できない", async {
        let app = app().await;
        let owner = register(&app, "ro-owner").await;
        let id = create_community(&app, &owner, "reject-owner", "PUBLIC").await;
        let (_, list) = call(
            &app,
            "GET",
            &format!("/communities/{id}/members"),
            Some(&owner.token),
            None,
        )
        .await;
        let owner_member = list["members"][0]["id"].as_str().unwrap();
        let (status, body) = call(
            &app,
            "PATCH",
            &format!("/communities/{id}/members/{owner_member}/reject"),
            Some(&owner.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["code"], "CANNOT_REJECT_OWNER");
    });

    test!(
        "ACTIVE なメンバーは参加申請の拒否で外せない",
        async {
            let app = app().await;
            let owner = register(&app, "ra-owner").await;
            let member = register(&app, "ra-member").await;
            let id = create_community(&app, &owner, "reject-active", "PUBLIC").await;
            let (_, joined) = call(
                &app,
                "POST",
                &format!("/communities/{id}/members"),
                Some(&member.token),
                None,
            )
            .await;
            assert_eq!(joined["status"], "ACTIVE");
            let member_id = joined["id"].as_str().unwrap();

            let (status, body) = call(
                &app,
                "PATCH",
                &format!("/communities/{id}/members/{member_id}/reject"),
                Some(&owner.token),
                None,
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], "MEMBER_NOT_PENDING");
        }
    );
});
