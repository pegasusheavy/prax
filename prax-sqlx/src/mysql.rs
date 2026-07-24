//! MySQL-specific functionality for SQLx.

use crate::config::DatabaseBackend;
use crate::error::SqlxResult;
use crate::types::quote_identifier;
use sqlx::Row;
use sqlx::mysql::MySqlPool;

/// MySQL-specific query helpers.
pub struct MySqlHelpers;

impl MySqlHelpers {
    /// Execute INSERT ... ON DUPLICATE KEY UPDATE (upsert).
    ///
    /// Identifiers are quoted via `quote_identifier` (embedded backticks are
    /// escaped); pass trusted identifiers only, not arbitrary user input.
    pub fn upsert_sql(table: &str, columns: &[&str], update_columns: &[&str]) -> String {
        let table = quote_identifier(DatabaseBackend::MySql, table);
        let cols = columns
            .iter()
            .map(|c| quote_identifier(DatabaseBackend::MySql, c))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders: Vec<String> = columns.iter().map(|_| "?".to_string()).collect();
        let vals = placeholders.join(", ");
        let updates: Vec<String> = update_columns
            .iter()
            .map(|c| {
                let col = quote_identifier(DatabaseBackend::MySql, c);
                format!("{} = VALUES({})", col, col)
            })
            .collect();
        let update_clause = updates.join(", ");

        format!(
            "INSERT INTO {} ({}) VALUES ({}) ON DUPLICATE KEY UPDATE {}",
            table, cols, vals, update_clause
        )
    }

    /// Generate MySQL JSON path expression.
    ///
    /// The column is quoted via `quote_identifier` and embedded single
    /// quotes in the path are escaped (`'` -> `''`). Pass trusted
    /// identifiers only, not arbitrary user input.
    pub fn json_extract(column: &str, path: &str) -> String {
        format!(
            "JSON_EXTRACT({}, '$.{}')",
            quote_identifier(DatabaseBackend::MySql, column),
            path.replace('\'', "''")
        )
    }

    /// Generate MySQL JSON_UNQUOTE expression.
    ///
    /// The column is quoted via `quote_identifier` and embedded single
    /// quotes in the path are escaped (`'` -> `''`). Pass trusted
    /// identifiers only, not arbitrary user input.
    pub fn json_unquote(column: &str, path: &str) -> String {
        format!(
            "JSON_UNQUOTE(JSON_EXTRACT({}, '$.{}'))",
            quote_identifier(DatabaseBackend::MySql, column),
            path.replace('\'', "''")
        )
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
            .map(|r| r.try_get::<String, _>(0))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(columns)
    }

    /// Generate a FULLTEXT search condition.
    ///
    /// Columns are quoted via `quote_identifier` (embedded backticks are
    /// escaped); pass trusted identifiers only, not arbitrary user input.
    /// `query` is not embedded in the generated SQL: the emitted statement
    /// contains a `?` placeholder, and the caller must bind the query value
    /// positionally.
    pub fn fulltext_match(columns: &[&str], _query: &str) -> String {
        let cols = columns
            .iter()
            .map(|c| quote_identifier(DatabaseBackend::MySql, c))
            .collect::<Vec<_>>()
            .join(", ");
        format!("MATCH({}) AGAINST(? IN BOOLEAN MODE)", cols)
    }

    /// Generate MySQL date format.
    ///
    /// Embedded single quotes in the format string are escaped (`'` -> `''`).
    /// The column is interpolated unquoted; pass trusted identifiers only.
    pub fn date_format(column: &str, format: &str) -> String {
        format!("DATE_FORMAT({}, '{}')", column, format.replace('\'', "''"))
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
        assert!(sql.contains("INSERT INTO `users`"));
        assert!(sql.contains("ON DUPLICATE KEY UPDATE"));
        assert!(sql.contains("`name` = VALUES(`name`)"));
    }

    #[test]
    fn test_upsert_sql_escapes_identifiers() {
        // Embedded backticks in identifiers are escaped.
        let sql = MySqlHelpers::upsert_sql("us`ers", &["na`me"], &["na`me"]);
        assert!(sql.contains("INSERT INTO `us``ers`"));
        assert!(sql.contains("`na``me` = VALUES(`na``me`)"));
    }

    #[test]
    fn test_json_extract() {
        assert_eq!(
            MySqlHelpers::json_extract("data", "name"),
            "JSON_EXTRACT(`data`, '$.name')"
        );
        // Embedded single quotes in the path are escaped.
        assert_eq!(
            MySqlHelpers::json_extract("data", "na'me"),
            "JSON_EXTRACT(`data`, '$.na''me')"
        );
        assert_eq!(
            MySqlHelpers::json_unquote("data", "na'me"),
            "JSON_UNQUOTE(JSON_EXTRACT(`data`, '$.na''me'))"
        );
    }

    #[test]
    fn test_date_format() {
        assert_eq!(
            MySqlHelpers::date_format("created_at", "%Y-%m-%d"),
            "DATE_FORMAT(created_at, '%Y-%m-%d')"
        );
        // Embedded single quotes in the format string are escaped.
        assert_eq!(
            MySqlHelpers::date_format("created_at", "%Y'%m"),
            "DATE_FORMAT(created_at, '%Y''%m')"
        );
    }

    #[test]
    fn test_fulltext_match() {
        assert_eq!(
            MySqlHelpers::fulltext_match(&["title", "content"], "search"),
            "MATCH(`title`, `content`) AGAINST(? IN BOOLEAN MODE)"
        );
    }
}
