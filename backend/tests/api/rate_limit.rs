//! レート制限と Cache-Control
use crate::common::*;
use meetup_test_macros::describe;

describe!("レート制限とキャッシュ", {
    describe!("レート制限", {
        test!("レート制限を有効にすると上限超過で 429", async {
            let pool = connect("sqlite::memory:").await.unwrap();
            let app = create_app(AppState::new(pool, SECRET), true);
            let mut last = StatusCode::OK;
            for _ in 0..61 {
                last = call(&app, "GET", "/communities", None, None).await.0;
            }
            assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
        });

        test!("ログインは短時間に繰り返すと 429 になる", async {
            let pool = connect("sqlite::memory:").await.unwrap();
            let app = create_app(AppState::new(pool, SECRET), true);
            let mut last = StatusCode::OK;
            for _ in 0..21 {
                last = call(
                    &app,
                    "POST",
                    "/auth/login",
                    None,
                    Some(json!({ "email": "a@example.com", "password": "x" })),
                )
                .await
                .0;
            }
            assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
        });
    });

    describe!("キャッシュ", {
        test!("キャッシュヘッダは公開 GET のみ public", async {
            let app = app().await;
            let owner = register(&app, "cache").await;
            let public = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/communities")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert!(
                public.headers()["cache-control"]
                    .to_str()
                    .unwrap()
                    .starts_with("public")
            );
            let authed = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/communities")
                        .header("authorization", format!("Bearer {}", owner.token))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(authed.headers()["cache-control"], "private, no-store");
        });

        test!(
            "公開 GET でもエラー応答は共有キャッシュに載せない",
            async {
                let app = app().await;
                let req = Request::builder()
                    .uri("/communities/00000000-0000-4000-8000-000000000000")
                    .body(Body::empty())
                    .unwrap();
                let res = raw(&app, req).await;
                assert_eq!(res.status(), StatusCode::NOT_FOUND);
                assert_eq!(res.headers()["cache-control"], "private, no-store");
                assert_eq!(res.headers()["vary"], "Authorization, Accept");
            }
        );
    });
});
