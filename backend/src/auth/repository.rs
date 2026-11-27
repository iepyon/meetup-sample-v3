use sqlx::{FromRow, SqlitePool};

use super::models::Account;
use crate::shared::time::{from_iso, to_iso};

#[derive(FromRow)]
struct AccountRow {
    id: String,
    name: String,
    email: String,
    password_hash: String,
    created_at: String,
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
        Account {
            id: row.id,
            name: row.name,
            email: row.email,
            password_hash: row.password_hash,
            created_at: from_iso(&row.created_at),
        }
    }
}

#[derive(Clone)]
pub struct AccountRepository {
    pool: SqlitePool,
}

impl AccountRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<Account>, sqlx::Error> {
        let row = sqlx::query_as::<_, AccountRow>("SELECT * FROM accounts WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Account::from))
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<Account>, sqlx::Error> {
        let row = sqlx::query_as::<_, AccountRow>("SELECT * FROM accounts WHERE email = ?")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Account::from))
    }

    pub async fn save(&self, account: &Account) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO accounts (id, name, email, password_hash, created_at) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, \
             email = excluded.email, password_hash = excluded.password_hash",
        )
        .bind(&account.id)
        .bind(&account.name)
        .bind(&account.email)
        .bind(&account.password_hash)
        .bind(to_iso(&account.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
