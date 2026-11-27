use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

/// スキーマ定義（旧 Prisma スキーマと同一の構造）。起動時に冪等に適用する。
const SCHEMA: &str = include_str!("schema.sql");

/// `file:./dev.db` / `sqlite:...` / 素のパス / `:memory:` を接続して返す。
pub async fn connect(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let in_memory = database_url.contains(":memory:");
    let url = database_url
        .strip_prefix("file:")
        .map(|p| format!("sqlite://{p}"))
        .unwrap_or_else(|| {
            if database_url.starts_with("sqlite:") {
                database_url.to_string()
            } else {
                format!("sqlite://{database_url}")
            }
        });
    let options = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        // インメモリ DB は接続ごとに別 DB になるため 1 接続に固定する
        .max_connections(if in_memory { 1 } else { 5 })
        // 接続が再作成されるとインメモリ DB が消えるため、寿命による再作成を無効化する
        .idle_timeout(None)
        .max_lifetime(None)
        .connect_with(options)
        .await?;
    apply_schema(&pool).await?;
    Ok(pool)
}

pub async fn apply_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for statement in SCHEMA.split(';') {
        if statement.trim().is_empty() {
            continue;
        }
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}
