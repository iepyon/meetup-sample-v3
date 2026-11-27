//! イベント: 作成 / 入力エラー / 権限チェック

use meetup_e2e::REQUIRES_BROWSER;
use meetup_e2e::flows::{
    CommunityForm, create_community, local_datetime_from_today, logout, register,
};
use meetup_e2e::{E2e, Page, unique_suffix};
use meetup_test_runner::{Node, describe, run, test};

/// オーナーとして登録し、公開コミュニティを作成してその名前を返す
async fn setup_owner_community(page: &Page, suffix: &str) -> String {
    register(
        page,
        &format!("イベントユーザー{suffix}"),
        &format!("event{suffix}@example.com"),
    )
    .await;
    let name = format!("イベントテスト{suffix}");
    create_community(
        page,
        CommunityForm {
            name: &name,
            description: &format!("{name}の説明文です"),
            category: Some("TECH"),
            visibility: Some("PUBLIC"),
        },
    )
    .await;
    name
}

async fn open_event_form(page: &Page) {
    page.get_by_role("button")
        .name("イベント作成")
        .click()
        .await;
    page.get_by_role("heading")
        .name("イベント作成")
        .expect_visible()
        .await;
}

fn main() {
    run(vec![イベント().multi_thread().ignore(REQUIRES_BROWSER)]);
}

fn イベント() -> Node {
    describe(
        "イベント",
        vec![
            describe(
                "イベント作成",
                vec![test(
                    "オーナーがイベントを作成するとコミュニティ詳細ページに遷移する",
                    || async {
                        let page = E2e::start().await;
                        let community = setup_owner_community(&page, &unique_suffix()).await;
                        open_event_form(&page).await;

                        page.get_by_label("タイトル").fill("Rust もくもく会").await;
                        page.locator("textarea#event-description")
                            .fill("Rustでもくもくプログラミングする会")
                            .await;
                        page.get_by_label("開始日時")
                            .fill(&local_datetime_from_today(30, 19))
                            .await;
                        page.get_by_label("終了日時")
                            .fill(&local_datetime_from_today(30, 21))
                            .await;
                        page.locator("select#event-format")
                            .select_option("ONLINE")
                            .await;
                        page.get_by_label("定員").fill("50").await;
                        page.get_by_role("button").name("作成").click().await;

                        page.get_by_role("heading")
                            .name(&community)
                            .expect_visible()
                            .await;
                        page.close().await;
                    },
                )],
            ),
            describe(
                "入力エラー",
                vec![test(
                    "開始日時が過去だと作成できず理由が表示される",
                    || async {
                        let page = E2e::start().await;
                        setup_owner_community(&page, &unique_suffix()).await;
                        open_event_form(&page).await;

                        page.get_by_label("タイトル").fill("過去のイベント").await;
                        page.get_by_label("開始日時")
                            .fill(&local_datetime_from_today(-1, 19))
                            .await;
                        page.get_by_label("終了日時")
                            .fill(&local_datetime_from_today(-1, 21))
                            .await;
                        page.get_by_role("button").name("作成").click().await;

                        page.get_by_role("alert")
                            .expect_text("開始日時には現在より後の日時を指定してください")
                            .await;
                        page.get_by_role("heading")
                            .name("イベント作成")
                            .expect_visible()
                            .await;
                        page.close().await;
                    },
                )],
            ),
            describe(
                "権限チェック",
                vec![test(
                    "一般メンバーにはイベント作成ボタンが表示されない",
                    || async {
                        let page = E2e::start().await;
                        let suffix = unique_suffix();
                        let community = setup_owner_community(&page, &suffix).await;
                        logout(&page).await;

                        // 別ユーザーで登録し、一覧からコミュニティを開いて参加する
                        register(
                            &page,
                            &format!("メンバーmember{suffix}"),
                            &format!("event-member{suffix}@example.com"),
                        )
                        .await;
                        page.get_by_role("heading")
                            .name(&community)
                            .level(2)
                            .exact()
                            .click()
                            .await;
                        page.get_by_role("heading")
                            .name(&community)
                            .level(1)
                            .exact()
                            .expect_visible()
                            .await;
                        page.get_by_role("button").name("参加する").click().await;

                        // 否定のアサーションの前に、参加の完了（退会ボタン）を待つ
                        page.get_by_role("button")
                            .name("退会する")
                            .expect_visible()
                            .await;
                        page.get_by_role("button")
                            .name("イベント作成")
                            .expect_hidden()
                            .await;
                        page.close().await;
                    },
                )],
            ),
        ],
    )
}
