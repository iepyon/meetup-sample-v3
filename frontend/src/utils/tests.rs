//! `utils.rs` の単体テスト

use super::*;
use meetup_test_macros::describe;

fn community(name: &str, category: Category) -> Community {
    Community {
        id: name.into(),
        name: name.into(),
        description: None,
        category,
        visibility: Visibility::Public,
    }
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

describe!("画面用の純粋関数", {
    describe!("表示ラベル", {
        test!("カテゴリのラベルを返す", {
            assert_eq!(category_label(Category::Tech), "テクノロジー");
            assert_eq!(category_label(Category::Business), "ビジネス");
            assert_eq!(category_label(Category::Hobby), "趣味");
        });

        test!("公開設定と開催形式のラベルを返す", {
            assert_eq!(visibility_label(Visibility::Public), "公開");
            assert_eq!(visibility_label(Visibility::Private), "非公開");
            assert_eq!(format_label(EventFormat::Hybrid), "ハイブリッド");
        });
    });

    describe!("絞り込み", {
        test!("カテゴリで絞り込める", {
            let list = vec![
                community("a", Category::Tech),
                community("b", Category::Hobby),
            ];
            assert_eq!(filter_by_category(&list, Some(Category::Tech)).len(), 1);
            assert_eq!(filter_by_category(&list, None).len(), 2);
        });
    });

    describe!("イベント入力", {
        test!("定員の入力を整数に変換する", {
            assert_eq!(parse_capacity("50"), 50);
            assert_eq!(parse_capacity(" 7 "), 7);
            assert_eq!(parse_capacity("abc"), 0);
        });

        test!("定員の入力のエッジケース", {
            assert_eq!(parse_capacity(""), 0);
            assert_eq!(parse_capacity("-3"), -3);
            assert_eq!(parse_capacity("1.5"), 0);
            assert_eq!(parse_capacity("99999999999999999999"), 0);
        });

        test!("正しいイベント入力は通る", {
            for capacity in [1, 1000] {
                let input = EventFormInput {
                    capacity,
                    ..valid_event_form()
                };
                assert!(validate_event_form(&input).is_ok());
            }
        });

        test!("タイトルが空白だけなら入力を促す", {
            let input = EventFormInput {
                title: "  ",
                ..valid_event_form()
            };
            assert_eq!(event_form_error(input), "タイトルを入力してください");
        });

        test!("日時が未入力なら入力を促す", {
            let input = EventFormInput {
                ends_at_ms: None,
                ..valid_event_form()
            };
            assert_eq!(
                event_form_error(input),
                "開始日時と終了日時を入力してください"
            );
        });

        test!(
            "開始日時が現在以前なら未来の日時を求める",
            {
                let input = EventFormInput {
                    starts_at_ms: Some(1_000.0),
                    ..valid_event_form()
                };
                assert_eq!(
                    event_form_error(input),
                    "開始日時には現在より後の日時を指定してください"
                );
            }
        );

        test!(
            "終了日時が開始日時以前なら後の日時を求める",
            {
                let input = EventFormInput {
                    ends_at_ms: Some(2_000.0),
                    ..valid_event_form()
                };
                assert_eq!(
                    event_form_error(input),
                    "終了日時には開始日時より後の日時を指定してください"
                );
            }
        );

        test!("定員が範囲外なら範囲を示す", {
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
        });
    });

    describe!("エラーメッセージ", {
        test!(
            "業務エラーはサーバーの日本語メッセージを表示する",
            {
                assert_eq!(
                    api_error_message(422, "終了日時は開始日時より後でなければなりません", "失敗"),
                    "終了日時は開始日時より後でなければなりません"
                );
                assert_eq!(
                    api_error_message(409, "既にメンバーです", "失敗"),
                    "既にメンバーです"
                );
            }
        );

        test!(
            "開発者向けのエラー文言は画面に出さない",
            {
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
        );

        test!(
            "認証と権限のエラーは対処が分かる文言にする",
            {
                assert_eq!(
                    api_error_message(401, "Authentication required", "失敗"),
                    "ログインの有効期限が切れました。もう一度ログインしてください"
                );
                assert_eq!(
                    api_error_message(403, "", "失敗"),
                    "この操作を行う権限がありません"
                );
            }
        );
    });

    describe!("入力形式", {
        test!("UUID 形式だけを受け付ける", {
            assert!(is_uuid("0f8fad5b-d9cb-469f-a165-70867728950e"));
            assert!(!is_uuid(""));
            assert!(!is_uuid("x/../../auth/me?"));
            assert!(!is_uuid("0f8fad5b-d9cb-469f-a165-70867728950"));
            assert!(!is_uuid("0f8fad5b-d9cb-469f-a165-70867728950g"));
        });

        test!(
            "新規登録のパスワードは 8 文字以上を求める",
            {
                assert_eq!(
                    validate_new_password("1234567"),
                    Err("パスワードは8文字以上で入力してください")
                );
                assert!(validate_new_password("12345678").is_ok());
            }
        );

        test!("列挙型は文字列と相互変換できる", {
            assert_eq!(Category::parse("TECH"), Some(Category::Tech));
            assert_eq!(Category::parse(""), None);
            assert_eq!(Visibility::Private.as_str(), "PRIVATE");
        });
    });
});
