use axum::http::StatusCode;

use super::errors::{LoginError, RegisterError};
use crate::shared::error::ApiError;

impl From<RegisterError> for ApiError {
    fn from(error: RegisterError) -> Self {
        match error {
            // 応答にメールアドレスを含めない（ログや画面への不要な露出を避ける）
            RegisterError::DuplicateEmail => ApiError::new(
                StatusCode::CONFLICT,
                "DUPLICATE_EMAIL",
                "このメールアドレスは既に登録されています",
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
