use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use meetup_backend::infrastructure::connect;
use meetup_backend::{AppState, create_app};
use serde_json::{Value, json};
use tower::ServiceExt;

const SECRET: &str = "test-secret";

async fn app() -> Router {
    let pool = connect("sqlite::memory:").await.expect("connect");
    create_app(AppState::new(pool, SECRET), false)
}

async fn app_with_pool() -> (Router, sqlx::SqlitePool) {
    let pool = connect("sqlite::memory:").await.expect("connect");
    (create_app(AppState::new(pool.clone(), SECRET), false), pool)
}

async fn call(
    app: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    let req = if let Some(b) = body {
        builder
            .header("content-type", "application/json")
            .body(Body::from(b.to_string()))
    } else {
        builder.body(Body::empty())
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

struct User {
    id: String,
    token: String,
}

async fn register(app: &Router, name: &str) -> User {
    let (status, body) = call(
        app,
        "POST",
        "/auth/register",
        None,
        Some(
            json!({ "name": name, "email": format!("{name}@example.com"), "password": "pass1234" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    User {
        id: body["account"]["id"].as_str().unwrap().to_string(),
        token: body["token"].as_str().unwrap().to_string(),
    }
}

async fn create_community(app: &Router, user: &User, name: &str, visibility: &str) -> String {
    let (status, body) = call(
        app,
        "POST",
        "/communities",
        Some(&user.token),
        Some(json!({ "name": name, "description": "説明", "category": "TECH", "visibility": visibility })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["community"]["id"].as_str().unwrap().to_string()
}

// ------------------------------------------------------------
// 認証
// ------------------------------------------------------------

#[tokio::test]
async fn 登録するとトークンとアカウントが返りパスワードハッシュは含まれない() {
    let app = app().await;
    let (status, body) = call(
        &app,
        "POST",
        "/auth/register",
        None,
        Some(json!({ "name": "太郎", "email": "taro@example.com", "password": "secret" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body["token"].is_string());
    assert_eq!(body["account"]["email"], "taro@example.com");
    assert!(body["account"].get("passwordHash").is_none());
}

#[tokio::test]
async fn 同じメールアドレスの登録は409になる() {
    let app = app().await;
    register(&app, "dup").await;
    let (status, body) = call(
        &app,
        "POST",
        "/auth/register",
        None,
        Some(json!({ "name": "x", "email": "dup@example.com", "password": "p" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "DUPLICATE_EMAIL");
}

#[tokio::test]
async fn 不正な入力は400のvalidation_errorになる() {
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
}

#[tokio::test]
async fn 壊れたjsonは400のinvalid_jsonになる() {
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
}

#[tokio::test]
async fn ログインできる_誤ったパスワードは401() {
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
}

#[tokio::test]
async fn auth_meは認証が必要で自分の情報を返す() {
    let app = app().await;
    let user = register(&app, "me").await;
    let (status, _) = call(&app, "GET", "/auth/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = call(&app, "GET", "/auth/me", Some(&user.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["account"]["id"], user.id);
    let (status, _) = call(&app, "GET", "/auth/me", Some("garbage"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ------------------------------------------------------------
// コミュニティ
// ------------------------------------------------------------

#[tokio::test]
async fn コミュニティを作成して取得できる() {
    let app = app().await;
    let owner = register(&app, "owner").await;
    let id = create_community(&app, &owner, "Rust 勉強会", "PUBLIC").await;
    let (status, body) = call(&app, "GET", &format!("/communities/{id}"), None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["community"]["name"], "Rust 勉強会");
    assert_eq!(body["community"]["description"], "説明");
}

#[tokio::test]
async fn 未ログインでのコミュニティ作成は401() {
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
}

#[tokio::test]
async fn 同名コミュニティは409_作成上限は422() {
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
}

#[tokio::test]
async fn 一覧は公開のみ_カテゴリで絞れる_member_meで自分の所属のみ() {
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

#[tokio::test]
async fn 非公開コミュニティはメンバー以外には404() {
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

// ------------------------------------------------------------
// メンバー
// ------------------------------------------------------------

#[tokio::test]
async fn 公開コミュニティへの参加はactive_重複参加は409_脱退できる() {
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

#[tokio::test]
async fn オーナーは脱退できない() {
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
}

#[tokio::test]
async fn メンバーid指定の脱退は本人のメンバーidのみ有効() {
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

#[tokio::test]
async fn 非公開コミュニティは承認制_承認と拒否ができる() {
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

#[tokio::test]
async fn 別コミュニティのメンバーは承認できない() {
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

#[tokio::test]
async fn メンバー一覧のaccountidはactiveメンバーにだけ見える() {
    let app = app().await;
    let owner = register(&app, "ls-owner").await;
    let outsider = register(&app, "ls-out").await;
    let id = create_community(&app, &owner, "listing", "PUBLIC").await;
    let path = format!("/communities/{id}/members");

    let (status, body) = call(&app, "GET", &path, None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["members"][0]["accountName"], "ls-owner");
    assert!(body["members"][0].get("accountId").is_none());

    let (_, body) = call(&app, "GET", &path, Some(&outsider.token), None).await;
    assert!(body["members"][0].get("accountId").is_none());

    let (_, body) = call(&app, "GET", &path, Some(&owner.token), None).await;
    assert_eq!(body["members"][0]["accountId"], owner.id);
}

#[tokio::test]
async fn 非公開コミュニティのメンバー一覧は非メンバーに404() {
    let app = app().await;
    let owner = register(&app, "pl-owner").await;
    let id = create_community(&app, &owner, "private-list", "PRIVATE").await;
    let path = format!("/communities/{id}/members");
    assert_eq!(
        call(&app, "GET", &path, None, None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, "GET", &path, Some(&owner.token), None).await.0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn オーナーのメンバー一覧には承認待ちの申請者が含まれる() {
    let app = app().await;
    let owner = register(&app, "pd-owner").await;
    let applicant = register(&app, "pd-applicant").await;
    let id = create_community(&app, &owner, "pending-owner", "PRIVATE").await;
    let path = format!("/communities/{id}/members");
    call(&app, "POST", &path, Some(&applicant.token), None).await;

    let (status, body) = call(&app, "GET", &path, Some(&owner.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 2);
    let members = body["members"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    let pending: Vec<&Value> = members
        .iter()
        .filter(|m| m["status"] == "PENDING")
        .collect();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0]["accountName"], "pd-applicant");
}

#[tokio::test]
async fn 管理者のメンバー一覧には承認待ちの申請者が含まれる() {
    let (app, pool) = app_with_pool().await;
    let owner = register(&app, "pa-owner").await;
    let admin = register(&app, "pa-admin").await;
    let applicant = register(&app, "pa-applicant").await;
    let id = create_community(&app, &owner, "pending-admin", "PRIVATE").await;
    let path = format!("/communities/{id}/members");
    let (_, admin_join) = call(&app, "POST", &path, Some(&admin.token), None).await;
    // ADMIN を付与する API は無いため、DB で昇格させる
    sqlx::query("UPDATE community_members SET role = 'ADMIN', status = 'ACTIVE' WHERE id = ?")
        .bind(admin_join["id"].as_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    call(&app, "POST", &path, Some(&applicant.token), None).await;

    let (status, body) = call(&app, "GET", &path, Some(&admin.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 3);
    let members = body["members"].as_array().unwrap();
    assert_eq!(members.len(), 3);
    assert!(
        members
            .iter()
            .any(|m| m["status"] == "PENDING" && m["accountName"] == "pa-applicant")
    );
}

#[tokio::test]
async fn 承認待ちの申請者はメンバー一覧の先頭に並ぶ() {
    let app = app().await;
    let owner = register(&app, "po-owner").await;
    let applicant = register(&app, "po-applicant").await;
    let id = create_community(&app, &owner, "pending-order", "PRIVATE").await;
    let path = format!("/communities/{id}/members");
    call(&app, "POST", &path, Some(&applicant.token), None).await;

    // 申請はオーナーより後に作られるが、1 件目のページに入る
    let (status, body) = call(
        &app,
        "GET",
        &format!("{path}?limit=1"),
        Some(&owner.token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 2);
    assert_eq!(body["members"][0]["status"], "PENDING");
    assert_eq!(body["members"][0]["accountName"], "po-applicant");
}

#[tokio::test]
async fn 承認待ちの申請者本人には非公開コミュニティのメンバー一覧が404() {
    let app = app().await;
    let owner = register(&app, "ps-owner").await;
    let applicant = register(&app, "ps-applicant").await;
    let id = create_community(&app, &owner, "pending-self", "PRIVATE").await;
    let path = format!("/communities/{id}/members");
    call(&app, "POST", &path, Some(&applicant.token), None).await;

    let (status, _) = call(&app, "GET", &path, Some(&applicant.token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn 一般メンバーのメンバー一覧には承認待ちの申請者が含まれない() {
    let app = app().await;
    let owner = register(&app, "pm-owner").await;
    let member = register(&app, "pm-member").await;
    let applicant = register(&app, "pm-applicant").await;
    let id = create_community(&app, &owner, "pending-member", "PRIVATE").await;
    let path = format!("/communities/{id}/members");
    let (_, joined) = call(&app, "POST", &path, Some(&member.token), None).await;
    let member_id = joined["id"].as_str().unwrap();
    call(
        &app,
        "PATCH",
        &format!("{path}/{member_id}/approve"),
        Some(&owner.token),
        None,
    )
    .await;
    call(&app, "POST", &path, Some(&applicant.token), None).await;

    let (status, body) = call(&app, "GET", &path, Some(&member.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 2);
    let members = body["members"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.iter().all(|m| m["status"] == "ACTIVE"));
}

// ------------------------------------------------------------
// イベント
// ------------------------------------------------------------

fn event_body(starts_in_hours: i64, ends_in_hours: i64) -> Value {
    let now = chrono::Utc::now();
    json!({
        "title": "もくもく会",
        "description": "説明",
        "startsAt": (now + chrono::Duration::hours(starts_in_hours)).to_rfc3339(),
        "endsAt": (now + chrono::Duration::hours(ends_in_hours)).to_rfc3339(),
        "format": "ONLINE",
        "capacity": 50,
    })
}

#[tokio::test]
async fn オーナーはイベントをdraftで作成でき_作成者だけが一覧で見える() {
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

#[tokio::test]
async fn 一般メンバーはイベントを作成できない() {
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
}

#[tokio::test]
async fn 過去の開始日時と終了前後の不整合は422() {
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
}

#[tokio::test]
async fn 非公開コミュニティのイベント一覧は非メンバーに404() {
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

#[tokio::test]
async fn 存在しないコミュニティのイベント一覧は404() {
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

#[tokio::test]
async fn イベントの入力検証は400() {
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
}

// ------------------------------------------------------------
// システム
// ------------------------------------------------------------

#[tokio::test]
async fn ヘルスチェックとopenapiが返る() {
    let app = app().await;
    let (status, body) = call(&app, "GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    let (status, body) = call(&app, "GET", "/openapi.json", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["openapi"], "3.0.3");
}

#[tokio::test]
async fn 未定義パスは404_メソッド不一致は405のjson() {
    let app = app().await;
    let (status, body) = call(&app, "GET", "/nope", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "NOT_FOUND");
    let (status, body) = call(&app, "DELETE", "/health", None, None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(body["code"], "METHOD_NOT_ALLOWED");
}

#[tokio::test]
async fn キャッシュヘッダは公開getのみpublic() {
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
}

#[tokio::test]
async fn レート制限を有効にすると上限超過で429() {
    let pool = connect("sqlite::memory:").await.unwrap();
    let app = create_app(AppState::new(pool, SECRET), true);
    let mut last = StatusCode::OK;
    for _ in 0..61 {
        last = call(&app, "GET", "/communities", None, None).await.0;
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn 承認待ちの申請者は非公開コミュニティを閲覧できない() {
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

#[tokio::test]
async fn 別コミュニティのメンバーidでは脱退できない() {
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

#[tokio::test]
async fn 未登録メールのログインも401で同じ応答になる() {
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

#[tokio::test]
async fn ログインは短時間に繰り返すと429になる() {
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
}

#[tokio::test]
async fn 保護されたエンドポイントは認証なしで401() {
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

#[tokio::test]
async fn 改ざんされたトークンは_保護apiで401_公開getでは匿名扱い() {
    let app = app().await;
    let (status, _) = call(&app, "GET", "/auth/me", Some("not.a.jwt"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = call(&app, "GET", "/communities", Some("not.a.jwt"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn 存在しないリソースは404() {
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
}

#[tokio::test]
async fn 非uuidのパスは400() {
    let app = app().await;
    let (status, body) = call(&app, "GET", "/communities/abc", None, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn ページネーションで件数とtotalが整合する() {
    let app = app().await;
    let owner = register(&app, "pg-owner").await;
    for i in 0..3 {
        create_community(&app, &owner, &format!("pg-{i}"), "PUBLIC").await;
    }
    let (_, body) = call(&app, "GET", "/communities?limit=2&offset=1", None, None).await;
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

#[tokio::test]
async fn コミュニティ作成の入力不備は400() {
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
}

#[tokio::test]
async fn 拒否したメンバーは一覧から消え_再参加できる() {
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

#[tokio::test]
async fn オーナーのメンバーは拒否できない() {
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
}

#[tokio::test]
async fn 長すぎるパスワードは400() {
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
}

#[tokio::test]
async fn adminは承認とイベント作成ができ_pendingと非メンバーは403() {
    let (app, pool) = app_with_pool().await;
    let owner = register(&app, "ad-owner").await;
    let admin = register(&app, "ad-admin").await;
    let pending = register(&app, "ad-pending").await;
    let outsider = register(&app, "ad-out").await;
    let id = create_community(&app, &owner, "admin-role", "PRIVATE").await;
    let members = format!("/communities/{id}/members");

    let (_, admin_join) = call(&app, "POST", &members, Some(&admin.token), None).await;
    let (_, pending_join) = call(&app, "POST", &members, Some(&pending.token), None).await;
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
}

#[tokio::test]
async fn publishedのイベントは未ログインにも見える() {
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

#[tokio::test]
async fn openapiは全ルートを含み_docsはsri付きhtmlを返す() {
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
        String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    assert!(html.contains("integrity=\"sha384-"));
}

#[tokio::test]
async fn メソッド不一致の405はallowヘッダ付き() {
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
}

#[tokio::test]
async fn 巨大なボディは413のjson() {
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
}

#[tokio::test]
async fn 一覧apiは参加で増えた件数とtotalが一致する() {
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
