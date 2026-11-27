//! `shared/validation.rs` の単体テスト

use super::*;
use meetup_test_macros::describe;

fn obj(v: serde_json::Value) -> JsonObject {
    v.as_object().unwrap().clone()
}

describe!("入力検証", {
    test!("メールアドレスの形式を検証できる", {
        assert!(is_valid_email("a@example.com"));
        assert!(!is_valid_email("a@example"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("a b@example.com"));
        assert!(!is_valid_email("plain"));
    });

    test!("UUID 形式を検証できる", {
        assert!(uuid_param("id", "550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(uuid_param("id", "not-a-uuid").is_err());
        assert!(uuid_param("id", "me").is_err());
    });

    test!("ページ指定の範囲外は 400 になる", {
        let mut q = HashMap::new();
        q.insert("limit".to_string(), "101".to_string());
        assert!(page_params(&q).is_err());
        q.insert("limit".to_string(), "10".to_string());
        assert_eq!(page_params(&q).unwrap(), (10, 0));
    });

    test!("文字列の長さは文字数で境界を検証する", {
        let ok = obj(serde_json::json!({ "n": "あ".repeat(100) }));
        assert!(required_string(&ok, "n", 1, 100).is_ok());
        let long = obj(serde_json::json!({ "n": "あ".repeat(101) }));
        assert!(required_string(&long, "n", 1, 100).is_err());
        let empty = obj(serde_json::json!({ "n": "" }));
        assert!(required_string(&empty, "n", 1, 100).is_err());
        assert!(required_string(&obj(serde_json::json!({})), "n", 1, 100).is_err());
        assert!(required_string(&obj(serde_json::json!({ "n": 1 })), "n", 1, 100).is_err());
    });

    test!(
        "nullable 文字列は省略と null を許し、上限を検証する",
        {
            assert_eq!(
                optional_nullable_string(&obj(serde_json::json!({})), "d", 5).unwrap(),
                None
            );
            assert_eq!(
                optional_nullable_string(&obj(serde_json::json!({ "d": null })), "d", 5).unwrap(),
                None
            );
            assert!(
                optional_nullable_string(&obj(serde_json::json!({ "d": "123456" })), "d", 5)
                    .is_err()
            );
            assert!(optional_nullable_string(&obj(serde_json::json!({ "d": 1 })), "d", 5).is_err());
        }
    );

    test!(
        "列挙値は許可された文字列だけを受け付ける",
        {
            let allowed = ["A", "B"];
            assert_eq!(
                required_enum(&obj(serde_json::json!({ "e": "A" })), "e", &allowed).unwrap(),
                "A"
            );
            assert!(required_enum(&obj(serde_json::json!({ "e": "C" })), "e", &allowed).is_err());
            assert!(required_enum(&obj(serde_json::json!({})), "e", &allowed).is_err());
        }
    );

    test!("整数は範囲と型を検証する", {
        assert_eq!(
            required_int(&obj(serde_json::json!({ "c": 1 })), "c", 1, 1000).unwrap(),
            1
        );
        assert_eq!(
            required_int(&obj(serde_json::json!({ "c": 1.0 })), "c", 1, 1000).unwrap(),
            1
        );
        for bad in [
            serde_json::json!({ "c": 0 }),
            serde_json::json!({ "c": 1001 }),
            serde_json::json!({ "c": 1.5 }),
            serde_json::json!({ "c": "1" }),
            serde_json::json!({}),
        ] {
            assert!(
                required_int(&obj(bad.clone()), "c", 1, 1000).is_err(),
                "{bad}"
            );
        }
    });

    test!("日時は RFC 3339 のみ受け付ける", {
        assert!(
            required_datetime(
                &obj(serde_json::json!({ "t": "2030-01-01T10:00:00.000Z" })),
                "t"
            )
            .is_ok()
        );
        assert!(
            required_datetime(
                &obj(serde_json::json!({ "t": "2030-01-01T10:00:00+09:00" })),
                "t"
            )
            .is_ok()
        );
        assert!(required_datetime(&obj(serde_json::json!({ "t": "2030-01-01" })), "t").is_err());
        assert!(required_datetime(&obj(serde_json::json!({ "t": "nope" })), "t").is_err());
    });

    test!("JSON ボディはオブジェクトのみ受け付ける", {
        assert!(json_object(&Bytes::from_static(b"{}")).is_ok());
        for bad in [&b"[]"[..], b"null", b"", b"{oops"] {
            assert!(json_object(&Bytes::copy_from_slice(bad)).is_err());
        }
    });

    test!("パスワードは bcrypt の上限バイト数まで", {
        assert!(required_password(&obj(serde_json::json!({ "p": "a".repeat(72) })), "p").is_ok());
        assert!(required_password(&obj(serde_json::json!({ "p": "a".repeat(73) })), "p").is_err());
        // 日本語は 1 文字 3 バイトなので 25 文字で 75 バイト
        assert!(required_password(&obj(serde_json::json!({ "p": "あ".repeat(25) })), "p").is_err());
    });

    test!("新規登録のパスワードは 8 文字以上", {
        let p = |v: &str| obj(serde_json::json!({ "p": v }));
        assert!(required_new_password(&p("1234567"), "p").is_err());
        assert!(required_new_password(&p("12345678"), "p").is_ok());
        // ログインでは最小長を課さない（既存アカウントを締め出さない）
        assert!(required_password(&p("1"), "p").is_ok());
    });

    test!(
        "メールアドレスは小文字にそろえ 254 文字までに制限する",
        {
            let e = |v: &str| obj(serde_json::json!({ "e": v }));
            assert_eq!(
                required_email(&e("User@Example.COM"), "e").unwrap(),
                "user@example.com"
            );
            let long = format!("{}@example.com", "a".repeat(EMAIL_MAX_CHARS));
            assert!(required_email(&e(&long), "e").is_err());
        }
    );

    test!("ページ指定の不正値と境界", {
        let q = |k: &str, v: &str| HashMap::from([(k.to_string(), v.to_string())]);
        assert!(page_params(&q("limit", "-1")).is_err());
        assert!(page_params(&q("limit", "abc")).is_err());
        assert!(page_params(&q("offset", "-1")).is_err());
        assert_eq!(page_params(&q("limit", "100")).unwrap(), (100, 0));
        assert_eq!(page_params(&q("limit", "0")).unwrap(), (0, 0));
    });
});
