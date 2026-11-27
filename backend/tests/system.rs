use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use meetup_backend::infrastructure::connect;
use meetup_backend::{AppState, create_app};
use meetup_test_runner::{Node, describe, run, test};
use serde_json::{Value, json};
use tower::ServiceExt;

mod common;
use common::*;

fn main() {
    run(vec![システム()]);
}

fn システム() -> Node {
    describe(
        "システム",
        vec![
            test("ヘルスチェックと OpenAPI が返る", || async {
                let app = app().await;
                let (status, body) = call(&app, "GET", "/health", None, None).await;
                assert_eq!(status, StatusCode::OK);
                assert_eq!(body["status"], "ok");
                let (status, body) = call(&app, "GET", "/openapi.json", None, None).await;
                assert_eq!(status, StatusCode::OK);
                assert_eq!(body["openapi"], "3.0.3");
            }),
            test(
                "未定義パスは 404、メソッド不一致は 405 の JSON",
                || async {
                    let app = app().await;
                    let (status, body) = call(&app, "GET", "/nope", None, None).await;
                    assert_eq!(status, StatusCode::NOT_FOUND);
                    assert_eq!(body["code"], "NOT_FOUND");
                    let (status, body) = call(&app, "DELETE", "/health", None, None).await;
                    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
                    assert_eq!(body["code"], "METHOD_NOT_ALLOWED");
                },
            ),
            test(
                "キャッシュヘッダは公開 GET のみ public",
                || async {
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
                },
            ),
            test(
                "レート制限を有効にすると上限超過で 429",
                || async {
                    let pool = connect("sqlite::memory:").await.unwrap();
                    let app = create_app(AppState::new(pool, SECRET), true);
                    let mut last = StatusCode::OK;
                    for _ in 0..61 {
                        last = call(&app, "GET", "/communities", None, None).await.0;
                    }
                    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
                },
            ),
            test(
                "承認待ちの申請者は非公開コミュニティを閲覧できない",
                || async {
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
                },
            ),
            test(
                "別コミュニティのメンバー ID では脱退できない",
                || async {
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
                },
            ),
            test(
                "未登録メールのログインも 401 で同じ応答になる",
                || async {
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
                },
            ),
            test(
                "ログインは短時間に繰り返すと 429 になる",
                || async {
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
                },
            ),
            test(
                "保護されたエンドポイントは認証なしで 401",
                || async {
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
                },
            ),
            test(
                "改ざんされたトークンは、保護 API で 401、公開 GET では匿名扱い",
                || async {
                    let app = app().await;
                    let (status, _) = call(&app, "GET", "/auth/me", Some("not.a.jwt"), None).await;
                    assert_eq!(status, StatusCode::UNAUTHORIZED);
                    let (status, body) =
                        call(&app, "GET", "/communities", Some("not.a.jwt"), None).await;
                    assert_eq!(status, StatusCode::OK);
                    assert_eq!(body["total"], 0);
                },
            ),
            test("存在しないリソースは 404", || async {
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
            }),
            test("UUID でないパスは 400", || async {
                let app = app().await;
                let (status, body) = call(&app, "GET", "/communities/abc", None, None).await;
                assert_eq!(status, StatusCode::BAD_REQUEST);
                assert_eq!(body["code"], "VALIDATION_ERROR");
            }),
            test(
                "ページネーションで件数と total が整合する",
                || async {
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
                },
            ),
            test("コミュニティ作成の入力不備は 400", || async {
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
            }),
            test(
                "拒否したメンバーは一覧から消え、再参加できる",
                || async {
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
                },
            ),
            test("オーナーのメンバーは拒否できない", || async {
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
            }),
            test("長すぎるパスワードは 400", || async {
                let app = app().await;
                let (status, _) = call(
                &app,
                "POST",
                "/auth/register",
                None,
                Some(json!({ "name": "n", "email": "long@example.com", "password": "a".repeat(73) })),
            )
            .await;
                assert_eq!(status, StatusCode::BAD_REQUEST);
            }),
            test(
                "ADMIN は承認とイベント作成ができ、PENDING と非メンバーは 403",
                || async {
                    let (app, pool) = app_with_pool().await;
                    let owner = register(&app, "ad-owner").await;
                    let admin = register(&app, "ad-admin").await;
                    let pending = register(&app, "ad-pending").await;
                    let outsider = register(&app, "ad-out").await;
                    let id = create_community(&app, &owner, "admin-role", "PRIVATE").await;
                    let members = format!("/communities/{id}/members");

                    let (_, admin_join) =
                        call(&app, "POST", &members, Some(&admin.token), None).await;
                    let (_, pending_join) =
                        call(&app, "POST", &members, Some(&pending.token), None).await;
                    // ADMIN を付与する API は無いため、DB で昇格させる
                    sqlx::query("UPDATE community_members SET role = 'ADMIN', status = 'ACTIVE' WHERE id = ?")
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
                },
            ),
            test(
                "PUBLISHED のイベントは未ログインにも見える",
                || async {
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
                },
            ),
            test(
                "OpenAPI は全ルートを含み、/docs は SRI 付き HTML を返す",
                || async {
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
                    let html = String::from_utf8(
                        res.into_body().collect().await.unwrap().to_bytes().to_vec(),
                    )
                    .unwrap();
                    assert!(html.contains("integrity=\"sha384-"));
                },
            ),
            test(
                "メソッド不一致の 405 は Allow ヘッダ付き",
                || async {
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
                },
            ),
            test("巨大なボディは 413 の JSON", || async {
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
            }),
            test(
                "一覧 API は参加で増えた件数と total が一致する",
                || async {
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
                },
            ),
        ],
    )
}
