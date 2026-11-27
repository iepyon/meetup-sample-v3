//! 認証（新規登録・ログイン・トークン）
use crate::common::*;
use meetup_test_macros::describe;

describe!("認証", {
    describe!("新規登録", {
        test!(
            "登録するとトークンとアカウントが返りパスワードハッシュは含まれない",
            async {
                let app = app().await;
                let body = json!({
                    "name": "太郎",
                    "email": "taro@example.com",
                    "password": "secret-pass",
                });
                let (status, body) = call(&app, "POST", "/auth/register", None, Some(body)).await;
                assert_eq!(status, StatusCode::CREATED);
                assert!(body["token"].is_string());
                assert_eq!(body["account"]["email"], "taro@example.com");
                assert!(body["account"].get("passwordHash").is_none());
            }
        );

        test!("同じメールアドレスの登録は 409 になる", async {
            let app = app().await;
            register(&app, "dup").await;
            let (status, body) = call(
                &app,
                "POST",
                "/auth/register",
                None,
                Some(json!({ "name": "x", "email": "dup@example.com", "password": "password1" })),
            )
            .await;
            assert_eq!(status, StatusCode::CONFLICT);
            assert_eq!(body["code"], "DUPLICATE_EMAIL");
        });

        test!("不正な入力は 400 の VALIDATION_ERROR になる", async {
            let app = app().await;
            let (status, body) = call(
                &app,
                "POST",
                "/auth/register",
                None,
                Some(json!({ "name": "", "email": "bad", "password": "p" })),
            )
            .await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(body["code"], "VALIDATION_ERROR");
        });

        test!("長すぎるパスワードは 400", async {
            let app = app().await;
            let (status, _) = call(
                &app,
                "POST",
                "/auth/register",
                None,
                Some(
                    json!({ "name": "n", "email": "long@example.com", "password": "a".repeat(73) }),
                ),
            )
            .await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
        });

        test!(
            "メールアドレスは大文字小文字を区別せず登録とログインができる",
            async {
                let app = app().await;
                let body = json!({ "name": "case", "email": "Case.User@Example.com", "password": "pass1234" });
                let (status, _) = call(&app, "POST", "/auth/register", None, Some(body)).await;
                assert_eq!(status, StatusCode::CREATED);

                let dup = json!({
                    "name": "case2",
                    "email": "case.user@example.com",
                    "password": "pass1234",
                });
                let (status, body) = call(&app, "POST", "/auth/register", None, Some(dup)).await;
                assert_eq!(status, StatusCode::CONFLICT);
                assert!(!body["message"].as_str().unwrap().contains('@'));

                let login = json!({ "email": "CASE.USER@example.com", "password": "pass1234" });
                let (status, _) = call(&app, "POST", "/auth/login", None, Some(login)).await;
                assert_eq!(status, StatusCode::OK);
            }
        );

        test!(
            "新規登録のパスワードが 8 文字未満なら 400",
            async {
                let app = app().await;
                let body =
                    json!({ "name": "short", "email": "short@example.com", "password": "1234567" });
                let (status, body) = call(&app, "POST", "/auth/register", None, Some(body)).await;
                assert_eq!(status, StatusCode::BAD_REQUEST);
                assert_eq!(body["code"], "VALIDATION_ERROR");
            }
        );
    });

    describe!("ログイン", {
        test!("ログインできる、誤ったパスワードは 401", async {
            let app = app().await;
            register(&app, "login").await;
            let (ok_status, ok_body) = call(
                &app,
                "POST",
                "/auth/login",
                None,
                Some(json!({ "email": "login@example.com", "password": "pass1234" })),
            )
            .await;
            assert_eq!(ok_status, StatusCode::OK);
            assert!(ok_body["token"].is_string());

            let (status, body) = call(
                &app,
                "POST",
                "/auth/login",
                None,
                Some(json!({ "email": "login@example.com", "password": "wrong" })),
            )
            .await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(body["code"], "INVALID_CREDENTIALS");
        });

        test!(
            "未登録メールのログインも 401 で同じ応答になる",
            async {
                let app = app().await;
                let (status, body) = call(
                    &app,
                    "POST",
                    "/auth/login",
                    None,
                    Some(json!({ "email": "nobody@example.com", "password": "x" })),
                )
                .await;
                assert_eq!(status, StatusCode::UNAUTHORIZED);
                assert_eq!(body["code"], "INVALID_CREDENTIALS");
            }
        );
    });

    describe!("トークン", {
        test!("/auth/me は認証が必要で自分の情報を返す", async {
            let app = app().await;
            let user = register(&app, "me").await;
            let (status, _) = call(&app, "GET", "/auth/me", None, None).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            let (status, body) = call(&app, "GET", "/auth/me", Some(&user.token), None).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body["account"]["id"], user.id);
            let (status, _) = call(&app, "GET", "/auth/me", Some("garbage"), None).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
        });

        test!(
            "保護されたエンドポイントは認証なしで 401",
            async {
                let app = app().await;
                let owner = register(&app, "na-owner").await;
                let id = create_community(&app, &owner, "no-auth", "PUBLIC").await;
                let member = "550e8400-e29b-41d4-a716-446655440000";
                let cases = [
                    ("POST", format!("/communities/{id}/members")),
                    ("DELETE", format!("/communities/{id}/members/me")),
                    ("DELETE", format!("/communities/{id}/members/{member}")),
                    (
                        "PATCH",
                        format!("/communities/{id}/members/{member}/approve"),
                    ),
                    (
                        "PATCH",
                        format!("/communities/{id}/members/{member}/reject"),
                    ),
                ];
                for (method, path) in cases {
                    let (status, _) = call(&app, method, &path, None, None).await;
                    assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
                }
                let (status, _) = call(
                    &app,
                    "POST",
                    &format!("/communities/{id}/events"),
                    None,
                    Some(event_body(24, 26)),
                )
                .await;
                assert_eq!(status, StatusCode::UNAUTHORIZED);
            }
        );

        test!(
            "改ざんされたトークンは、保護 API で 401、公開 GET では匿名扱い",
            async {
                let app = app().await;
                let (status, _) = call(&app, "GET", "/auth/me", Some("not.a.jwt"), None).await;
                assert_eq!(status, StatusCode::UNAUTHORIZED);
                let (status, body) =
                    call(&app, "GET", "/communities", Some("not.a.jwt"), None).await;
                assert_eq!(status, StatusCode::OK);
                assert_eq!(body["total"], 0);
            }
        );
    });
});
