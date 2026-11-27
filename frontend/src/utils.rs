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

/// UUID（8-4-4-4-12 の 16 進）か。URL の `:id` を API パスへ埋め込む前に検証する
/// （デコード済みの `../` などで別の API パスへリクエストを向けられないように）。
pub fn is_uuid(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(p, len)| p.len() == len && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// 新規登録時のパスワードの最小文字数（backend の `PASSWORD_MIN_CHARS` と揃える）
pub const PASSWORD_MIN_CHARS: usize = 8;

/// 新規登録のパスワードを送信前に検査する
pub fn validate_new_password(password: &str) -> Result<(), &'static str> {
    if password.chars().count() < PASSWORD_MIN_CHARS {
        return Err("パスワードは8文字以上で入力してください");
    }
    Ok(())
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
mod tests;
