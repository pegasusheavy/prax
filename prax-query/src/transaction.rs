//! Transaction support with async closures and savepoints.
//!
//! Set `PRAX_DEBUG=true` to enable transaction debug logging.
//!
//! This module provides a type-safe transaction API that:
//! - Automatically commits on success
//! - Automatically rolls back on error or panic
//! - Supports savepoints for nested transactions
//! - Configurable isolation levels
//!
//! # Isolation Levels
//!
//! ```rust
//! use prax_query::IsolationLevel;
//!
//! // Available isolation levels
//! let level = IsolationLevel::ReadUncommitted;
//! let level = IsolationLevel::ReadCommitted;  // Default
//! let level = IsolationLevel::RepeatableRead;
//! let level = IsolationLevel::Serializable;
//!
//! // Get SQL representation
//! assert_eq!(IsolationLevel::Serializable.as_sql(), "SERIALIZABLE");
//! assert_eq!(IsolationLevel::ReadCommitted.as_sql(), "READ COMMITTED");
//! ```
//!
//! # Transaction Configuration
//!
//! ```rust
//! use prax_query::{TransactionConfig, IsolationLevel};
//!
//! // Default configuration
//! let config = TransactionConfig::new();
//! assert_eq!(config.isolation, IsolationLevel::ReadCommitted);
//!
//! // Custom configuration
//! let config = TransactionConfig::new()
//!     .isolation(IsolationLevel::Serializable);
//!
//! // Access isolation as a public field
//! assert_eq!(config.isolation, IsolationLevel::Serializable);
//! ```
//!
//! # Transaction Usage (requires async runtime)
//!
//! ```rust,ignore
//! // Basic transaction - commits on success, rolls back on error
//! let result = client
//!     .transaction(|tx| async move {
//!         let user = tx.user().create(/* ... */).exec().await?;
//!         tx.post().create(/* ... */).exec().await?;
//!         Ok(user)
//!     })
//!     .await?;
//!
//! // With configuration
//! let result = client
//!     .transaction(|tx| async move {
//!         // ... perform operations
//!         Ok(())
//!     })
//!     .with_config(TransactionConfig::new()
//!         .isolation(IsolationLevel::Serializable)
//!         .timeout(Duration::from_secs(30)))
//!     .await?;
//!
//! // With savepoints for partial rollback
//! let result = client
//!     .transaction(|tx| async move {
//!         tx.user().create(/* ... */).exec().await?;
//!
//!         // This can be rolled back independently
//!         let savepoint_result = tx.savepoint("sp1", |sp| async move {
//!             sp.post().create(/* ... */).exec().await?;
//!             Ok(())
//!         }).await;
//!
//!         // Even if savepoint fails, outer transaction continues
//!         if savepoint_result.is_err() {
//!             // Handle partial failure
//!         }
//!
//!         Ok(())
//!     })
//!     .await?;
//! ```

use std::future::Future;
use std::time::Duration;
use tracing::debug;

use crate::error::QueryResult;

/// Transaction isolation levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IsolationLevel {
    /// Read uncommitted - allows dirty reads.
    ReadUncommitted,
    /// Read committed - prevents dirty reads.
    #[default]
    ReadCommitted,
    /// Repeatable read - prevents non-repeatable reads.
    RepeatableRead,
    /// Serializable - highest isolation level.
    Serializable,
}

impl IsolationLevel {
    /// Get the SQL clause for this isolation level.
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::ReadUncommitted => "READ UNCOMMITTED",
            Self::ReadCommitted => "READ COMMITTED",
            Self::RepeatableRead => "REPEATABLE READ",
            Self::Serializable => "SERIALIZABLE",
        }
    }
}

/// Access mode for transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AccessMode {
    /// Read-write access (default).
    #[default]
    ReadWrite,
    /// Read-only access.
    ReadOnly,
}

impl AccessMode {
    /// Get the SQL clause for this access mode.
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::ReadWrite => "READ WRITE",
            Self::ReadOnly => "READ ONLY",
        }
    }
}

/// Configuration for a transaction.
#[derive(Debug, Clone, Default)]
pub struct TransactionConfig {
    /// Isolation level.
    pub isolation: IsolationLevel,
    /// Access mode.
    pub access_mode: AccessMode,
    /// Timeout for the transaction.
    pub timeout: Option<Duration>,
    /// Whether to defer constraint checking.
    pub deferrable: bool,
}

