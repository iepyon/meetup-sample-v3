use axum::http::StatusCode;

use super::errors::{CreateEventError, ListEventsError};
use crate::shared::error::ApiError;

impl From<CreateEventError> for ApiError {
    fn from(error: CreateEventError) -> Self {
        match error {
            CreateEventError::CommunityNotFound => ApiError::new(
                StatusCode::NOT_FOUND,
                "COMMUNITY_NOT_FOUND",
                "コミュニティが見つかりません",
            ),
            CreateEventError::EventDateInPast => ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "EVENT_DATE_IN_PAST",
                "開始日時は現在時刻より未来でなければなりません",
            ),
            CreateEventError::EventEndBeforeStart => ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "EVENT_END_BEFORE_START",
                "終了日時は開始日時より後でなければなりません",
            ),
            CreateEventError::Internal => ApiError::internal(),
        }
    }
}

impl From<ListEventsError> for ApiError {
    fn from(error: ListEventsError) -> Self {
        match error {
            ListEventsError::CommunityNotFound => ApiError::new(
                StatusCode::NOT_FOUND,
                "COMMUNITY_NOT_FOUND",
                "コミュニティが見つかりません",
            ),
            ListEventsError::Internal => ApiError::internal(),
        }
    }
}
