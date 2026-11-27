//! 本番構成の SPA 配信（FRONTEND_DIST でビルド済み SPA を同一オリジン配信する構成）
use crate::common::*;
use meetup_test_macros::describe;

const HASHED_ASSET: &str = "app-0123456789abcdef.js";

/// ビルド済み SPA（index.html とハッシュ付きアセット）を一時ディレクトリに置き、同一オリジンで配信するアプリ
async fn spa_app() -> Router {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("meetup-spa-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<!doctype html><title>spa</title>").unwrap();
    std::fs::write(dir.join(HASHED_ASSET), "console.log(1)").unwrap();
    let pool = connect("sqlite::memory:").await.unwrap();
    meetup_backend::create_app_with_frontend(
        AppState::new(pool, SECRET),
        false,
        Some(dir.to_string_lossy().into_owned()),
    )
}

describe!("本番構成の SPA 配信", {
    test!(
        "SPA のページ遷移は index.html を返し、セキュリティヘッダが付く",
        async {
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
    );

    test!(
        "SPA 構成でも JSON の API 呼び出しはそのまま通る",
        async {
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
    );

    test!(
        "ハッシュ付きの静的アセットは長期キャッシュされる",
        async {
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
    );
});
