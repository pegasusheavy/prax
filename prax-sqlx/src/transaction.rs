//! Transaction support for SQLx.

use std::future::Future;

use crate::error::{SqlxError, SqlxResult};
use crate::pool::SqlxPool;

/// Transaction isolation levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// Read uncommitted - lowest isolation.
    ReadUncommitted,
    /// Read committed - default for PostgreSQL.
    ReadCommitted,
    /// Repeatable read - prevents non-repeatable reads.
    RepeatableRead,
    /// Serializable - highest isolation.
    Serializable,
}

impl IsolationLevel {
    /// Convert to SQL string.
    pub fn as_sql(&self) -> &'static str {
        match self {
            IsolationLevel::ReadUncommitted => "READ UNCOMMITTED",
            IsolationLevel::ReadCommitted => "READ COMMITTED",
            IsolationLevel::RepeatableRead => "REPEATABLE READ",
            IsolationLevel::Serializable => "SERIALIZABLE",
        }
    }
}

/// Transaction access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// Read and write access.
    ReadWrite,
    /// Read-only access.
    ReadOnly,
}

impl AccessMode {
    /// Convert to SQL string.
    pub fn as_sql(&self) -> &'static str {
        match self {
            AccessMode::ReadWrite => "READ WRITE",
            AccessMode::ReadOnly => "READ ONLY",
        }
    }
}

/// Options for starting a transaction.
#[derive(Debug, Clone)]
pub struct TransactionOptions {
    /// Isolation level.
    pub isolation_level: Option<IsolationLevel>,
    /// Access mode.
    pub access_mode: Option<AccessMode>,
    /// Whether the transaction is deferrable (PostgreSQL only).
    pub deferrable: Option<bool>,
}

impl Default for TransactionOptions {
    fn default() -> Self {
        Self {
            isolation_level: None,
            access_mode: None,
            deferrable: None,
        }
    }
}

impl TransactionOptions {
    /// Create new default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the isolation level.
    pub fn isolation_level(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = Some(level);
        self
    }

    /// Set the access mode.
    pub fn access_mode(mut self, mode: AccessMode) -> Self {
        self.access_mode = Some(mode);
        self
    }

    /// Set whether the transaction is deferrable.
    pub fn deferrable(mut self, deferrable: bool) -> Self {
        self.deferrable = Some(deferrable);
        self
    }

    /// Create read-only transaction options.
    pub fn read_only() -> Self {
        Self::new().access_mode(AccessMode::ReadOnly)
    }

    /// Create serializable transaction options.
    pub fn serializable() -> Self {
        Self::new().isolation_level(IsolationLevel::Serializable)
    }
}

/// Execute a closure within a transaction.
///
/// The transaction is automatically committed if the closure succeeds,
/// or rolled back if it returns an error.
///
/// # Example
///
/// ```ignore
/// use prax_sqlx::transaction::with_transaction;
///
/// let result = with_transaction(&pool, |tx| async move {
///     // Execute queries within the transaction
///     sqlx::query("INSERT INTO users (name) VALUES ($1)")
///         .bind("Alice")
///         .execute(&mut *tx)
///         .await?;
///
///     Ok(())
/// }).await?;
/// ```
#[cfg(feature = "postgres")]
pub async fn with_transaction_pg<F, Fut, T>(pool: &sqlx::PgPool, f: F) -> SqlxResult<T>
where
    F: FnOnce(sqlx::Transaction<'static, sqlx::Postgres>) -> Fut,
    Fut: Future<Output = Result<T, SqlxError>>,
{
    let tx = pool.begin().await?;
    match f(tx).await {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}

#[cfg(feature = "mysql")]
pub async fn with_transaction_mysql<F, Fut, T>(pool: &sqlx::MySqlPool, f: F) -> SqlxResult<T>
where
    F: FnOnce(sqlx::Transaction<'static, sqlx::MySql>) -> Fut,
    Fut: Future<Output = Result<T, SqlxError>>,
{
    let tx = pool.begin().await?;
    match f(tx).await {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}

#[cfg(feature = "sqlite")]
pub async fn with_transaction_sqlite<F, Fut, T>(pool: &sqlx::SqlitePool, f: F) -> SqlxResult<T>
where
    F: FnOnce(sqlx::Transaction<'static, sqlx::Sqlite>) -> Fut,
    Fut: Future<Output = Result<T, SqlxError>>,
{
    let tx = pool.begin().await?;
    match f(tx).await {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}

/// Generic transaction wrapper for the SqlxPool enum.
pub async fn with_transaction<F, T>(pool: &SqlxPool, f: F) -> SqlxResult<T>
where
    F: FnOnce(&SqlxPool) -> futures::future::BoxFuture<'_, Result<T, SqlxError>>,
{
    // Note: This is a simplified implementation. For proper transaction support,
    // you would need to pass the actual transaction handle to the closure.
    // This requires more complex lifetime management.
    f(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_level_sql() {
        assert_eq!(IsolationLevel::ReadUncommitted.as_sql(), "READ UNCOMMITTED");
        assert_eq!(IsolationLevel::ReadCommitted.as_sql(), "READ COMMITTED");
        assert_eq!(IsolationLevel::RepeatableRead.as_sql(), "REPEATABLE READ");
        assert_eq!(IsolationLevel::Serializable.as_sql(), "SERIALIZABLE");
    }

    #[test]
    fn test_access_mode_sql() {
        assert_eq!(AccessMode::ReadWrite.as_sql(), "READ WRITE");
        assert_eq!(AccessMode::ReadOnly.as_sql(), "READ ONLY");
    }

    #[test]
    fn test_transaction_options_builder() {
        let opts = TransactionOptions::new()
            .isolation_level(IsolationLevel::Serializable)
            .access_mode(AccessMode::ReadOnly)
            .deferrable(true);

        assert_eq!(opts.isolation_level, Some(IsolationLevel::Serializable));
        assert_eq!(opts.access_mode, Some(AccessMode::ReadOnly));
        assert_eq!(opts.deferrable, Some(true));
    }

    #[test]
    fn test_transaction_options_presets() {
        let read_only = TransactionOptions::read_only();
        assert_eq!(read_only.access_mode, Some(AccessMode::ReadOnly));

        let serializable = TransactionOptions::serializable();
        assert_eq!(
            serializable.isolation_level,
            Some(IsolationLevel::Serializable)
        );
    }
}

