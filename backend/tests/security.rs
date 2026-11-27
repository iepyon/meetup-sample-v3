use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

mod common;
use common::*;

// ------------------------------------------------------------
// セキュリティ
// ------------------------------------------------------------

async fn raw(app: &Router, req: Request<Body>) -> axum::response::Response {
    app.clone().oneshot(req).await.unwrap()
}

#[tokio::test]
async fn 承認待ちのメンバーは管理ロールでも承認できない() {
    let (app, pool) = app_with_pool().await;
    let owner = register(&app, "pa2-owner").await;
    let pending_admin = register(&app, "pa2-admin").await;
    let applicant = register(&app, "pa2-applicant").await;
    let id = create_community(&app, &owner, "pending-admin-role", "PRIVATE").await;
    let members = format!("/communities/{id}/members");
    let (_, admin_join) = call(&app, "POST", &members, Some(&pending_admin.token), None).await;
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

#[tokio::test]
async fn json以外のボディは415になる() {
    let app = app().await;
    let req = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header("content-type", "text/plain")
        .body(Body::from(r#"{"email":"a@example.com","password":"x"}"#))
        .unwrap();
    let res = raw(&app, req).await;
    assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn charset付きのjsonは受け付ける() {
    let app = app().await;
    let req = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header("content-type", "application/json; charset=utf-8")
        .body(Body::from(r#"{"email":"none@example.com","password":"x"}"#))
        .unwrap();
    let res = raw(&app, req).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn 全応答にセキュリティヘッダが付きjsonにはcspが付く() {
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

#[tokio::test]
async fn 公開getでもエラー応答は共有キャッシュに載せない() {
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

#[tokio::test]
async fn メールアドレスは大文字小文字を区別せず登録とログインができる() {
    let app = app().await;
    let body = json!({ "name": "case", "email": "Case.User@Example.com", "password": "pass1234" });
    let (status, _) = call(&app, "POST", "/auth/register", None, Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);

    let dup = json!({ "name": "case2", "email": "case.user@example.com", "password": "pass1234" });
    let (status, body) = call(&app, "POST", "/auth/register", None, Some(dup)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(!body["message"].as_str().unwrap().contains('@'));

    let login = json!({ "email": "CASE.USER@example.com", "password": "pass1234" });
    let (status, _) = call(&app, "POST", "/auth/login", None, Some(login)).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn 新規登録のパスワードが8文字未満なら400() {
    let app = app().await;
    let body = json!({ "name": "short", "email": "short@example.com", "password": "1234567" });
    let (status, body) = call(&app, "POST", "/auth/register", None, Some(body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn ボディのないpostはcontent_typeなしでも受け付ける() {
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

// ------------------------------------------------------------
// 本番構成（FRONTEND_DIST で SPA を同一オリジン配信）
// ------------------------------------------------------------

const HASHED_ASSET: &str = "app-0123456789abcdef.js";

async fn spa_app() -> Router {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("meetup-spa-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<!doctype html><title>spa</title>").unwrap();
    std::fs::write(dir.join(HASHED_ASSET), "console.log(1)").unwrap();
    let pool = meetup_backend::infrastructure::connect("sqlite::memory:")
        .await
        .unwrap();
    meetup_backend::create_app_with_frontend(
        meetup_backend::AppState::new(pool, SECRET),
        false,
        Some(dir.to_string_lossy().into_owned()),
    )
}

#[tokio::test]
async fn spaのページ遷移はindex_htmlを返しセキュリティヘッダが付く() {
    let app = spa_app().await;
    let req = Request::builder()
        .uri("/communities/0f8fad5b-d9cb-469f-a165-70867728950e")
        .header("accept", "text/html")
        .body(Body::empty())
        .unwrap();
    let res = raw(&app, req).await;
    assert_eq!(res.status(), StatusCode::OK);
    let h = res.headers();
    assert!(h["content-type"].to_str().unwrap().starts_with("text/html"));
    assert_eq!(h["cache-control"], "no-cache");
    assert_eq!(h["x-content-type-options"], "nosniff");
    assert!(h.get("content-security-policy").is_none());
}

#[tokio::test]
async fn spa構成でもjsonのapi呼び出しはそのまま通る() {
    let app = spa_app().await;
    let req = Request::builder()
        .uri("/communities")
        .header("accept", "application/json")
        .body(Body::empty())
        .unwrap();
    let res = raw(&app, req).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        res.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );
    assert!(res.headers().get("content-security-policy").is_some());
}

#[tokio::test]
async fn ハッシュ付きの静的アセットは長期キャッシュされる() {
    let app = spa_app().await;
    let req = Request::builder()
        .uri(format!("/{HASHED_ASSET}"))
        .body(Body::empty())
        .unwrap();
    let res = raw(&app, req).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()["cache-control"],
        "public, max-age=31536000, immutable"
    );
    assert_eq!(res.headers()["x-content-type-options"], "nosniff");
}

#[tokio::test]
async fn activeなメンバーは参加申請の拒否で外せない() {
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
