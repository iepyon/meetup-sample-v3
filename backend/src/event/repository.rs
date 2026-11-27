use sqlx::{FromRow, SqlitePool};

use super::models::{Event, EventFormat, EventStatus};
use crate::shared::time::{from_iso, to_iso};

#[derive(FromRow)]
struct EventRow {
    id: String,
    community_id: String,
    created_by: String,
    title: String,
    description: Option<String>,
    starts_at: String,
    ends_at: String,
    format: String,
    capacity: i64,
    status: String,
    created_at: String,
    updated_at: String,
}

impl From<EventRow> for Event {
    fn from(r: EventRow) -> Self {
        Event {
            id: r.id,
            community_id: r.community_id,
            created_by: r.created_by,
            title: r.title,
            description: r.description,
            starts_at: from_iso(&r.starts_at),
            ends_at: from_iso(&r.ends_at),
            format: EventFormat::parse(&r.format).unwrap_or(EventFormat::Online),
            capacity: r.capacity,
            status: EventStatus::parse(&r.status).unwrap_or(EventStatus::Draft),
            created_at: from_iso(&r.created_at),
            updated_at: from_iso(&r.updated_at),
        }
    }
}

#[derive(Clone)]
pub struct EventRepository {
    pool: SqlitePool,
}

impl EventRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn save(&self, event: &Event) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO events (id, community_id, created_by, title, description, starts_at, \
             ends_at, format, capacity, status, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET title = excluded.title, description = excluded.description, \
             starts_at = excluded.starts_at, ends_at = excluded.ends_at, format = excluded.format, \
             capacity = excluded.capacity, status = excluded.status, updated_at = excluded.updated_at",
        )
        .bind(&event.id)
        .bind(&event.community_id)
        .bind(&event.created_by)
        .bind(&event.title)
        .bind(&event.description)
        .bind(to_iso(&event.starts_at))
        .bind(to_iso(&event.ends_at))
        .bind(event.format.as_str())
        .bind(event.capacity)
        .bind(event.status.as_str())
        .bind(to_iso(&event.created_at))
        .bind(to_iso(&event.updated_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 一覧: DRAFT は作成者本人にだけ見える。開始日時の昇順。
    pub async fn list_by_community(
        &self,
        community_id: &str,
        viewer_account_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Event>, i64), sqlx::Error> {
        // viewer 未指定時は空文字（created_by は UUID のため一致しない）
        let viewer = viewer_account_id.unwrap_or("");
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT * FROM events WHERE community_id = ? AND (status != 'DRAFT' OR created_by = ?) \
             ORDER BY starts_at ASC LIMIT ? OFFSET ?",
        )
        .bind(community_id)
        .bind(viewer)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM events WHERE community_id = ? AND (status != 'DRAFT' OR created_by = ?)",
        )
        .bind(community_id)
        .bind(viewer)
        .fetch_one(&self.pool)
        .await?;
        Ok((rows.into_iter().map(Event::from).collect(), total))
    }
}
