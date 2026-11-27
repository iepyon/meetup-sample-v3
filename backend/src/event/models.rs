use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use super::errors::CreateEventError;
use crate::shared::time::to_iso;

crate::string_enum!(EventFormat { Online => "ONLINE", Offline => "OFFLINE", Hybrid => "HYBRID" });
crate::string_enum!(EventStatus { Draft => "DRAFT", Published => "PUBLISHED", Closed => "CLOSED", Cancelled => "CANCELLED" });

#[derive(Debug, Clone)]
pub struct Event {
    pub id: String,
    pub community_id: String,
    pub created_by: String,
    pub title: String,
    pub description: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub format: EventFormat,
    pub capacity: i64,
    pub status: EventStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Event {
    pub fn to_response(&self) -> Value {
        json!({
            "id": self.id,
            "communityId": self.community_id,
            "title": self.title,
            "description": self.description,
            "startsAt": to_iso(&self.starts_at),
            "endsAt": to_iso(&self.ends_at),
            "format": self.format.as_str(),
            "capacity": self.capacity,
            "status": self.status.as_str(),
            "createdAt": to_iso(&self.created_at),
            "updatedAt": to_iso(&self.updated_at),
        })
    }
}

pub struct CreateEventInput {
    pub id: String,
    pub community_id: String,
    pub created_by: String,
    pub title: String,
    pub description: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub format: EventFormat,
    pub capacity: i64,
    pub now: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// イベントを DRAFT で生成する（ファクトリ）。日時の不変条件を検証する。
pub fn create_event(input: CreateEventInput) -> Result<Event, CreateEventError> {
    if input.starts_at <= input.now {
        return Err(CreateEventError::EventDateInPast);
    }
    if input.ends_at <= input.starts_at {
        return Err(CreateEventError::EventEndBeforeStart);
    }
    Ok(Event {
        id: input.id,
        community_id: input.community_id,
        created_by: input.created_by,
        title: input.title,
        description: input.description,
        starts_at: input.starts_at,
        ends_at: input.ends_at,
        format: input.format,
        capacity: input.capacity,
        status: EventStatus::Draft,
        created_at: input.created_at,
        updated_at: input.updated_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn input(starts_in: i64, ends_in: i64) -> CreateEventInput {
        let now = Utc::now();
        CreateEventInput {
            id: "e".into(),
            community_id: "c".into(),
            created_by: "a".into(),
            title: "t".into(),
            description: None,
            starts_at: now + Duration::hours(starts_in),
            ends_at: now + Duration::hours(ends_in),
            format: EventFormat::Online,
            capacity: 10,
            now,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn 正しい日時ならdraftで作成される() {
        let event = create_event(input(1, 2)).unwrap();
        assert_eq!(event.status, EventStatus::Draft);
    }

    #[test]
    fn 過去の開始日時は拒否される() {
        assert!(matches!(
            create_event(input(-1, 2)),
            Err(CreateEventError::EventDateInPast)
        ));
    }

    #[test]
    fn 終了が開始以前なら拒否される() {
        assert!(matches!(
            create_event(input(2, 2)),
            Err(CreateEventError::EventEndBeforeStart)
        ));
    }
}
