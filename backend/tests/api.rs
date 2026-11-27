use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use meetup_test_runner::{Node, describe, run, test};
use serde_json::{Value, json};
use tower::ServiceExt;

mod common;
use common::*;

fn main() {
    run(vec![認証(), コミュニティ(), イベント()]);
}

fn 認証() -> Node {
    describe(
        "認証",
        vec![
            test(
                "登録するとトークンとアカウントが返りパスワードハッシュは含まれない",
                || async {
                    let app = app().await;
                    let body = json!({
                        "name": "太郎",
                        "email": "taro@example.com",
                        "password": "secret-pass",
                    });
                    let (status, body) =
                        call(&app, "POST", "/auth/register", None, Some(body)).await;
                    assert_eq!(status, StatusCode::CREATED);
                    assert!(body["token"].is_string());
                    assert_eq!(body["account"]["email"], "taro@example.com");
                    assert!(body["account"].get("passwordHash").is_none());
                },
            ),
            test(
                "同じメールアドレスの登録は 409 になる",
                || async {
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
                },
            ),
            test(
                "不正な入力は 400 の VALIDATION_ERROR になる",
                || async {
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
                },
            ),
            test("壊れた JSON は 400 の INVALID_JSON になる", || async {
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
            }),
            test(
                "ログインできる、誤ったパスワードは 401",
                || async {
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
                },
            ),
            test(
                "/auth/me は認証が必要で自分の情報を返す",
                || async {
                    let app = app().await;
                    let user = register(&app, "me").await;
                    let (status, _) = call(&app, "GET", "/auth/me", None, None).await;
                    assert_eq!(status, StatusCode::UNAUTHORIZED);
                    let (status, body) =
                        call(&app, "GET", "/auth/me", Some(&user.token), None).await;
                    assert_eq!(status, StatusCode::OK);
                    assert_eq!(body["account"]["id"], user.id);
                    let (status, _) = call(&app, "GET", "/auth/me", Some("garbage"), None).await;
                    assert_eq!(status, StatusCode::UNAUTHORIZED);
                },
            ),
        ],
    )
}

fn コミュニティ() -> Node {
    describe(
        "コミュニティ",
        vec![
            test("コミュニティを作成して取得できる", || async {
                let app = app().await;
                let owner = register(&app, "owner").await;
                let id = create_community(&app, &owner, "Rust 勉強会", "PUBLIC").await;
                let (status, body) =
                    call(&app, "GET", &format!("/communities/{id}"), None, None).await;
                assert_eq!(status, StatusCode::OK);
                assert_eq!(body["community"]["name"], "Rust 勉強会");
                assert_eq!(body["community"]["description"], "説明");
            }),
            test(
                "未ログインでのコミュニティ作成は 401",
                || async {
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
                },
            ),
            test(
                "同名コミュニティは 409、作成上限は 422",
                || async {
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
                },
            ),
            test(
                "作成上限に達したら既存の名前でも 422 で名前の存在を明かさない",
                || async {
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
                },
            ),
            test(
                "一覧は公開のみ、カテゴリで絞れ、member=me で自分の所属のみ",
                || async {
                    let app = app().await;
                    let owner = register(&app, "lister").await;
                    create_community(&app, &owner, "public-tech", "PUBLIC").await;
                    create_community(&app, &owner, "private-one", "PRIVATE").await;
                    let (_, _) = call(
                &app,
                "POST",
                "/communities",
                Some(&owner.token),
                Some(json!({ "name": "public-hobby", "category": "HOBBY", "visibility": "PUBLIC" })),
            )
            .await;

                    let (_, body) = call(&app, "GET", "/communities", None, None).await;
                    assert_eq!(body["total"], 2, "非公開は一覧に出ない: {body}");

                    let (_, body) =
                        call(&app, "GET", "/communities?category=HOBBY", None, None).await;
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
                },
            ),
            test(
                "非公開コミュニティはメンバー以外には 404",
                || async {
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
                },
            ),
        ],
    )
}

fn イベント() -> Node {
    describe(
        "イベント",
        vec![
            test(
                "オーナーはイベントを DRAFT で作成でき、作成者だけが一覧で見える",
                || async {
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
                },
            ),
            test(
                "一般メンバーはイベントを作成できない",
                || async {
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
                },
            ),
            test(
                "過去の開始日時と終了前後の不整合は 422",
                || async {
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
                },
            ),
            test(
                "非公開コミュニティのイベント一覧は非メンバーに 404",
                || async {
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
                },
            ),
            test(
                "存在しないコミュニティのイベント一覧は 404",
                || async {
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
                },
            ),
            test("イベントの入力検証は 400", || async {
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
            }),
        ],
    )
}
