use chrono::{DateTime, Utc};
use sqlx::{FromRow, QueryBuilder, Sqlite, SqlitePool};

use super::models::{
    Community, CommunityCategory, CommunityMember, CommunityVisibility, MemberRole, MemberStatus,
};
use crate::shared::time::{from_iso, to_iso};

// ============================================================
// コミュニティ
// ============================================================

#[derive(FromRow)]
struct CommunityRow {
    id: String,
    name: String,
    description: Option<String>,
    category: String,
    visibility: String,
    created_at: String,
    updated_at: String,
}

impl From<CommunityRow> for Community {
    fn from(r: CommunityRow) -> Self {
        Community {
            id: r.id,
            name: r.name,
            description: r.description,
            category: CommunityCategory::parse(&r.category).unwrap_or(CommunityCategory::Tech),
            visibility: CommunityVisibility::parse(&r.visibility)
                .unwrap_or(CommunityVisibility::Private),
            created_at: from_iso(&r.created_at),
            updated_at: from_iso(&r.updated_at),
        }
    }
}

pub struct FindAllOptions<'a> {
    pub category: Option<&'a str>,
    pub member_account_id: Option<&'a str>,
    pub visibility: Option<CommunityVisibility>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Clone)]
pub struct CommunityRepository {
    pool: SqlitePool,
}

impl CommunityRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<Community>, sqlx::Error> {
        let row = sqlx::query_as::<_, CommunityRow>("SELECT * FROM communities WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Community::from))
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<Community>, sqlx::Error> {
        let row = sqlx::query_as::<_, CommunityRow>("SELECT * FROM communities WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Community::from))
    }

    /// コミュニティとオーナーメンバーを 1 トランザクションで保存する。
    pub async fn save_with_owner(
        &self,
        community: &Community,
        owner: &CommunityMember,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO communities (id, name, description, category, visibility, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&community.id)
        .bind(&community.name)
        .bind(&community.description)
        .bind(community.category.as_str())
        .bind(community.visibility.as_str())
        .bind(to_iso(&community.created_at))
        .bind(to_iso(&community.updated_at))
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO community_members (id, community_id, account_id, role, status, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&owner.id)
        .bind(&owner.community_id)
        .bind(&owner.account_id)
        .bind(owner.role.as_str())
        .bind(owner.status.as_str())
        .bind(to_iso(&owner.created_at))
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    pub async fn find_all(
        &self,
        options: FindAllOptions<'_>,
    ) -> Result<(Vec<Community>, i64), sqlx::Error> {
        let push_where = |qb: &mut QueryBuilder<'_, Sqlite>| {
            qb.push(" WHERE 1 = 1");
            if let Some(category) = options.category {
                qb.push(" AND category = ").push_bind(category.to_string());
            }
            if let Some(visibility) = options.visibility {
                qb.push(" AND visibility = ").push_bind(visibility.as_str());
            }
            if let Some(account_id) = options.member_account_id {
                qb.push(
                    " AND EXISTS (SELECT 1 FROM community_members m \
                     WHERE m.community_id = communities.id AND m.status = 'ACTIVE' AND m.account_id = ",
                )
                .push_bind(account_id.to_string())
                .push(")");
            }
        };

        let mut count_qb = QueryBuilder::<Sqlite>::new("SELECT COUNT(*) FROM communities");
        push_where(&mut count_qb);
        let total: i64 = count_qb.build_query_scalar().fetch_one(&self.pool).await?;

        let mut qb = QueryBuilder::<Sqlite>::new("SELECT * FROM communities");
        push_where(&mut qb);
        qb.push(" ORDER BY created_at DESC LIMIT ")
            .push_bind(options.limit)
            .push(" OFFSET ")
            .push_bind(options.offset);
        let rows = qb
            .build_query_as::<CommunityRow>()
            .fetch_all(&self.pool)
            .await?;
        Ok((rows.into_iter().map(Community::from).collect(), total))
    }

    pub async fn count_by_owner_account_id(&self, account_id: &str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM community_members WHERE account_id = ? AND role = 'OWNER'",
        )
        .bind(account_id)
        .fetch_one(&self.pool)
        .await
    }
}

// ============================================================
// コミュニティメンバー
// ============================================================

#[derive(FromRow)]
struct MemberRow {
    id: String,
    community_id: String,
    account_id: String,
    role: String,
    status: String,
    created_at: String,
}

impl From<MemberRow> for CommunityMember {
    fn from(r: MemberRow) -> Self {
        CommunityMember {
            id: r.id,
            community_id: r.community_id,
            account_id: r.account_id,
            role: MemberRole::parse(&r.role).unwrap_or(MemberRole::Member),
            status: MemberStatus::parse(&r.status).unwrap_or(MemberStatus::Pending),
            created_at: from_iso(&r.created_at),
        }
    }
}

/// メンバー一覧の Read モデル行（アカウント名を結合済み）
#[derive(Debug, Clone)]
pub struct MemberReadRow {
    pub id: String,
    pub community_id: String,
    pub account_id: String,
    pub account_name: String,
    pub role: MemberRole,
    pub status: MemberStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct MemberReadDbRow {
    id: String,
    community_id: String,
    account_id: String,
    account_name: String,
    role: String,
    status: String,
    created_at: String,
}

#[derive(Clone)]
pub struct CommunityMemberRepository {
    pool: SqlitePool,
}

impl CommunityMemberRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<CommunityMember>, sqlx::Error> {
        let row = sqlx::query_as::<_, MemberRow>("SELECT * FROM community_members WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(CommunityMember::from))
    }

