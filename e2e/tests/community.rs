//! コミュニティ: 作成 / 詳細 / カテゴリフィルター

use meetup_e2e::REQUIRES_BROWSER;
use meetup_e2e::flows::{CommunityForm, create_community, register};
use meetup_e2e::{E2e, Page, unique_suffix};
use meetup_test_runner::{Node, describe, run, test};

async fn register_user(page: &Page, suffix: &str) {
    register(
        page,
        &format!("コミュニティユーザー{suffix}"),
        &format!("community{suffix}@example.com"),
    )
    .await;
}

fn main() {
    run(vec![コミュニティ().multi_thread().ignore(REQUIRES_BROWSER)]);
}

fn コミュニティ() -> Node {
    describe(
        "コミュニティ",
        vec![
            describe(
                "コミュニティ作成",
                vec![test(
                    "コミュニティを作成すると一覧に表示される",
                    || async {
                        let page = E2e::start().await;
                        let suffix = unique_suffix();
                        register_user(&page, &suffix).await;

                        let name = format!("テストコミュニティ{suffix}");
                        create_community(
                            &page,
                            CommunityForm {
                                name: &name,
                                description: &format!("{name}の説明文です"),
                                category: Some("TECH"),
                                visibility: Some("PUBLIC"),
                            },
                        )
                        .await;

                        page.get_by_role("link")
                            .name("コミュニティ一覧")
                            .click()
                            .await;
                        page.get_by_role("heading")
                            .name(&name)
                            .level(2)
                            .exact()
                            .expect_visible()
                            .await;
                        page.close().await;
                    },
                )],
            ),
            describe(
                "コミュニティ詳細",
                vec![test(
                    "コミュニティ詳細ページを閲覧できる",
                    || async {
                        let page = E2e::start().await;
                        let suffix = unique_suffix();
                        register_user(&page, &suffix).await;

                        let name = format!("詳細テスト{suffix}");
                        create_community(
                            &page,
                            CommunityForm {
                                name: &name,
                                description: "詳細ページのテスト用コミュニティ",
                                category: None,
                                visibility: None,
                            },
                        )
                        .await;

                        page.get_by_text("詳細ページのテスト用コミュニティ")
                            .expect_visible()
                            .await;
                        page.get_by_text("メンバー一覧").expect_visible().await;
                        page.close().await;
                    },
                )],
            ),
            describe(
                "カテゴリフィルター",
                vec![test(
                    "カテゴリフィルターで絞り込みができる",
                    || async {
                        let page = E2e::start().await;
                        let suffix = unique_suffix();
                        register_user(&page, &suffix).await;

                        let tech = format!("テクノロジーコミュニティ{suffix}");
                        let biz = format!("ビジネスコミュニティ{suffix}");
                        for (name, description, category) in [
                            (&tech, "テクノロジーの説明", "TECH"),
                            (&biz, "ビジネスの説明", "BUSINESS"),
                        ] {
                            let form = CommunityForm {
                                name,
                                description,
                                category: Some(category),
                                visibility: None,
                            };
                            create_community(&page, form).await;
                        }

                        page.get_by_role("link")
                            .name("コミュニティ一覧")
                            .click()
                            .await;
                        page.get_by_text(&tech).expect_visible().await;
                        page.get_by_text(&biz).expect_visible().await;

                        let filter = page.get_by_label("カテゴリフィルター");
                        filter.select_option("TECH").await;
                        page.get_by_text(&tech).expect_visible().await;
                        page.get_by_text(&biz).expect_hidden().await;

                        filter.select_option("BUSINESS").await;
                        page.get_by_text(&biz).expect_visible().await;
                        page.get_by_text(&tech).expect_hidden().await;

                        // フィルター解除
                        filter.select_option("").await;
                        page.get_by_text(&tech).expect_visible().await;
                        page.get_by_text(&biz).expect_visible().await;
                        page.close().await;
                    },
                )],
            ),
        ],
    )
}
