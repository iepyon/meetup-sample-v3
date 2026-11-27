//! コミュニティ（作成・一覧・閲覧）
use crate::common::*;
use meetup_test_macros::describe;

describe!("コミュニティ", {
    describe!("作成", {
        test!("コミュニティを作成して取得できる", async {
            let app = app().await;
            let owner = register(&app, "owner").await;
            let id = create_community(&app, &owner, "Rust 勉強会", "PUBLIC").await;
            let (status, body) = call(&app, "GET", &format!("/communities/{id}"), None, None).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body["community"]["name"], "Rust 勉強会");
            assert_eq!(body["community"]["description"], "説明");
        });

        test!("未ログインでのコミュニティ作成は 401", async {
            let app = app().await;
            let (status, _) = call(
                &app,
                "POST",
                "/communities",
                None,
                Some(json!({ "name": "n", "category": "TECH", "visibility": "PUBLIC" })),
            )
            .await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
        });

        test!("同名コミュニティは 409、作成上限は 422", async {
            let app = app().await;
            let owner = register(&app, "limit").await;
            create_community(&app, &owner, "same", "PUBLIC").await;
            let (status, body) = call(
                &app,
                "POST",
                "/communities",
                Some(&owner.token),
                Some(json!({ "name": "same", "category": "TECH", "visibility": "PUBLIC" })),
            )
            .await;
            assert_eq!(status, StatusCode::CONFLICT);
            assert_eq!(body["code"], "DUPLICATE_COMMUNITY_NAME");

            for i in 1..10 {
                create_community(&app, &owner, &format!("c{i}"), "PUBLIC").await;
            }
            let (status, body) = call(
                &app,
                "POST",
                "/communities",
                Some(&owner.token),
                Some(json!({ "name": "eleventh", "category": "TECH", "visibility": "PUBLIC" })),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], "TOO_MANY_COMMUNITIES");
        });

        test!(
            "作成上限に達したら既存の名前でも 422 で名前の存在を明かさない",
            async {
                let app = app().await;
                let owner = register(&app, "limit-hide").await;
                let other = register(&app, "limit-other").await;
                create_community(&app, &other, "secret-name", "PRIVATE").await;
                for i in 0..10 {
                    create_community(&app, &owner, &format!("h{i}"), "PUBLIC").await;
                }
                let (status, body) = call(
                    &app,
                    "POST",
                    "/communities",
                    Some(&owner.token),
                    Some(json!({ "name": "secret-name", "category": "TECH", "visibility": "PUBLIC" })),
                )
                .await;
                assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
                assert_eq!(body["code"], "TOO_MANY_COMMUNITIES");
            }
        );

        test!("コミュニティ作成の入力不備は 400", async {
            let app = app().await;
            let owner = register(&app, "cv-owner").await;
            for body in [
                json!({ "category": "TECH", "visibility": "PUBLIC" }),
                json!({ "name": "", "category": "TECH", "visibility": "PUBLIC" }),
                json!({ "name": "x".repeat(101), "category": "TECH", "visibility": "PUBLIC" }),
                json!({ "name": "n", "category": "NOPE", "visibility": "PUBLIC" }),
                json!({ "name": "n", "category": "TECH", "visibility": "NOPE" }),
            ] {
                let (status, _) = call(
                    &app,
                    "POST",
                    "/communities",
                    Some(&owner.token),
                    Some(body.clone()),
                )
                .await;
                assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
            }
        });
    });

    describe!("一覧", {
        test!(
            "一覧は公開のみ、カテゴリで絞れ、member=me で自分の所属のみ",
            async {
                let app = app().await;
                let owner = register(&app, "lister").await;
                create_community(&app, &owner, "public-tech", "PUBLIC").await;
                create_community(&app, &owner, "private-one", "PRIVATE").await;
                let (_, _) = call(
                    &app,
                    "POST",
                    "/communities",
                    Some(&owner.token),
                    Some(
                        json!({ "name": "public-hobby", "category": "HOBBY", "visibility": "PUBLIC" }),
                    ),
                )
                .await;

                let (_, body) = call(&app, "GET", "/communities", None, None).await;
                assert_eq!(body["total"], 2, "非公開は一覧に出ない: {body}");

                let (_, body) = call(&app, "GET", "/communities?category=HOBBY", None, None).await;
                assert_eq!(body["total"], 1);
                assert_eq!(body["communities"][0]["name"], "public-hobby");

                let (status, _) = call(&app, "GET", "/communities?member=me", None, None).await;
                assert_eq!(status, StatusCode::UNAUTHORIZED);
                let (_, body) = call(
                    &app,
                    "GET",
                    "/communities?member=me",
                    Some(&owner.token),
                    None,
                )
                .await;
                assert_eq!(body["total"], 3);

                let (status, _) = call(&app, "GET", "/communities?limit=101", None, None).await;
                assert_eq!(status, StatusCode::BAD_REQUEST);
            }
        );

        test!(
            "ページネーションで件数と total が整合する",
            async {
                let app = app().await;
                let owner = register(&app, "pg-owner").await;
                for i in 0..3 {
                    create_community(&app, &owner, &format!("pg-{i}"), "PUBLIC").await;
                }
                let (_, body) =
                    call(&app, "GET", "/communities?limit=2&offset=1", None, None).await;
                assert_eq!(body["communities"].as_array().unwrap().len(), 2);
                assert_eq!(body["total"], 3);
                assert_eq!(
                    call(&app, "GET", "/communities?limit=100", None, None)
                        .await
                        .0,
                    StatusCode::OK
                );
                for bad in ["limit=-1", "limit=abc", "offset=-1"] {
                    assert_eq!(
                        call(&app, "GET", &format!("/communities?{bad}"), None, None)
                            .await
                            .0,
                        StatusCode::BAD_REQUEST,
                        "{bad}"
                    );
                }
            }
        );

        test!(
            "一覧 API は参加で増えた件数と total が一致する",
            async {
                let app = app().await;
                let owner = register(&app, "tt-owner").await;
                let id = create_community(&app, &owner, "total-check", "PUBLIC").await;
                for i in 0..3 {
                    let u = register(&app, &format!("tt-u{i}")).await;
                    call(
                        &app,
                        "POST",
                        &format!("/communities/{id}/members"),
                        Some(&u.token),
                        None,
                    )
                    .await;
                }
                let (_, body) = call(
                    &app,
                    "GET",
                    &format!("/communities/{id}/members?limit=2&offset=1"),
                    None,
                    None,
                )
                .await;
                assert_eq!(body["members"].as_array().unwrap().len(), 2);
                assert_eq!(body["total"], 4);
            }
        );
    });

    describe!("閲覧", {
        test!(
            "非公開コミュニティはメンバー以外には 404",
            async {
                let app = app().await;
                let owner = register(&app, "priv-owner").await;
                let other = register(&app, "priv-other").await;
                let id = create_community(&app, &owner, "secret", "PRIVATE").await;
                let path = format!("/communities/{id}");
                assert_eq!(
                    call(&app, "GET", &path, None, None).await.0,
                    StatusCode::NOT_FOUND
                );
                assert_eq!(
                    call(&app, "GET", &path, Some(&other.token), None).await.0,
                    StatusCode::NOT_FOUND
                );
                assert_eq!(
                    call(&app, "GET", &path, Some(&owner.token), None).await.0,
                    StatusCode::OK
                );
            }
        );

        test!(
            "承認待ちの申請者は非公開コミュニティを閲覧できない",
            async {
                let app = app().await;
                let owner = register(&app, "pd-owner").await;
                let applicant = register(&app, "pd-app").await;
                let id = create_community(&app, &owner, "pending-view", "PRIVATE").await;
                let (_, joined) = call(
                    &app,
                    "POST",
                    &format!("/communities/{id}/members"),
                    Some(&applicant.token),
                    None,
                )
                .await;
                assert_eq!(joined["status"], "PENDING");

                let (status, _) = call(
                    &app,
                    "GET",
                    &format!("/communities/{id}"),
                    Some(&applicant.token),
                    None,
                )
                .await;
                assert_eq!(status, StatusCode::NOT_FOUND);

                let member_id = joined["id"].as_str().unwrap();
                call(
                    &app,
                    "PATCH",
                    &format!("/communities/{id}/members/{member_id}/approve"),
                    Some(&owner.token),
                    None,
                )
                .await;
                let (status, _) = call(
                    &app,
                    "GET",
                    &format!("/communities/{id}"),
                    Some(&applicant.token),
                    None,
                )
                .await;
                assert_eq!(status, StatusCode::OK, "承認後は閲覧できる");
            }
        );
    });
});
