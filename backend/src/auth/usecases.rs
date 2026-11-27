use chrono::{DateTime, Utc};

use super::errors::{LoginError, RegisterError};
use super::models::{Account, CreateAccountInput, create_account};
use super::repository::AccountRepository;
use crate::shared::auth::generate_token;
use crate::shared::error::{is_unique_violation, log_internal};

/// bcrypt のコスト（旧実装と同じ 10）
const SALT_ROUNDS: u32 = 10;

/// 未登録メールでも bcrypt 検証を 1 回走らせ、応答時間でメールの登録有無を判別されないようにする。
/// （cost 10 の有効なハッシュ。どのパスワードとも一致しない）
const DUMMY_HASH: &str = "$2b$10$CwTycUXWue0Thq9StjUM0uJ8.6vbWqBqEAl0pcnBmWUKRxpOyrT/G";

// ============================================================
// アカウント登録
// ============================================================

pub struct RegisterCommand {
    pub id: String,
    pub name: String,
    pub email: String,
    pub password: String,
    pub created_at: DateTime<Utc>,
}

/// 重複チェック・パスワードハッシュ化・永続化を調整する（ドメインロジックは `create_account`）。
pub async fn register(
    accounts: &AccountRepository,
    command: RegisterCommand,
) -> Result<Account, RegisterError> {
    let duplicate = || RegisterError::DuplicateEmail;

    let existing = accounts.find_by_email(&command.email).await.map_err(|e| {
        log_internal("register.find_by_email", &e);
        RegisterError::Internal
    })?;
    if existing.is_some() {
        return Err(duplicate());
    }

    let password = command.password.clone();
    let password_hash = tokio::task::spawn_blocking(move || bcrypt::hash(password, SALT_ROUNDS))
        .await
        .map_err(|_| RegisterError::Internal)?
        .map_err(|_| RegisterError::Internal)?;

    let account = create_account(CreateAccountInput {
        id: command.id.clone(),
        name: command.name.clone(),
        email: command.email.clone(),
        password_hash,
        created_at: command.created_at,
    });

    accounts.save(&account).await.map_err(|e| {
        if is_unique_violation(&e) {
            duplicate()
        } else {
            log_internal("register.save", &e);
            RegisterError::Internal
        }
    })?;
    Ok(account)
}

// ============================================================
// ログイン
// ============================================================

pub struct LoginCommand {
    pub email: String,
    pub password: String,
}

pub struct LoginResult {
    pub token: String,
    pub account: Account,
}

/// メールアドレスとパスワードを検証し、JWT を発行する。
pub async fn login(
    accounts: &AccountRepository,
    jwt_secret: &str,
    command: LoginCommand,
) -> Result<LoginResult, LoginError> {
    let account = accounts.find_by_email(&command.email).await.map_err(|e| {
        log_internal("login.find_by_email", &e);
        LoginError::Internal
    })?;

    let hash = account
        .as_ref()
        .map_or_else(|| DUMMY_HASH.to_string(), |a| a.password_hash.clone());
    let password = command.password;
    let valid = tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash))
        .await
        .map_err(|_| LoginError::Internal)?
        .unwrap_or(false);
    let account = match account {
        Some(account) if valid => account,
        _ => return Err(LoginError::InvalidCredentials),
    };

    let token = generate_token(&account.id, jwt_secret).map_err(|_| LoginError::Internal)?;
    Ok(LoginResult { token, account })
}
