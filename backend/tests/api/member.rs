//! メンバー（参加・脱退・ロールと権限）
use crate::common::*;
use meetup_test_macros::describe;

describe!("メンバー", {
    describe!("参加と脱退", {
        test!(
            "公開コミュニティへの参加は ACTIVE、重複参加は 409、脱退できる",
            async {
                let app = app().await;
                let owner = register(&app, "j-owner").await;
                let user = register(&app, "j-user").await;
                let id = create_community(&app, &owner, "open", "PUBLIC").await;
                let members = format!("/communities/{id}/members");

                let (status, body) = call(&app, "POST", &members, Some(&user.token), None).await;
                assert_eq!(status, StatusCode::CREATED);
                assert_eq!(body["status"], "ACTIVE");
                assert_eq!(body["role"], "MEMBER");

                let (status, body) = call(&app, "POST", &members, Some(&user.token), None).await;
                assert_eq!(status, StatusCode::CONFLICT);
                assert_eq!(body["code"], "ALREADY_MEMBER");

                let (status, _) = call(
                    &app,
                    "DELETE",
                    &format!("{members}/me"),
                    Some(&user.token),
                    None,
                )
                .await;
                assert_eq!(status, StatusCode::OK);
                let (status, body) = call(
                    &app,
                    "DELETE",
                    &format!("{members}/me"),
                    Some(&user.token),
                    None,
                )
                .await;
                assert_eq!(status, StatusCode::NOT_FOUND);
                assert_eq!(body["code"], "MEMBER_NOT_FOUND");
            }
        );

        test!("オーナーは脱退できない", async {
            let app = app().await;
            let owner = register(&app, "ol-owner").await;
            let id = create_community(&app, &owner, "no-leave", "PUBLIC").await;
            let (status, body) = call(
                &app,
                "DELETE",
                &format!("/communities/{id}/members/me"),
                Some(&owner.token),
                None,
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], "OWNER_CANNOT_LEAVE");
        });

        test!(
            "メンバー ID 指定の脱退は本人のメンバー ID のみ有効",
            async {
                let app = app().await;
                let owner = register(&app, "id-owner").await;
                let user = register(&app, "id-user").await;
                let other = register(&app, "id-other").await;
                let id = create_community(&app, &owner, "by-id", "PUBLIC").await;
                let members = format!("/communities/{id}/members");
                let (_, joined) = call(&app, "POST", &members, Some(&user.token), None).await;
                let member_id = joined["id"].as_str().unwrap();
                call(&app, "POST", &members, Some(&other.token), None).await;

                let (status, _) = call(
                    &app,
                    "DELETE",
                    &format!("{members}/{member_id}"),
                    Some(&other.token),
                    None,
                )
                .await;
                assert_eq!(
                    status,
                    StatusCode::NOT_FOUND,
                    "他人のメンバーIDでは脱退できない"
                );
                let (status, _) = call(
                    &app,
                    "DELETE",
                    &format!("{members}/{member_id}"),
                    Some(&user.token),
                    None,
                )
                .await;
                assert_eq!(status, StatusCode::OK);
            }
        );

        test!(
            "別コミュニティのメンバー ID では脱退できない",
            async {
                let app = app().await;
                let owner_a = register(&app, "lc-oa").await;
                let owner_b = register(&app, "lc-ob").await;
                let user = register(&app, "lc-user").await;
                let a = create_community(&app, &owner_a, "lc-a", "PUBLIC").await;
                let b = create_community(&app, &owner_b, "lc-b", "PUBLIC").await;
                let (_, in_a) = call(
                    &app,
                    "POST",
                    &format!("/communities/{a}/members"),
                    Some(&user.token),
                    None,
                )
                .await;
                call(
                    &app,
                    "POST",
                    &format!("/communities/{b}/members"),
                    Some(&user.token),
                    None,
                )
                .await;
                let member_in_a = in_a["id"].as_str().unwrap();

                // パスは B だが、メンバーID は A のもの → 404 で、B の所属は維持される
                let (status, _) = call(
                    &app,
                    "DELETE",
                    &format!("/communities/{b}/members/{member_in_a}"),
                    Some(&user.token),
                    None,
                )
                .await;
                assert_eq!(status, StatusCode::NOT_FOUND);
                let (status, _) = call(
                    &app,
                    "DELETE",
                    &format!("/communities/{b}/members/me"),
                    Some(&user.token),
                    None,
                )
                .await;
                assert_eq!(status, StatusCode::OK, "B の所属は削除されていなかった");
            }
        );
    });

    describe!("ロールと権限", {
        test!(
            "ADMIN は承認とイベント作成ができ、PENDING と非メンバーは 403",
            async {
                let (app, pool) = app_with_pool().await;
                let owner = register(&app, "ad-owner").await;
                let admin = register(&app, "ad-admin").await;
                let pending = register(&app, "ad-pending").await;
                let outsider = register(&app, "ad-out").await;
                let id = create_community(&app, &owner, "admin-role", "PRIVATE").await;
                let members = format!("/communities/{id}/members");

                let (_, admin_join) = call(&app, "POST", &members, Some(&admin.token), None).await;
                let (_, pending_join) =
                    call(&app, "POST", &members, Some(&pending.token), None).await;
                // ADMIN を付与する API は無いため、DB で昇格させる
                sqlx::query(
                    "UPDATE community_members SET role = 'ADMIN', status = 'ACTIVE' WHERE id = ?",
                )
                .bind(admin_join["id"].as_str().unwrap())
                .execute(&pool)
                .await
                .unwrap();

                let events = format!("/communities/{id}/events");
                for who in [&pending, &outsider] {
                    let (status, _) = call(
                        &app,
                        "POST",
                        &events,
                        Some(&who.token),
                        Some(event_body(24, 26)),
                    )
                    .await;
                    assert_eq!(status, StatusCode::FORBIDDEN);
                }
                let (status, _) = call(
                    &app,
                    "POST",
                    &events,
                    Some(&admin.token),
                    Some(event_body(24, 26)),
                )
                .await;
                assert_eq!(status, StatusCode::CREATED);

                let pending_id = pending_join["id"].as_str().unwrap();
                let (status, body) = call(
                    &app,
                    "PATCH",
                    &format!("{members}/{pending_id}/approve"),
                    Some(&admin.token),
                    None,
                )
                .await;
                assert_eq!(status, StatusCode::OK, "{body}");
            }
        );

        test!(
            "承認待ちのメンバーは管理ロールでも承認できない",
            async {
                let (app, pool) = app_with_pool().await;
                let owner = register(&app, "pa2-owner").await;
                let pending_admin = register(&app, "pa2-admin").await;
                let applicant = register(&app, "pa2-applicant").await;
                let id = create_community(&app, &owner, "pending-admin-role", "PRIVATE").await;
                let members = format!("/communities/{id}/members");
                let (_, admin_join) =
                    call(&app, "POST", &members, Some(&pending_admin.token), None).await;
                // ロールだけ ADMIN にし、status は PENDING のままにする
                sqlx::query("UPDATE community_members SET role = 'ADMIN' WHERE id = ?")
                    .bind(admin_join["id"].as_str().unwrap())
                    .execute(&pool)
                    .await
                    .unwrap();
                let (_, joined) = call(&app, "POST", &members, Some(&applicant.token), None).await;
                let target = joined["id"].as_str().unwrap();

                let (status, _) = call(
                    &app,
                    "PATCH",
                    &format!("{members}/{target}/approve"),
                    Some(&pending_admin.token),
                    None,
                )
                .await;
                assert_eq!(status, StatusCode::FORBIDDEN);
            }
        );
    });
});