    pub async fn find_by_ids(
        &self,
        community_id: &str,
        account_id: &str,
    ) -> Result<Option<CommunityMember>, sqlx::Error> {
        let row = sqlx::query_as::<_, MemberRow>(
            "SELECT * FROM community_members WHERE community_id = ? AND account_id = ?",
        )
        .bind(community_id)
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(CommunityMember::from))
    }

    pub async fn save(&self, member: &CommunityMember) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO community_members (id, community_id, account_id, role, status, created_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET role = excluded.role, status = excluded.status",
        )
        .bind(&member.id)
        .bind(&member.community_id)
        .bind(&member.account_id)
        .bind(member.role.as_str())
        .bind(member.status.as_str())
        .bind(to_iso(&member.created_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM community_members WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// メンバーをアカウント名付きで取得する。
    /// 既定は ACTIVE のみ。`include_pending` が真なら承認待ち（PENDING）も含め、
    /// 審査対象がページングで埋もれないよう PENDING を先頭に並べる（同じ状態の中は作成日時の昇順）。
    pub async fn list_with_names(
        &self,
        community_id: &str,
        include_pending: bool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<MemberReadRow>, i64), sqlx::Error> {
        let active = MemberStatus::Active.as_str();
        // PENDING を含めないときは ACTIVE を 2 回渡し、条件を ACTIVE のみに保つ
        let second = if include_pending {
            MemberStatus::Pending.as_str()
        } else {
            active
        };
        let rows = sqlx::query_as::<_, MemberReadDbRow>(
            "SELECT m.id, m.community_id, m.account_id, a.name AS account_name, \
                    m.role, m.status, m.created_at \
             FROM community_members m JOIN accounts a ON a.id = m.account_id \
             WHERE m.community_id = ? AND m.status IN (?, ?) \
             ORDER BY (m.status = ?) ASC, m.created_at ASC LIMIT ? OFFSET ?",
        )
        .bind(community_id)
        .bind(active)
        .bind(second)
        .bind(active)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM community_members WHERE community_id = ? AND status IN (?, ?)",
        )
        .bind(community_id)
        .bind(active)
        .bind(second)
        .fetch_one(&self.pool)
        .await?;

        let members = rows
            .into_iter()
            .map(|r| MemberReadRow {
                id: r.id,
                community_id: r.community_id,
                account_id: r.account_id,
                account_name: r.account_name,
                role: MemberRole::parse(&r.role).unwrap_or(MemberRole::Member),
                status: MemberStatus::parse(&r.status).unwrap_or(MemberStatus::Pending),
                created_at: from_iso(&r.created_at),
            })
            .collect();
        Ok((members, total))
    }
}
