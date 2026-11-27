//! HTTP の共通の挙動（ルーティング・リクエストボディ・セキュリティヘッダ・ヘルスチェックと OpenAPI）
use crate::common::*;
use meetup_test_macros::describe;

describe!("HTTP の共通の挙動", {
    describe!("ルーティングと存在しないリソース", {
        test!(
            "未定義パスは 404、メソッド不一致は 405 の JSON",
            async {
                let app = app().await;
                let (status, body) = call(&app, "GET", "/nope", None, None).await;
                assert_eq!(status, StatusCode::NOT_FOUND);
                assert_eq!(body["code"], "NOT_FOUND");
                let (status, body) = call(&app, "DELETE", "/health", None, None).await;
                assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
                assert_eq!(body["code"], "METHOD_NOT_ALLOWED");
            }
        );

        test!("存在しないリソースは 404", async {
            let app = app().await;
            let user = register(&app, "nf-user").await;
            let ghost = "550e8400-e29b-41d4-a716-446655440000";
            for (method, path) in [
                ("GET", format!("/communities/{ghost}")),
                ("GET", format!("/communities/{ghost}/members")),
                ("POST", format!("/communities/{ghost}/members")),
            ] {
                let (status, body) = call(&app, method, &path, Some(&user.token), None).await;
                assert_eq!(status, StatusCode::NOT_FOUND, "{method} {path}");
                assert_eq!(body["code"], "COMMUNITY_NOT_FOUND");
            }
        });

        test!("UUID でないパスは 400", async {
            let app = app().await;
            let (status, body) = call(&app, "GET", "/communities/abc", None, None).await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(body["code"], "VALIDATION_ERROR");
        });

        test!("メソッド不一致の 405 は Allow ヘッダ付き", async {
            let app = app().await;
            let res = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("DELETE")
                        .uri("/health")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
            assert!(res.headers().contains_key("allow"));
        });
    });

    describe!("リクエストボディ", {
        test!("壊れた JSON は 400 の INVALID_JSON になる", async {
            let app = app().await;
            let req = Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from("{not json"))
                .unwrap();
            let res = app.clone().oneshot(req).await.unwrap();
            assert_eq!(res.status(), StatusCode::BAD_REQUEST);
            let bytes = res.into_body().collect().await.unwrap().to_bytes();
            let body: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["code"], "INVALID_JSON");
        });

        test!("巨大なボディは 413 の JSON", async {
            let app = app().await;
            let big = "x".repeat(200 * 1024);
            let req = Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(format!("{{\"email\":\"{big}\"}}")))
                .unwrap();
            let res = app.clone().oneshot(req).await.unwrap();
            assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
            let bytes = res.into_body().collect().await.unwrap().to_bytes();
            let body: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["code"], "PAYLOAD_TOO_LARGE");
        });

        test!("JSON 以外のボディは 415 になる", async {
            let app = app().await;
            let req = Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "text/plain")
                .body(Body::from(r#"{"email":"a@example.com","password":"x"}"#))
                .unwrap();
            let res = raw(&app, req).await;
            assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        });

        test!("charset 付きの JSON は受け付ける", async {
            let app = app().await;
            let req = Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json; charset=utf-8")
                .body(Body::from(r#"{"email":"none@example.com","password":"x"}"#))
                .unwrap();
            let res = raw(&app, req).await;
            assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        });

        test!(
            "ボディのない POST は Content-Type なしでも受け付ける",
            async {
                let app = app().await;
                let owner = register(&app, "nb-owner").await;
                let user = register(&app, "nb-user").await;
                let id = create_community(&app, &owner, "no-body", "PUBLIC").await;
                let req = Request::builder()
                    .method("POST")
                    .uri(format!("/communities/{id}/members"))
                    .header("authorization", format!("Bearer {}", user.token))
                    .body(Body::empty())
                    .unwrap();
                let res = raw(&app, req).await;
                assert_eq!(res.status(), StatusCode::CREATED);
            }
        );
    });

    describe!("セキュリティヘッダ", {
        test!(
            "全応答にセキュリティヘッダが付き、JSON には CSP が付く",
            async {
                let app = app().await;
                let req = Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap();
                let res = raw(&app, req).await;
                let h = res.headers();
                assert_eq!(h["x-content-type-options"], "nosniff");
                assert_eq!(h["x-frame-options"], "DENY");
                assert_eq!(h["referrer-policy"], "no-referrer");
                assert_eq!(
                    h["content-security-policy"],
                    "default-src 'none'; frame-ancestors 'none'"
                );

                // /docs（Swagger UI の HTML）には CSP を付けない
                let req = Request::builder().uri("/docs").body(Body::empty()).unwrap();
                let res = raw(&app, req).await;
                assert_eq!(res.headers()["x-content-type-options"], "nosniff");
                assert!(res.headers().get("content-security-policy").is_none());
            }
        );
    });

    describe!("ヘルスチェックと OpenAPI", {
        test!("ヘルスチェックと OpenAPI が返る", async {
            let app = app().await;
            let (status, body) = call(&app, "GET", "/health", None, None).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body["status"], "ok");
            let (status, body) = call(&app, "GET", "/openapi.json", None, None).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body["openapi"], "3.0.3");
        });

        test!(
            "OpenAPI は全ルートを含み、/docs は SRI 付き HTML を返す",
            async {
                let app = app().await;
                let (_, doc) = call(&app, "GET", "/openapi.json", None, None).await;
                for path in [
                    "/health",
                    "/auth/register",
                    "/auth/login",
                    "/auth/me",
                    "/communities",
                    "/communities/{id}",
                    "/communities/{id}/members",
                    "/communities/{id}/members/me",
                    "/communities/{id}/members/{memberId}",
                    "/communities/{id}/members/{memberId}/approve",
                    "/communities/{id}/members/{memberId}/reject",
                    "/communities/{id}/events",
                ] {
                    assert!(doc["paths"].get(path).is_some(), "{path} が OpenAPI に無い");
                }
                let res = app
                    .clone()
                    .oneshot(Request::builder().uri("/docs").body(Body::empty()).unwrap())
                    .await
                    .unwrap();
                assert_eq!(res.status(), StatusCode::OK);
                let html =
                    String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec())
                        .unwrap();
                assert!(html.contains("integrity=\"sha384-"));
            }
        );
    });
});