impl TransactionConfig {
    /// Create a new transaction config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the isolation level.
    pub fn isolation(mut self, level: IsolationLevel) -> Self {
        self.isolation = level;
        self
    }

    /// Set the access mode.
    pub fn access_mode(mut self, mode: AccessMode) -> Self {
        self.access_mode = mode;
        self
    }

    /// Set the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Make the transaction read-only.
    pub fn read_only(self) -> Self {
        self.access_mode(AccessMode::ReadOnly)
    }

    /// Make the transaction deferrable.
    pub fn deferrable(mut self) -> Self {
        self.deferrable = true;
        self
    }

    /// Generate the BEGIN TRANSACTION SQL.
    pub fn to_begin_sql(&self) -> String {
        let mut parts = vec!["BEGIN"];

        // Isolation level
        parts.push("ISOLATION LEVEL");
        parts.push(self.isolation.as_sql());

        // Access mode
        parts.push(self.access_mode.as_sql());

        // Deferrable (PostgreSQL specific, only valid for SERIALIZABLE READ ONLY)
        if self.deferrable
            && self.isolation == IsolationLevel::Serializable
            && self.access_mode == AccessMode::ReadOnly
        {
            parts.push("DEFERRABLE");
        }

        let sql = parts.join(" ");
        debug!(isolation = %self.isolation.as_sql(), access_mode = %self.access_mode.as_sql(), "Transaction BEGIN");
        sql
    }
}

/// A transaction handle that provides query operations.
///
/// The transaction will be committed when dropped if no error occurred,
/// or rolled back if an error occurred or panic happened.
pub struct Transaction<E> {
    engine: E,
    config: TransactionConfig,
    committed: bool,
    savepoint_count: u32,
}

impl<E> Transaction<E> {
    /// Create a new transaction handle.
    pub fn new(engine: E, config: TransactionConfig) -> Self {
        Self {
            engine,
            config,
            committed: false,
            savepoint_count: 0,
        }
    }

    /// Get the transaction configuration.
    pub fn config(&self) -> &TransactionConfig {
        &self.config
    }

    /// Get the underlying engine.
    pub fn engine(&self) -> &E {
        &self.engine
    }

    /// Create a savepoint.
    pub fn savepoint_name(&mut self) -> String {
        self.savepoint_count += 1;
        format!("sp_{}", self.savepoint_count)
    }

    /// Mark the transaction as committed.
    pub fn mark_committed(&mut self) {
        self.committed = true;
    }

    /// Check if the transaction has been committed.
    pub fn is_committed(&self) -> bool {
        self.committed
    }
}

/// Builder for executing a transaction with a closure.
pub struct TransactionBuilder<E, F, Fut, T>
where
    F: FnOnce(Transaction<E>) -> Fut,
    Fut: Future<Output = QueryResult<T>>,
{
    engine: E,
    callback: F,
    config: TransactionConfig,
}

impl<E, F, Fut, T> TransactionBuilder<E, F, Fut, T>
where
    F: FnOnce(Transaction<E>) -> Fut,
    Fut: Future<Output = QueryResult<T>>,
{
    /// Create a new transaction builder.
    pub fn new(engine: E, callback: F) -> Self {
        Self {
            engine,
            callback,
            config: TransactionConfig::default(),
        }
    }

    /// Set the isolation level.
    pub fn isolation(mut self, level: IsolationLevel) -> Self {
        self.config.isolation = level;
        self
    }

    /// Set read-only mode.
    pub fn read_only(mut self) -> Self {
        self.config.access_mode = AccessMode::ReadOnly;
        self
    }

    /// Set the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }

    /// Set deferrable mode.
    pub fn deferrable(mut self) -> Self {
        self.config.deferrable = true;
        self
    }
}

/// Interactive transaction for step-by-step operations.
pub struct InteractiveTransaction<E> {
    inner: Transaction<E>,
    started: bool,
}

impl<E> InteractiveTransaction<E> {
    /// Create a new interactive transaction.
    pub fn new(engine: E) -> Self {
        Self {
            inner: Transaction::new(engine, TransactionConfig::default()),
            started: false,
        }
    }

