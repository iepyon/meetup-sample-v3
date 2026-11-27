//! `types.rs` の単体テスト

use super::*;
use meetup_test_macros::describe;

describe!("API の型", {
    test!("コミュニティのレスポンスを解釈できる", {
        let json = r#"{"community":{"id":"c1","name":"n","description":null,"category":"TECH","visibility":"PRIVATE","createdAt":"2026-01-01T00:00:00.000Z","updatedAt":"2026-01-01T00:00:00.000Z"}}"#;
        let res: CommunityResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.community.category, Category::Tech);
        assert_eq!(res.community.visibility, Visibility::Private);
        assert!(res.community.description.is_none());
    });

    test!(
        "メンバー一覧は account_id が無くても解釈できる",
        {
            let json = r#"{"members":[{"id":"m1","communityId":"c","accountName":"太郎","role":"OWNER","status":"ACTIVE","createdAt":"x"}]}"#;
            let res: ListMembersResponse = serde_json::from_str(json).unwrap();
            assert!(res.members[0].account_id.is_none());
            assert_eq!(res.members[0].role, Role::Owner);
        }
    );

    test!("リクエストは API の形式で直列化される", {
        let v = serde_json::to_value(CreateEventRequest {
            title: "t".into(),
            description: "d".into(),
            starts_at: "s".into(),
            ends_at: "e".into(),
            format: EventFormat::Hybrid,
            capacity: 5,
        })
        .unwrap();
        assert_eq!(v["startsAt"], "s");
        assert_eq!(v["endsAt"], "e");
        assert_eq!(v["format"], "HYBRID");
        let c = serde_json::to_value(CreateCommunityRequest {
            name: "n".into(),
            description: "d".into(),
            category: Category::Business,
            visibility: Visibility::Public,
        })
        .unwrap();
        assert_eq!(c["category"], "BUSINESS");
        assert_eq!(c["visibility"], "PUBLIC");
    });

    test!("全ての列挙型が文字列と往復できる", {
        for c in Category::ALL {
            assert_eq!(Category::parse(c.as_str()), Some(*c));
        }
        for v in Visibility::ALL {
            assert_eq!(Visibility::parse(v.as_str()), Some(*v));
        }
        for f in EventFormat::ALL {
            assert_eq!(EventFormat::parse(f.as_str()), Some(*f));
        }
        for r in Role::ALL {
            assert_eq!(Role::parse(r.as_str()), Some(*r));
        }
        assert_eq!(Visibility::parse("x"), None);
    });
});
