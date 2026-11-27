use axum::http::StatusCode;

use super::errors::{LoginError, RegisterError};
use crate::shared::error::ApiError;

impl From<RegisterError> for ApiError {
    fn from(error: RegisterError) -> Self {
        match error {
            RegisterError::DuplicateEmail { email } => ApiError::new(
                StatusCode::CONFLICT,
                "DUPLICATE_EMAIL",
                format!("このメールアドレスは既に登録されています: {email}"),
            ),
            RegisterError::Internal => ApiError::internal(),
        }
    }
}

impl From<LoginError> for ApiError {
    fn from(error: LoginError) -> Self {
        match error {
            LoginError::InvalidCredentials => ApiError::new(
                StatusCode::UNAUTHORIZED,
                "INVALID_CREDENTIALS",
                "メールアドレスまたはパスワードが正しくありません",
            ),
            LoginError::Internal => ApiError::internal(),
        }
    }
}
