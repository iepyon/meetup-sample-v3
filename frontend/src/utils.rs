//! 純粋関数（ラベル変換・絞り込み・入力変換）。ブラウザ API に依存せずネイティブでテストできる。

use crate::types::{Category, Community, EventFormat, Visibility};

pub fn category_label(category: Category) -> &'static str {
    match category {
        Category::Tech => "テクノロジー",
        Category::Business => "ビジネス",
        Category::Hobby => "趣味",
    }
}

pub fn visibility_label(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Public => "公開",
        Visibility::Private => "非公開",
    }
}

pub fn format_label(format: EventFormat) -> &'static str {
    match format {
        EventFormat::Online => "オンライン",
        EventFormat::Offline => "オフライン",
        EventFormat::Hybrid => "ハイブリッド",
    }
}

/// カテゴリで絞り込む（`None` は全件）
pub fn filter_by_category(communities: &[Community], category: Option<Category>) -> Vec<Community> {
    match category {
        Some(c) => communities
            .iter()
            .filter(|x| x.category == c)
            .cloned()
            .collect(),
        None => communities.to_vec(),
    }
}

/// 定員入力を整数にする。数値でなければ 0（フォームのバリデーションで弾く）
pub fn parse_capacity(input: &str) -> i64 {
    input.trim().parse().unwrap_or(0)
}

/// イベント作成フォームの入力。日時は UNIX エポックからのミリ秒（未入力・不正なら `None`）
pub struct EventFormInput<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub starts_at_ms: Option<f64>,
    pub ends_at_ms: Option<f64>,
    pub capacity: i64,
    pub now_ms: f64,
}

/// 送信前の入力チェック。サーバーの制約（`validation.rs` / `event::models`）と同じ条件を、
/// 利用者が直し方の分かる日本語で返す
pub fn validate_event_form(input: &EventFormInput) -> Result<(), &'static str> {
    if input.title.trim().is_empty() {
        return Err("タイトルを入力してください");
    }
    if input.title.chars().count() > 100 {
        return Err("タイトルは100文字以内で入力してください");
    }
    if input.description.chars().count() > 1000 {
        return Err("説明は1000文字以内で入力してください");
    }
    let (Some(starts), Some(ends)) = (input.starts_at_ms, input.ends_at_ms) else {
        return Err("開始日時と終了日時を入力してください");
    };
    if starts <= input.now_ms {
        return Err("開始日時には現在より後の日時を指定してください");
    }
    if ends <= starts {
        return Err("終了日時には開始日時より後の日時を指定してください");
    }
    if !(1..=1000).contains(&input.capacity) {
        return Err("定員は1〜1000の整数で入力してください");
    }
    Ok(())
}

