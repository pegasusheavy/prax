//! PostgreSQL-specific functionality for SQLx.

use crate::error::SqlxResult;
use sqlx::postgres::{PgPool, PgRow};
use sqlx::Row;

/// PostgreSQL-specific query helpers.
pub struct PgHelpers;

impl PgHelpers {
    /// Execute a query with RETURNING clause.
    pub async fn query_returning(
        pool: &PgPool,
        sql: &str,
    ) -> SqlxResult<Vec<PgRow>> {
        let rows = sqlx::query(sql).fetch_all(pool).await?;
        Ok(rows)
    }

    /// Execute INSERT ... ON CONFLICT (upsert).
    pub async fn upsert(
        _pool: &PgPool,
        table: &str,
        columns: &[&str],
        conflict_columns: &[&str],
        update_columns: &[&str],
    ) -> SqlxResult<String> {
        let cols = columns.join(", ");
        let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();
        let vals = placeholders.join(", ");
        let conflict = conflict_columns.join(", ");
        let updates: Vec<String> = update_columns
            .iter()
            .map(|c| format!("{} = EXCLUDED.{}", c, c))
            .collect();
        let update_clause = updates.join(", ");

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {} RETURNING *",
            table, cols, vals, conflict, update_clause
        );

        Ok(sql)
    }

    /// Generate a PostgreSQL array literal.
    pub fn array_literal<T: std::fmt::Display>(values: &[T]) -> String {
        let items: Vec<String> = values.iter().map(|v| format!("'{}'", v)).collect();
        format!("ARRAY[{}]", items.join(", "))
    }

    /// Generate a PostgreSQL JSON/JSONB path expression.
    pub fn json_path(column: &str, path: &[&str]) -> String {
        if path.is_empty() {
            column.to_string()
        } else {
            let path_str: Vec<String> = path.iter().map(|p| format!("'{}'", p)).collect();
            format!("{}->>{}", column, path_str.join("->"))
        }
    }

    /// Check if a PostgreSQL extension is available.
    pub async fn has_extension(pool: &PgPool, extension: &str) -> SqlxResult<bool> {
        let sql = "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = $1)";
        let row = sqlx::query(sql).bind(extension).fetch_one(pool).await?;
        let exists: bool = row.try_get(0)?;
        Ok(exists)
    }

    /// Get PostgreSQL version.
    pub async fn version(pool: &PgPool) -> SqlxResult<String> {
        let sql = "SELECT version()";
        let row = sqlx::query(sql).fetch_one(pool).await?;
        let version: String = row.try_get(0)?;
        Ok(version)
    }

    /// Execute LISTEN for notifications.
    pub async fn listen(pool: &PgPool, channel: &str) -> SqlxResult<()> {
        let sql = format!("LISTEN {}", channel);
        sqlx::query(&sql).execute(pool).await?;
        Ok(())
    }

    /// Execute NOTIFY.
    pub async fn notify(pool: &PgPool, channel: &str, payload: &str) -> SqlxResult<()> {
        let sql = format!("NOTIFY {}, '{}'", channel, payload.replace('\'', "''"));
        sqlx::query(&sql).execute(pool).await?;
        Ok(())
    }
}

/// PostgreSQL advisory lock helpers.
pub struct AdvisoryLock;

impl AdvisoryLock {
    /// Acquire an advisory lock (blocking).
    pub async fn acquire(pool: &PgPool, key: i64) -> SqlxResult<()> {
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(key)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Try to acquire an advisory lock (non-blocking).
    pub async fn try_acquire(pool: &PgPool, key: i64) -> SqlxResult<bool> {
        let row = sqlx::query("SELECT pg_try_advisory_lock($1)")
            .bind(key)
            .fetch_one(pool)
            .await?;
        let acquired: bool = row.try_get(0)?;
        Ok(acquired)
    }

    /// Release an advisory lock.
    pub async fn release(pool: &PgPool, key: i64) -> SqlxResult<()> {
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(key)
            .execute(pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_literal() {
        assert_eq!(
            PgHelpers::array_literal(&[1, 2, 3]),
            "ARRAY['1', '2', '3']"
        );
        assert_eq!(
            PgHelpers::array_literal(&["a", "b"]),
            "ARRAY['a', 'b']"
        );
    }

    #[test]
    fn test_json_path() {
        assert_eq!(PgHelpers::json_path("data", &[]), "data");
        assert_eq!(
            PgHelpers::json_path("data", &["name"]),
            "data->>'name'"
        );
        assert_eq!(
            PgHelpers::json_path("data", &["user", "name"]),
            "data->>'user'->'name'"
        );
    }
}

