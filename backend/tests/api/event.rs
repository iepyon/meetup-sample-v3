//! イベント（作成・一覧と公開範囲）
use crate::common::*;
use meetup_test_macros::describe;

describe!("イベント", {
    describe!("作成", {
        test!(
            "オーナーはイベントを DRAFT で作成でき、作成者だけが一覧で見える",
            async {
                let app = app().await;
                let owner = register(&app, "ev-owner").await;
                let id = create_community(&app, &owner, "ev-comm", "PUBLIC").await;
                let path = format!("/communities/{id}/events");

                let (status, body) = call(
                    &app,
                    "POST",
                    &path,
                    Some(&owner.token),
                    Some(event_body(24, 26)),
                )
                .await;
                assert_eq!(status, StatusCode::CREATED, "{body}");
                assert_eq!(body["event"]["status"], "DRAFT");

                let (_, body) = call(&app, "GET", &path, None, None).await;
                assert_eq!(body["total"], 0, "DRAFT は未ログインには見えない");
                let (_, body) = call(&app, "GET", &path, Some(&owner.token), None).await;
                assert_eq!(body["total"], 1);
                assert_eq!(body["events"][0]["title"], "もくもく会");
            }
        );

        test!("一般メンバーはイベントを作成できない", async {
            let app = app().await;
            let owner = register(&app, "em-owner").await;
            let member = register(&app, "em-member").await;
            let id = create_community(&app, &owner, "em-comm", "PUBLIC").await;
            call(
                &app,
                "POST",
                &format!("/communities/{id}/members"),
                Some(&member.token),
                None,
            )
            .await;

            let (status, body) = call(
                &app,
                "POST",
                &format!("/communities/{id}/events"),
                Some(&member.token),
                Some(event_body(24, 26)),
            )
            .await;
            assert_eq!(status, StatusCode::FORBIDDEN);
            assert_eq!(body["code"], "FORBIDDEN");
        });

        test!("過去の開始日時と終了前後の不整合は 422", async {
            let app = app().await;
            let owner = register(&app, "ed-owner").await;
            let id = create_community(&app, &owner, "ed-comm", "PUBLIC").await;
            let path = format!("/communities/{id}/events");

            let (status, body) = call(
                &app,
                "POST",
                &path,
                Some(&owner.token),
                Some(event_body(-2, 1)),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], "EVENT_DATE_IN_PAST");

            let (status, body) = call(
                &app,
                "POST",
                &path,
                Some(&owner.token),
                Some(event_body(5, 3)),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], "EVENT_END_BEFORE_START");
        });

        test!("イベントの入力検証は 400", async {
            let app = app().await;
            let owner = register(&app, "ev-val").await;
            let id = create_community(&app, &owner, "ev-val-comm", "PUBLIC").await;
            let mut body = event_body(24, 26);
            body["capacity"] = json!(0);
            let (status, _) = call(
                &app,
                "POST",
                &format!("/communities/{id}/events"),
                Some(&owner.token),
                Some(body),
            )
            .await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
        });
    });

    describe!("一覧と公開範囲", {
        test!(
            "非公開コミュニティのイベント一覧は非メンバーに 404",
            async {
                let app = app().await;
                let owner = register(&app, "pe-owner").await;
                let outsider = register(&app, "pe-out").await;
                let id = create_community(&app, &owner, "private-events", "PRIVATE").await;
                let path = format!("/communities/{id}/events");

                assert_eq!(
                    call(&app, "GET", &path, None, None).await.0,
                    StatusCode::NOT_FOUND
                );
                assert_eq!(
                    call(&app, "GET", &path, Some(&outsider.token), None)
                        .await
                        .0,
                    StatusCode::NOT_FOUND
                );
                assert_eq!(
                    call(&app, "GET", &path, Some(&owner.token), None).await.0,
                    StatusCode::OK
                );
            }
        );

        test!(
            "存在しないコミュニティのイベント一覧は 404",
            async {
                let app = app().await;
                let (status, body) = call(
                    &app,
                    "GET",
                    "/communities/550e8400-e29b-41d4-a716-446655440000/events",
                    None,
                    None,
                )
                .await;
                assert_eq!(status, StatusCode::NOT_FOUND);
                assert_eq!(body["code"], "COMMUNITY_NOT_FOUND");
            }
        );

        test!(
            "PUBLISHED のイベントは未ログインにも見える",
            async {
                let (app, pool) = app_with_pool().await;
                let owner = register(&app, "pub-owner").await;
                let id = create_community(&app, &owner, "pub-events", "PUBLIC").await;
                let path = format!("/communities/{id}/events");
                call(
                    &app,
                    "POST",
                    &path,
                    Some(&owner.token),
                    Some(event_body(24, 26)),
                )
                .await;
                sqlx::query("UPDATE events SET status = 'PUBLISHED'")
                    .execute(&pool)
                    .await
                    .unwrap();

                let (_, body) = call(&app, "GET", &path, None, None).await;
                assert_eq!(body["total"], 1);
                assert_eq!(body["events"][0]["status"], "PUBLISHED");
            }
        );
    });
});
