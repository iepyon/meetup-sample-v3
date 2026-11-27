use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use serde_json::{Value, json};

use super::repository::AccountRepository;
use super::usecases::{
    LoginCommand, RegisterCommand, login as login_usecase, register as register_usecase,
};
use crate::app::AppState;
use crate::shared::auth::{generate_token, require_account};
use crate::shared::error::{ApiError, ApiResult, log_internal};
use crate::shared::validation::{json_object, required_email, required_password, required_string};

/// POST /auth/register - アカウント登録
pub async fn register(
    State(state): State<AppState>,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let obj = json_object(&body)?;
    let command = RegisterCommand {
        id: uuid::Uuid::new_v4().to_string(),
        created_at: chrono::Utc::now(),
        name: required_string(&obj, "name", 1, 100)?,
        email: required_email(&obj, "email")?,
        password: required_password(&obj, "password")?,
    };

    let accounts = AccountRepository::new(state.pool.clone());
    let account = register_usecase(&accounts, command).await?;
    let token = generate_token(&account.id, &state.jwt_secret).map_err(|_| ApiError::internal())?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "token": token, "account": account.to_response() })),
    ))
}

/// POST /auth/login - ログイン
pub async fn login(State(state): State<AppState>, body: Bytes) -> ApiResult<Json<Value>> {
    let obj = json_object(&body)?;
    let command = LoginCommand {
        email: required_email(&obj, "email")?,
        password: required_password(&obj, "password")?,
    };

    let accounts = AccountRepository::new(state.pool.clone());
    let result = login_usecase(&accounts, &state.jwt_secret, command).await?;
    Ok(Json(
        json!({ "token": result.token, "account": result.account.to_response() }),
    ))
}

/// GET /auth/me - 現在のアカウント情報取得
pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let account_id = require_account(&headers, &state.jwt_secret)?;
    let accounts = AccountRepository::new(state.pool.clone());
    let account = accounts.find_by_id(&account_id).await.map_err(|e| {
        log_internal("auth.me", &e);
        ApiError::internal()
    })?;
    match account {
        Some(account) => Ok(Json(json!({ "account": account.to_response() }))),
        None => Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "ACCOUNT_NOT_FOUND",
            "Account not found",
        )),
    }
}