/// API の失敗を利用者向けの文言にする。409 / 422 / 429 はサーバーの日本語メッセージを使い、
/// 開発者向け（英語の検証メッセージや内部エラー）の文言は画面に出さない
pub fn api_error_message(status: u16, server_message: &str, fallback: &str) -> String {
    let message = match status {
        0 | 409 | 422 | 429 if !server_message.is_empty() => server_message,
        400 => "入力内容に誤りがあります。各項目を確認してください",
        401 => "ログインの有効期限が切れました。もう一度ログインしてください",
        403 => "この操作を行う権限がありません",
        _ => fallback,
    };
    message.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn community(name: &str, category: Category) -> Community {
        Community {
            id: name.into(),
            name: name.into(),
            description: None,
            category,
            visibility: Visibility::Public,
        }
    }

    #[test]
    fn カテゴリのラベルを返す() {
        assert_eq!(category_label(Category::Tech), "テクノロジー");
        assert_eq!(category_label(Category::Business), "ビジネス");
        assert_eq!(category_label(Category::Hobby), "趣味");
    }

    #[test]
    fn 公開設定と開催形式のラベルを返す() {
        assert_eq!(visibility_label(Visibility::Public), "公開");
        assert_eq!(visibility_label(Visibility::Private), "非公開");
        assert_eq!(format_label(EventFormat::Hybrid), "ハイブリッド");
    }

    #[test]
    fn カテゴリで絞り込める() {
        let list = vec![
            community("a", Category::Tech),
            community("b", Category::Hobby),
        ];
        assert_eq!(filter_by_category(&list, Some(Category::Tech)).len(), 1);
        assert_eq!(filter_by_category(&list, None).len(), 2);
    }

    #[test]
    fn 定員の入力を整数に変換する() {
        assert_eq!(parse_capacity("50"), 50);
        assert_eq!(parse_capacity(" 7 "), 7);
        assert_eq!(parse_capacity("abc"), 0);
    }

    #[test]
    fn 定員の入力のエッジケース() {
        assert_eq!(parse_capacity(""), 0);
        assert_eq!(parse_capacity("-3"), -3);
        assert_eq!(parse_capacity("1.5"), 0);
        assert_eq!(parse_capacity("99999999999999999999"), 0);
    }

    fn event_form_error(input: EventFormInput) -> &'static str {
        validate_event_form(&input).err().unwrap_or("")
    }

    fn valid_event_form<'a>() -> EventFormInput<'a> {
        EventFormInput {
            title: "もくもく会",
            description: "",
            starts_at_ms: Some(2_000.0),
            ends_at_ms: Some(3_000.0),
            capacity: 50,
            now_ms: 1_000.0,
        }
    }

    #[test]
    fn 正しいイベント入力は通る() {
        for capacity in [1, 1000] {
            let input = EventFormInput {
                capacity,
                ..valid_event_form()
            };
            assert!(validate_event_form(&input).is_ok());
        }
    }

    #[test]
    fn タイトルが空白だけなら入力を促す() {
        let input = EventFormInput {
            title: "  ",
            ..valid_event_form()
        };
        assert_eq!(event_form_error(input), "タイトルを入力してください");
    }

    #[test]
    fn 日時が未入力なら入力を促す() {
        let input = EventFormInput {
            ends_at_ms: None,
            ..valid_event_form()
        };
        assert_eq!(
            event_form_error(input),
            "開始日時と終了日時を入力してください"
        );
    }

    #[test]
    fn 開始日時が現在以前なら未来の日時を求める() {
        let input = EventFormInput {
            starts_at_ms: Some(1_000.0),
            ..valid_event_form()
        };
        assert_eq!(
            event_form_error(input),
            "開始日時には現在より後の日時を指定してください"
        );
    }

    #[test]
    fn 終了日時が開始日時以前なら後の日時を求める() {
        let input = EventFormInput {
            ends_at_ms: Some(2_000.0),
            ..valid_event_form()
        };
        assert_eq!(
            event_form_error(input),
            "終了日時には開始日時より後の日時を指定してください"
        );
    }

    #[test]
    fn 定員が範囲外なら範囲を示す() {
        for capacity in [0, 1001] {
            let input = EventFormInput {
                capacity,
                ..valid_event_form()
            };
            assert_eq!(
                event_form_error(input),
                "定員は1〜1000の整数で入力してください"
            );
        }
    }

    #[test]
    fn 業務エラーはサーバーの日本語メッセージを表示する() {
        assert_eq!(
            api_error_message(422, "終了日時は開始日時より後でなければなりません", "失敗"),
            "終了日時は開始日時より後でなければなりません"
        );
        assert_eq!(
            api_error_message(409, "既にメンバーです", "失敗"),
            "既にメンバーです"
        );
    }

    #[test]
    fn 開発者向けのエラー文言は画面に出さない() {
        assert_eq!(
            api_error_message(400, "request/body/capacity must be >= 1", "失敗"),
            "入力内容に誤りがあります。各項目を確認してください"
        );
        assert_eq!(
            api_error_message(500, "Internal server error", "失敗"),
            "失敗"
        );
        assert_eq!(api_error_message(422, "", "失敗"), "失敗");
    }

    #[test]
    fn 認証と権限のエラーは対処が分かる文言にする() {
        assert_eq!(
            api_error_message(401, "Authentication required", "失敗"),
            "ログインの有効期限が切れました。もう一度ログインしてください"
        );
        assert_eq!(
            api_error_message(403, "", "失敗"),
            "この操作を行う権限がありません"
        );
    }

    #[test]
    fn 列挙型は文字列と相互変換できる() {
        assert_eq!(Category::parse("TECH"), Some(Category::Tech));
        assert_eq!(Category::parse(""), None);
        assert_eq!(Visibility::Private.as_str(), "PRIVATE");
    }
}
