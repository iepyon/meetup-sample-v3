use chrono::{DateTime, Utc};

use crate::shared::time::to_iso;

/// アカウント集約（読み取り専用の値オブジェクトとして扱う）。
#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

pub struct CreateAccountInput {
    pub id: String,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

/// アカウントを生成する（ファクトリ関数）。入力制約は境界（validation）で検証済み。
pub fn create_account(input: CreateAccountInput) -> Account {
    Account {
        id: input.id,
        name: input.name,
        email: input.email,
        password_hash: input.password_hash,
        created_at: input.created_at,
    }
}

impl Account {
    /// レスポンス用 JSON（passwordHash を含めない）
    pub fn to_response(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "email": self.email,
            "createdAt": to_iso(&self.created_at),
        })
    }
}
