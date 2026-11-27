//! メンバー: 参加申請の審査 / コミュニティ参加 / コミュニティ退会

use meetup_e2e::REQUIRES_BROWSER;
use meetup_e2e::flows::{
    CommunityForm, apply_via_api, create_community, logout, open_community_from, register,
};
use meetup_e2e::{E2e, Locator, Page, unique_suffix};
use meetup_test_runner::{Node, describe, run, test};

/// オーナーとして登録し、非公開コミュニティを作成して URL からコミュニティ ID を返す
async fn setup_private_community(page: &Page, suffix: &str, prefix: &str, name: &str) -> String {
    register(
        page,
        &format!("メンバーユーザー{suffix}"),
        &format!("{prefix}{suffix}@example.com"),
    )
    .await;
    create_community(
        page,
        CommunityForm {
            name,
            description: &format!("{name}の説明"),
            category: None,
            visibility: Some("PRIVATE"),
        },
    )
    .await;
    let path = page.path().await;
    path.rsplit('/').next().unwrap_or_default().to_string()
}

/// メンバー一覧のうち、指定した名前のメンバーのカード
fn member_card<'p>(page: &'p Page, name: &str) -> Locator<'p> {
    page.locator("div.rounded-lg")
        .filter_has(page.get_by_text(name).exact())
}

/// オーナーが非公開コミュニティを作り、別ユーザーが API で参加申請した状態にする。
/// オーナーの画面で申請者のカードが PENDING で表示されるまで進める。
/// `label` はコミュニティ名・表示名に、`prefix` はメールアドレスの接頭辞に使う
async fn setup_pending_application(page: &Page, label: &str, prefix: &str, suffix: &str) -> String {
    let community = format!("{label}テストコミュニティ{suffix}");
    let applicant = format!("{label}申請者{suffix}");
    let id = setup_private_community(page, suffix, &format!("{prefix}-owner"), &community).await;
    let email = format!("{prefix}-applicant{suffix}@example.com");
    apply_via_api(page, &id, &applicant, &email).await;

    // マイコミュニティ経由で詳細を開き直し、メンバー一覧を再取得させる
    open_community_from(page, "マイコミュニティ", &community).await;
    member_card(page, &applicant)
        .get_by_text("MEMBER / PENDING")
        .exact()
        .expect_visible()
        .await;
    applicant
}

/// オーナーが公開コミュニティを作り、別ユーザーで登録し直してその詳細ページを開く。
/// `owner` / `member` はメールアドレスの接頭辞、`label` はコミュニティ名・表示名に使う
async fn open_as_another_user(page: &Page, suffix: &str, owner: &str, member: &str, label: &str) {
    let community = format!("{label}テストコミュニティ{suffix}");
    register(
        page,
        &format!("オーナー{owner}{suffix}"),
        &format!("{owner}{suffix}@example.com"),
    )
    .await;
    let form = CommunityForm {
        name: &community,
        description: &format!("{community}の説明"),
        category: None,
        visibility: None,
    };
    create_community(page, form).await;
    logout(page).await;

    register(
        page,
        &format!("{label}者{suffix}"),
        &format!("{member}{suffix}@example.com"),
    )
    .await;
    open_community_from(page, "コミュニティ一覧", &community).await;
}

fn main() {
    run(vec![メンバー().multi_thread().ignore(REQUIRES_BROWSER)]);
}

fn メンバー() -> Node {
    describe(
        "メンバー",
        vec![
            describe(
                "参加申請の審査",
                vec![
                    test(
                        "オーナーは非公開コミュニティの参加申請を承認できる",
                        || async {
                            let page = E2e::start().await;
                            let applicant = setup_pending_application(
                                &page,
                                "承認",
                                "approve",
                                &unique_suffix(),
                            )
                            .await;
                            let card = member_card(&page, &applicant);

                            card.get_by_role("button")
                                .name("承認")
                                .exact()
                                .click()
                                .await;

                            card.get_by_text("MEMBER / ACTIVE")
                                .exact()
                                .expect_visible()
                                .await;
                            card.get_by_role("button").expect_count(0).await;
                            page.close().await;
                        },
                    ),
                    test(
                        "オーナーは非公開コミュニティの参加申請を拒否できる",
                        || async {
                            let page = E2e::start().await;
                            let applicant = setup_pending_application(
                                &page,
                                "拒否",
                                "reject",
                                &unique_suffix(),
                            )
                            .await;
                            let card = member_card(&page, &applicant);

                            card.get_by_role("button")
                                .name("拒否")
                                .exact()
                                .click()
                                .await;

                            card.expect_count(0).await;
                            page.close().await;
                        },
                    ),
                ],
            ),
            describe(
                "コミュニティ参加",
                vec![test(
                    "コミュニティに参加するとメンバーとして表示される",
                    || async {
                        let page = E2e::start().await;
                        open_as_another_user(&page, &unique_suffix(), "owner", "joiner", "参加")
                            .await;

                        page.get_by_role("button")
                            .name("参加する")
                            .expect_visible()
                            .await;
                        page.get_by_role("button").name("参加する").click().await;

                        // 参加済みになると退会ボタンが表示される
                        page.get_by_role("button")
                            .name("退会する")
                            .expect_visible()
                            .await;
                        page.close().await;
                    },
                )],
            ),
            describe(
                "コミュニティ退会",
                vec![test("コミュニティから退会できる", || async {
                    let page = E2e::start().await;
                    open_as_another_user(&page, &unique_suffix(), "owner2", "leaver", "退会").await;
                    page.get_by_role("button").name("参加する").click().await;
                    page.get_by_role("button")
                        .name("退会する")
                        .expect_visible()
                        .await;

                    page.get_by_role("button").name("退会する").click().await;

                    // 参加ボタンが再表示される
                    page.get_by_role("button")
                        .name("参加する")
                        .expect_visible()
                        .await;
                    page.close().await;
                })],
            ),
        ],
    )
}
