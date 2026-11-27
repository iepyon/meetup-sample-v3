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

    #[test]
    fn 列挙型は文字列と相互変換できる() {
        assert_eq!(Category::parse("TECH"), Some(Category::Tech));
        assert_eq!(Category::parse(""), None);
        assert_eq!(Visibility::Private.as_str(), "PRIVATE");
    }
}
