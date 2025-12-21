//! MySQL-specific functionality for SQLx.

use crate::error::SqlxResult;
use sqlx::Row;
use sqlx::mysql::MySqlPool;

/// MySQL-specific query helpers.
pub struct MySqlHelpers;

impl MySqlHelpers {
    /// Execute INSERT ... ON DUPLICATE KEY UPDATE (upsert).
    pub fn upsert_sql(table: &str, columns: &[&str], update_columns: &[&str]) -> String {
        let cols = columns.join(", ");
        let placeholders: Vec<String> = columns.iter().map(|_| "?".to_string()).collect();
        let vals = placeholders.join(", ");
        let updates: Vec<String> = update_columns
            .iter()
            .map(|c| format!("{} = VALUES({})", c, c))
            .collect();
        let update_clause = updates.join(", ");

        format!(
            "INSERT INTO {} ({}) VALUES ({}) ON DUPLICATE KEY UPDATE {}",
            table, cols, vals, update_clause
        )
    }

    /// Generate MySQL JSON path expression.
    pub fn json_extract(column: &str, path: &str) -> String {
        format!("JSON_EXTRACT({}, '$.{}')", column, path)
    }

    /// Generate MySQL JSON_UNQUOTE expression.
    pub fn json_unquote(column: &str, path: &str) -> String {
        format!("JSON_UNQUOTE(JSON_EXTRACT({}, '$.{}'))", column, path)
    }

    /// Get last insert ID.
    pub async fn last_insert_id(pool: &MySqlPool) -> SqlxResult<u64> {
        let row = sqlx::query("SELECT LAST_INSERT_ID()")
            .fetch_one(pool)
            .await?;
        let id: u64 = row.try_get(0)?;
        Ok(id)
    }

    /// Get MySQL version.
    pub async fn version(pool: &MySqlPool) -> SqlxResult<String> {
        let row = sqlx::query("SELECT VERSION()").fetch_one(pool).await?;
        let version: String = row.try_get(0)?;
        Ok(version)
    }

    /// Check if a table exists.
    pub async fn table_exists(pool: &MySqlPool, table: &str) -> SqlxResult<bool> {
        let sql = "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = ?";
        let row = sqlx::query(sql).bind(table).fetch_one(pool).await?;
        let count: i64 = row.try_get(0)?;
        Ok(count > 0)
    }

    /// Get table columns.
    pub async fn get_columns(pool: &MySqlPool, table: &str) -> SqlxResult<Vec<String>> {
        let sql = "SELECT column_name FROM information_schema.columns WHERE table_name = ? ORDER BY ordinal_position";
        let rows = sqlx::query(sql).bind(table).fetch_all(pool).await?;
        let columns: Vec<String> = rows
            .iter()
            .map(|r| r.try_get::<String, _>(0).unwrap_or_default())
            .collect();
        Ok(columns)
    }

    /// Generate a FULLTEXT search condition.
    pub fn fulltext_match(columns: &[&str], _query: &str) -> String {
        let cols = columns.join(", ");
        format!("MATCH({}) AGAINST(? IN BOOLEAN MODE)", cols)
    }

    /// Generate MySQL date format.
    pub fn date_format(column: &str, format: &str) -> String {
        format!("DATE_FORMAT({}, '{}')", column, format)
    }
}

/// MySQL lock helpers.
pub struct MySqlLock;

impl MySqlLock {
    /// Get a named lock.
    pub async fn get_lock(pool: &MySqlPool, name: &str, timeout: i32) -> SqlxResult<bool> {
        let row = sqlx::query("SELECT GET_LOCK(?, ?)")
            .bind(name)
            .bind(timeout)
            .fetch_one(pool)
            .await?;
        let result: Option<i32> = row.try_get(0)?;
        Ok(result == Some(1))
    }

    /// Release a named lock.
    pub async fn release_lock(pool: &MySqlPool, name: &str) -> SqlxResult<bool> {
        let row = sqlx::query("SELECT RELEASE_LOCK(?)")
            .bind(name)
            .fetch_one(pool)
            .await?;
        let result: Option<i32> = row.try_get(0)?;
        Ok(result == Some(1))
    }

    /// Check if a named lock is free.
    pub async fn is_free_lock(pool: &MySqlPool, name: &str) -> SqlxResult<bool> {
        let row = sqlx::query("SELECT IS_FREE_LOCK(?)")
            .bind(name)
            .fetch_one(pool)
            .await?;
        let result: Option<i32> = row.try_get(0)?;
        Ok(result == Some(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upsert_sql() {
        let sql = MySqlHelpers::upsert_sql("users", &["id", "name", "email"], &["name", "email"]);
        assert!(sql.contains("INSERT INTO users"));
        assert!(sql.contains("ON DUPLICATE KEY UPDATE"));
        assert!(sql.contains("name = VALUES(name)"));
    }

    #[test]
    fn test_json_extract() {
        assert_eq!(
            MySqlHelpers::json_extract("data", "name"),
            "JSON_EXTRACT(data, '$.name')"
        );
    }

    #[test]
    fn test_fulltext_match() {
        assert_eq!(
            MySqlHelpers::fulltext_match(&["title", "content"], "search"),
            "MATCH(title, content) AGAINST(? IN BOOLEAN MODE)"
        );
    }
}
