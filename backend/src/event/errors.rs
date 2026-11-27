use chrono::{DateTime, Utc};

use crate::shared::error::log_internal;

#[derive(Debug)]
pub enum CreateEventError {
    CommunityNotFound,
    EventDateInPast,
    EventEndBeforeStart,
    Internal,
}

impl From<sqlx::Error> for CreateEventError {
    fn from(e: sqlx::Error) -> Self {
        log_internal("CreateEventError", &e);
        Self::Internal
    }
}

#[derive(Debug)]
pub enum ListEventsError {
    CommunityNotFound,
    Internal,
}

impl From<sqlx::Error> for ListEventsError {
    fn from(e: sqlx::Error) -> Self {
        log_internal("ListEventsError", &e);
        Self::Internal
    }
}

#[derive(Debug, Clone)]
pub struct EventCreatedEvent {
    pub event_id: String,
    pub community_id: String,
    pub created_by: String,
    pub title: String,
    pub occurred_at: DateTime<Utc>,
}
