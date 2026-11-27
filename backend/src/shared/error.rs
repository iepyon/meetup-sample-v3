use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

// ============================================================
// API エラー（HTTP レスポンスに変換される `{ code, message }` 形式）
// ============================================================

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", message)
    }

    pub fn invalid_json() -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "INVALID_JSON",
            "Request body is not valid JSON",
        )
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "Authentication required",
        )
    }

    pub fn forbidden() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "FORBIDDEN",
            "この操作を行う権限がありません",
        )
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", "Resource not found")
    }

    /// 内部エラー。詳細はレスポンスに含めず、種別だけをログに残す。
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Internal server error",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({ "code": self.code, "message": self.message })),
        )
            .into_response()
    }
}

/// sqlx のエラーメッセージにはクエリ引数（メールアドレス・パスワードハッシュ等）が
/// 含まれ得るため、メッセージは出さずエラー種別だけをログに残す。
pub fn log_internal(context: &str, err: &sqlx::Error) {
    let kind = match err {
        sqlx::Error::Database(_) => "Database",
        sqlx::Error::Io(_) => "Io",
        sqlx::Error::PoolTimedOut => "PoolTimedOut",
        sqlx::Error::RowNotFound => "RowNotFound",
        _ => "Other",
    };
    eprintln!("[error] {context}: sqlx error kind={kind}");
}

pub fn is_unique_violation(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(e) if e.is_unique_violation())
}
