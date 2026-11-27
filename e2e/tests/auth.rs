//! 認証: 新規登録 / ログイン / ログアウト

use meetup_e2e::flows::{PASSWORD, logout, register};
use meetup_e2e::{E2e, unique_suffix};
use meetup_test_macros::describe;

describe!("認証", {
    describe!("新規登録", {
        #[tokio::test(flavor = "multi_thread")]
        #[ignore = "E2E: Chromium と trunk build 済みの SPA が必要（d e2e で実行）"]
        test!(
            "新規アカウントを登録するとホームにリダイレクトされる",
            async {
                let page = E2e::start().await;
                let suffix = unique_suffix();
                page.goto("/register").await;

                page.get_by_role("heading")
                    .name("新規登録")
                    .expect_visible()
                    .await;

                page.get_by_label("名前")
                    .fill(&format!("テストユーザー{suffix}"))
                    .await;
                page.get_by_label("メールアドレス")
                    .fill(&format!("test{suffix}@example.com"))
                    .await;
                page.get_by_label("パスワード").fill(PASSWORD).await;
                page.get_by_role("button").name("登録").click().await;

                page.expect_path("/").await;
                page.close().await;
            }
        );
    });

    describe!("ログイン", {
        #[tokio::test(flavor = "multi_thread")]
        #[ignore = "E2E: Chromium と trunk build 済みの SPA が必要（d e2e で実行）"]
        test!(
            "登録済みアカウントでログインするとヘッダーにユーザー名が表示される",
            async {
                let page = E2e::start().await;
                let suffix = unique_suffix();
                let name = format!("ログインユーザー{suffix}");
                let email = format!("login{suffix}@example.com");

                register(&page, &name, &email).await;
                logout(&page).await;

                page.goto("/login").await;
                page.get_by_role("heading")
                    .name("ログイン")
                    .expect_visible()
                    .await;
                page.get_by_label("メールアドレス").fill(&email).await;
                page.get_by_label("パスワード").fill(PASSWORD).await;
                page.get_by_role("button").name("ログイン").click().await;

                page.expect_path("/").await;
                page.get_by_text(&name).expect_visible().await;
                page.close().await;
            }
        );

        #[tokio::test(flavor = "multi_thread")]
        #[ignore = "E2E: Chromium と trunk build 済みの SPA が必要（d e2e で実行）"]
        test!(
            "誤った認証情報ではログインに失敗しユーザーは認証されない",
            async {
                let page = E2e::start().await;
                page.goto("/login").await;

                page.get_by_label("メールアドレス")
                    .fill("nonexistent@example.com")
                    .await;
                page.get_by_label("パスワード").fill("wrongpassword").await;
                page.get_by_role("button").name("ログイン").click().await;

                // 否定のアサーションは即座に通ってしまうため、先にエラー表示を待つ
                page.get_by_role("alert").expect_visible().await;
                page.expect_path("/login").await;
                page.get_by_role("button")
                    .name("ログアウト")
                    .expect_hidden()
                    .await;
                page.close().await;
            }
        );
    });

    describe!("ログアウト", {
        #[tokio::test(flavor = "multi_thread")]
        #[ignore = "E2E: Chromium と trunk build 済みの SPA が必要（d e2e で実行）"]
        test!(
            "ログアウトするとログインページにリダイレクトされる",
            async {
                let page = E2e::start().await;
                let suffix = unique_suffix();
                register(
                    &page,
                    &format!("ログアウトユーザー{suffix}"),
                    &format!("logout{suffix}@example.com"),
                )
                .await;

                page.get_by_role("button").name("ログアウト").click().await;

                // 未ログイン状態のヘッダー（ログイン・新規登録リンク）になる
                page.get_by_role("link")
                    .name("ログイン")
                    .expect_visible()
                    .await;
                page.get_by_role("link")
                    .name("新規登録")
                    .expect_visible()
                    .await;
                page.close().await;
            }
        );
    });
});