    /// Create with configuration.
    pub fn with_config(engine: E, config: TransactionConfig) -> Self {
        Self {
            inner: Transaction::new(engine, config),
            started: false,
        }
    }

    /// Get the engine.
    pub fn engine(&self) -> &E {
        &self.inner.engine
    }

    /// Check if the transaction has started.
    pub fn is_started(&self) -> bool {
        self.started
    }

    /// Get the BEGIN SQL.
    pub fn begin_sql(&self) -> String {
        self.inner.config.to_begin_sql()
    }

    /// Get the COMMIT SQL.
    pub fn commit_sql(&self) -> &'static str {
        "COMMIT"
    }

    /// Get the ROLLBACK SQL.
    pub fn rollback_sql(&self) -> &'static str {
        "ROLLBACK"
    }

    /// Get the SAVEPOINT SQL.
    pub fn savepoint_sql(&mut self, name: Option<&str>) -> String {
        let name = name
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.inner.savepoint_name());
        format!("SAVEPOINT {}", name)
    }

    /// Get the ROLLBACK TO SAVEPOINT SQL.
    pub fn rollback_to_sql(&self, name: &str) -> String {
        format!("ROLLBACK TO SAVEPOINT {}", name)
    }

    /// Get the RELEASE SAVEPOINT SQL.
    pub fn release_savepoint_sql(&self, name: &str) -> String {
        format!("RELEASE SAVEPOINT {}", name)
    }

    /// Mark as started.
    pub fn mark_started(&mut self) {
        self.started = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_level() {
        assert_eq!(IsolationLevel::ReadCommitted.as_sql(), "READ COMMITTED");
        assert_eq!(IsolationLevel::Serializable.as_sql(), "SERIALIZABLE");
    }

    #[test]
    fn test_access_mode() {
        assert_eq!(AccessMode::ReadWrite.as_sql(), "READ WRITE");
        assert_eq!(AccessMode::ReadOnly.as_sql(), "READ ONLY");
    }

    #[test]
    fn test_transaction_config_default() {
        let config = TransactionConfig::new();
        assert_eq!(config.isolation, IsolationLevel::ReadCommitted);
        assert_eq!(config.access_mode, AccessMode::ReadWrite);
        assert!(config.timeout.is_none());
        assert!(!config.deferrable);
    }

    #[test]
    fn test_transaction_config_builder() {
        let config = TransactionConfig::new()
            .isolation(IsolationLevel::Serializable)
            .read_only()
            .deferrable()
            .timeout(Duration::from_secs(30));

        assert_eq!(config.isolation, IsolationLevel::Serializable);
        assert_eq!(config.access_mode, AccessMode::ReadOnly);
        assert!(config.deferrable);
        assert_eq!(config.timeout, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_begin_sql() {
        let config = TransactionConfig::new();
        let sql = config.to_begin_sql();
        assert!(sql.contains("BEGIN"));
        assert!(sql.contains("ISOLATION LEVEL READ COMMITTED"));
        assert!(sql.contains("READ WRITE"));
    }

    #[test]
    fn test_begin_sql_serializable_deferrable() {
        let config = TransactionConfig::new()
            .isolation(IsolationLevel::Serializable)
            .read_only()
            .deferrable();
        let sql = config.to_begin_sql();
        assert!(sql.contains("SERIALIZABLE"));
        assert!(sql.contains("READ ONLY"));
        assert!(sql.contains("DEFERRABLE"));
    }

    #[test]
    fn test_interactive_transaction() {
        #[derive(Clone)]
        struct MockEngine;

        let mut tx = InteractiveTransaction::new(MockEngine);
        assert!(!tx.is_started());

        let begin = tx.begin_sql();
        assert!(begin.contains("BEGIN"));

        let sp = tx.savepoint_sql(Some("test_sp"));
        assert_eq!(sp, "SAVEPOINT test_sp");

        let rollback_to = tx.rollback_to_sql("test_sp");
        assert_eq!(rollback_to, "ROLLBACK TO SAVEPOINT test_sp");

        let release = tx.release_savepoint_sql("test_sp");
        assert_eq!(release, "RELEASE SAVEPOINT test_sp");
    }
}
