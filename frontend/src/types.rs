//! API の DTO。サーバーのレスポンス形式（camelCase）に対応する。

use serde::{Deserialize, Serialize};

// ============================================================
// 列挙型（スキーマ由来）
// ============================================================

macro_rules! api_enum {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $(#[serde(rename = $text)] $variant),+
        }

        #[allow(dead_code)]
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }

            pub fn parse(value: &str) -> Option<Self> {
                match value { $($text => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}

api_enum!(Category { Tech => "TECH", Business => "BUSINESS", Hobby => "HOBBY" });
api_enum!(Visibility { Public => "PUBLIC", Private => "PRIVATE" });
api_enum!(Role { Owner => "OWNER", Admin => "ADMIN", Member => "MEMBER" });
api_enum!(MemberStatus { Pending => "PENDING", Active => "ACTIVE" });
api_enum!(EventFormat { Online => "ONLINE", Offline => "OFFLINE", Hybrid => "HYBRID" });
api_enum!(EventStatus { Draft => "DRAFT", Published => "PUBLISHED", Closed => "CLOSED", Cancelled => "CANCELLED" });

// ============================================================
// Auth
// ============================================================

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AccountResponse {
    pub account: Account,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub account: Account,
    pub token: String,
}

// ============================================================
// Community
// ============================================================

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Community {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Category,
    pub visibility: Visibility,
}

#[derive(Debug, Deserialize)]
pub struct CommunityResponse {
    pub community: Community,
}

#[derive(Debug, Deserialize)]
pub struct ListCommunitiesResponse {
    pub communities: Vec<Community>,
}

#[derive(Debug, Serialize)]
pub struct CreateCommunityRequest {
    pub name: String,
    pub description: String,
    pub category: Category,
    pub visibility: Visibility,
}

// ============================================================
// Member
// ============================================================

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CommunityMember {
    pub id: String,
    /// ACTIVE メンバー自身が閲覧している場合のみ返却される（列挙防止）
    #[serde(rename = "accountId")]
    pub account_id: Option<String>,
    #[serde(rename = "accountName")]
    pub account_name: Option<String>,
    pub role: Role,
    pub status: MemberStatus,
}

#[derive(Debug, Deserialize)]
pub struct ListMembersResponse {
    pub members: Vec<CommunityMember>,
}

// ============================================================
// Event
// ============================================================

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CommunityEvent {
    pub id: String,
    pub title: String,
    pub status: EventStatus,
}

#[derive(Debug, Deserialize)]
pub struct ListEventsResponse {
    pub events: Vec<CommunityEvent>,
}

#[derive(Debug, Serialize)]
pub struct CreateEventRequest {
    pub title: String,
    pub description: String,
    #[serde(rename = "startsAt")]
    pub starts_at: String,
    #[serde(rename = "endsAt")]
    pub ends_at: String,
    pub format: EventFormat,
    pub capacity: i64,
}

#[cfg(test)]
mod tests {
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
}
