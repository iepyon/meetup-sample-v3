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
mod tests;
