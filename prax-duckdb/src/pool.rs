//! DuckDB connection pool.
//!
//! DuckDB supports concurrent access within a single process through
//! connection pooling. This module provides a simple connection pool
//! that manages multiple connections to the same database.
//!
//! # In-memory databases
//!
//! Each `Connection::open_in_memory()` opens a separate, *isolated*
//! database, so pooling multiple connections to `:memory:` would give
//! every checkout its own private database (writes through one pooled
//! connection would be invisible to reads through another). To avoid
//! this split-brain, pools built with an in-memory config are forced to
//! a single shared connection: `min_connections` and `max_connections`
//! are clamped to 1, and a concurrent second checkout waits for the
//! connection to be returned instead of opening a new isolated database.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use parking_lot::Mutex;
use tokio::sync::Semaphore;
use tracing::{debug, info};

use crate::config::DuckDbConfig;
use crate::connection::DuckDbConnection;
use crate::error::{DuckDbError, DuckDbResult};

/// Pool configuration.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections.
    pub max_connections: usize,
    /// Minimum number of connections to keep open.
    pub min_connections: usize,
    /// Connection timeout in milliseconds.
    pub connection_timeout_ms: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connection_timeout_ms: 30_000,
        }
    }
}

/// A DuckDB connection pool.
///
/// Manages multiple connections to a DuckDB database for concurrent access.
#[derive(Clone)]
pub struct DuckDbPool {
    /// Database configuration.
    config: Arc<DuckDbConfig>,
    /// Pool configuration.
    pool_config: Arc<PoolConfig>,
    /// Available connections.
    connections: Arc<Mutex<Vec<DuckDbConnection>>>,
    /// Semaphore to limit concurrent connections.
    semaphore: Arc<Semaphore>,
}

impl DuckDbPool {
    /// Create a new connection pool.
    pub async fn new(config: DuckDbConfig) -> DuckDbResult<Self> {
        Self::with_pool_config(config, PoolConfig::default()).await
    }

    /// Create a new connection pool with custom pool configuration.
    ///
    /// For in-memory databases the pool is forced to a single shared
    /// connection (see module-level docs): `min_connections` and
    /// `max_connections` are clamped to 1 regardless of the provided
    /// values, so every checkout sees the same database and concurrent
    /// checkouts serialize on the one connection.
    pub async fn with_pool_config(
        config: DuckDbConfig,
        pool_config: PoolConfig,
    ) -> DuckDbResult<Self> {
        // In-memory DuckDB databases are per-connection: each
        // `Connection::open_in_memory()` is a separate, isolated database.
        // Clamp to a single connection so all checkouts share one database.
        // With one semaphore permit, a concurrent second `get` waits for
        // the connection to be returned rather than creating a new
        // isolated in-memory database.
        let pool_config = if config.is_in_memory() {
            PoolConfig {
                max_connections: 1,
                min_connections: 1,
                ..pool_config
            }
        } else {
            pool_config
        };

        info!(
            max_connections = pool_config.max_connections,
            min_connections = pool_config.min_connections,
            "Creating DuckDB connection pool"
        );

        let pool = Self {
            config: Arc::new(config),
            pool_config: Arc::new(pool_config.clone()),
            connections: Arc::new(Mutex::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(pool_config.max_connections)),
        };

        // Pre-create minimum connections
        for _ in 0..pool_config.min_connections {
            let conn = pool.create_connection()?;
            pool.connections.lock().push(conn);
        }

        Ok(pool)
    }

    /// Create a builder for the pool.
    pub fn builder() -> DuckDbPoolBuilder {
        DuckDbPoolBuilder::default()
    }

    /// Get a connection from the pool.
    ///
    /// Waits at most `connection_timeout_ms` for a permit when the pool
    /// is saturated — notably the single shared connection of an
    /// in-memory pool — and fails with [`DuckDbError::Timeout`] instead
    /// of blocking forever behind a long-held checkout.
    pub async fn get(&self) -> DuckDbResult<PooledConnection> {
        debug!("Acquiring connection from pool");

        // Acquire permit, honoring the configured connection timeout so a
        // saturated pool (e.g. one long-held checkout on a
        // single-connection in-memory pool) fails fast instead of
        // deadlocking every later checkout.
        let timeout = Duration::from_millis(self.pool_config.connection_timeout_ms);
        let permit = tokio::time::timeout(timeout, self.semaphore.clone().acquire_owned())
            .await
            .map_err(|_| {
                DuckDbError::timeout(format!(
                    "timed out after {:?} waiting for a connection from the pool",
                    timeout
                ))
            })?
            .map_err(|e| DuckDbError::pool(format!("Failed to acquire semaphore: {}", e)))?;

        // Try to get an existing connection
        let conn = {
            let mut connections = self.connections.lock();
            connections.pop()
        };

        let conn = match conn {
            Some(c) => c,
            None => self.create_connection()?,
        };

        Ok(PooledConnection {
            conn: Some(conn),
            pool: self.clone(),
            poisoned: AtomicBool::new(false),
            _permit: permit,
        })
    }

    /// Create a new connection.
    fn create_connection(&self) -> DuckDbResult<DuckDbConnection> {
        debug!("Creating new DuckDB connection");
        DuckDbConnection::new(&self.config)
    }

    /// Return a connection to the pool.
    fn return_connection(&self, conn: DuckDbConnection) {
        let mut connections = self.connections.lock();
        if connections.len() < self.pool_config.max_connections {
            connections.push(conn);
        }
        // If pool is full, connection is dropped
    }

    /// Get pool status.
    pub fn status(&self) -> PoolStatus {
        let available = self.connections.lock().len();
        let permits = self.semaphore.available_permits();

        PoolStatus {
            max_connections: self.pool_config.max_connections,
            available_connections: available,
            available_permits: permits,
            in_use: self.pool_config.max_connections - permits,
        }
    }

    /// Get a reference to the database configuration.
    pub fn config(&self) -> &DuckDbConfig {
        &self.config
    }

    /// Get a reference to the pool configuration.
    pub fn pool_config(&self) -> &PoolConfig {
        &self.pool_config
    }
}

impl std::fmt::Debug for DuckDbPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DuckDbPool")
            .field("status", &self.status())
            .finish()
    }
}

/// Pool status information.
#[derive(Debug, Clone)]
pub struct PoolStatus {
    /// Maximum connections in the pool.
    pub max_connections: usize,
    /// Available connections in the pool.
    pub available_connections: usize,
    /// Available permits.
    pub available_permits: usize,
    /// Connections currently in use.
    pub in_use: usize,
}

/// A connection borrowed from the pool.
///
/// When dropped, the connection is returned to the pool.
pub struct PooledConnection {
    conn: Option<DuckDbConnection>,
    pool: DuckDbPool,
    /// Set when the connection's session state is no longer trustworthy
    /// (e.g. a transaction rollback failed, leaving a possibly-open
    /// transaction behind). Poisoned connections are dropped on release
    /// instead of returning to the idle pool.
    poisoned: AtomicBool,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl PooledConnection {
    /// Get a reference to the underlying connection.
    pub fn connection(&self) -> &DuckDbConnection {
        self.conn.as_ref().expect("Connection already taken")
    }

    /// Mark the connection as unfit for reuse. A poisoned connection is
    /// dropped when this guard is released instead of being recycled
    /// into the idle pool; the semaphore permit still frees, so the pool
    /// opens a fresh connection on the next checkout.
    pub(crate) fn poison(&self) {
        self.poisoned.store(true, Ordering::Release);
    }

    /// Query and return all rows as JSON.
    pub async fn query(
        &self,
        sql: &str,
        params: &[prax_query::filter::FilterValue],
    ) -> DuckDbResult<Vec<serde_json::Value>> {
        let conn = self.connection().clone();
        let sql = sql.to_string();
        let params = params.to_vec();

        tokio::task::spawn_blocking(move || conn.query(&sql, &params))
            .await
            .map_err(|e| DuckDbError::internal(format!("Task join error: {}", e)))?
    }

    /// Query and return the first row.
    pub async fn query_one(
        &self,
        sql: &str,
        params: &[prax_query::filter::FilterValue],
    ) -> DuckDbResult<serde_json::Value> {
        let conn = self.connection().clone();
        let sql = sql.to_string();
        let params = params.to_vec();

        tokio::task::spawn_blocking(move || conn.query_one(&sql, &params))
            .await
            .map_err(|e| DuckDbError::internal(format!("Task join error: {}", e)))?
    }

    /// Query and return the first row or None.
    pub async fn query_optional(
        &self,
        sql: &str,
        params: &[prax_query::filter::FilterValue],
    ) -> DuckDbResult<Option<serde_json::Value>> {
        let conn = self.connection().clone();
        let sql = sql.to_string();
        let params = params.to_vec();

        tokio::task::spawn_blocking(move || conn.query_optional(&sql, &params))
            .await
            .map_err(|e| DuckDbError::internal(format!("Task join error: {}", e)))?
    }

    /// Query and return typed row snapshots. Drives the synchronous
    /// `query_rows` on the inner connection from a `spawn_blocking`
    /// so the caller's runtime isn't stalled on DuckDB's blocking API.
    pub async fn query_rows(
        &self,
        sql: &str,
        params: &[prax_query::filter::FilterValue],
    ) -> DuckDbResult<Vec<crate::row_ref::DuckDbRowRef>> {
        let conn = self.connection().clone();
        let sql = sql.to_string();
        let params = params.to_vec();
        tokio::task::spawn_blocking(move || conn.query_rows(&sql, &params))
            .await
            .map_err(|e| DuckDbError::internal(format!("Task join error: {}", e)))?
    }

    /// Execute a statement and return affected rows.
    pub async fn execute(
        &self,
        sql: &str,
        params: &[prax_query::filter::FilterValue],
    ) -> DuckDbResult<usize> {
        let conn = self.connection().clone();
        let sql = sql.to_string();
        let params = params.to_vec();

        tokio::task::spawn_blocking(move || conn.execute(&sql, &params))
            .await
            .map_err(|e| DuckDbError::internal(format!("Task join error: {}", e)))?
    }

    /// Execute a batch of SQL statements.
    pub async fn execute_batch(&self, sql: &str) -> DuckDbResult<()> {
        let conn = self.connection().clone();
        let sql = sql.to_string();

        tokio::task::spawn_blocking(move || conn.execute_batch(&sql))
            .await
            .map_err(|e| DuckDbError::internal(format!("Task join error: {}", e)))?
    }

    /// Copy data to Parquet.
    pub async fn copy_to_parquet(&self, query: &str, path: &str) -> DuckDbResult<()> {
        let conn = self.connection().clone();
        let query = query.to_string();
        let path = path.to_string();

        tokio::task::spawn_blocking(move || conn.copy_to_parquet(&query, &path))
            .await
            .map_err(|e| DuckDbError::internal(format!("Task join error: {}", e)))?
    }

    /// Copy data to CSV.
    pub async fn copy_to_csv(&self, query: &str, path: &str, header: bool) -> DuckDbResult<()> {
        let conn = self.connection().clone();
        let query = query.to_string();
        let path = path.to_string();

        tokio::task::spawn_blocking(move || conn.copy_to_csv(&query, &path, header))
            .await
            .map_err(|e| DuckDbError::internal(format!("Task join error: {}", e)))?
    }

    /// Query a Parquet file.
    pub async fn query_parquet(&self, path: &str) -> DuckDbResult<Vec<serde_json::Value>> {
        let conn = self.connection().clone();
        let path = path.to_string();

        tokio::task::spawn_blocking(move || conn.query_parquet(&path))
            .await
            .map_err(|e| DuckDbError::internal(format!("Task join error: {}", e)))?
    }

    /// Query a CSV file.
    pub async fn query_csv(
        &self,
        path: &str,
        header: bool,
    ) -> DuckDbResult<Vec<serde_json::Value>> {
        let conn = self.connection().clone();
        let path = path.to_string();

        tokio::task::spawn_blocking(move || conn.query_csv(&path, header))
            .await
            .map_err(|e| DuckDbError::internal(format!("Task join error: {}", e)))?
    }

    /// Query a JSON file.
    pub async fn query_json(&self, path: &str) -> DuckDbResult<Vec<serde_json::Value>> {
        let conn = self.connection().clone();
        let path = path.to_string();

        tokio::task::spawn_blocking(move || conn.query_json(&path))
            .await
            .map_err(|e| DuckDbError::internal(format!("Task join error: {}", e)))?
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            // Poisoned connections (e.g. a failed rollback left the
            // transaction state unknown) are dropped here rather than
            // recycled, so the next checkout can't inherit dirty
            // session state.
            if !self.poisoned.load(Ordering::Acquire) {
                self.pool.return_connection(conn);
            }
        }
    }
}

impl std::fmt::Debug for PooledConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledConnection").finish_non_exhaustive()
    }
}

/// Builder for DuckDB connection pool.
#[derive(Debug, Default)]
pub struct DuckDbPoolBuilder {
    config: Option<DuckDbResult<DuckDbConfig>>,
    pool_config: PoolConfig,
}

impl DuckDbPoolBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the database configuration.
    pub fn config(mut self, config: DuckDbConfig) -> Self {
        self.config = Some(Ok(config));
        self
    }

    /// Set the database path.
    ///
    /// Path errors (e.g. an uncreatable parent directory) are stored and
    /// surfaced from [`build`](Self::build) rather than silently falling
    /// back to an in-memory database.
    pub fn path(mut self, path: &str) -> Self {
        self.config = Some(DuckDbConfig::from_path(path));
        self
    }

    /// Use an in-memory database.
    ///
    /// In-memory pools always use a single shared connection
    /// (`min_connections`/`max_connections` are clamped to 1 at build
    /// time) because each in-memory DuckDB connection is a separate,
    /// isolated database.
    pub fn in_memory(mut self) -> Self {
        self.config = Some(Ok(DuckDbConfig::in_memory()));
        self
    }

    /// Set the database URL.
    ///
    /// URL parse errors are stored and surfaced from
    /// [`build`](Self::build).
    pub fn url(mut self, url: &str) -> Self {
        self.config = Some(DuckDbConfig::from_url(url));
        self
    }

    /// Set maximum connections.
    pub fn max_connections(mut self, max: usize) -> Self {
        self.pool_config.max_connections = max;
        self
    }

    /// Set minimum connections.
    pub fn min_connections(mut self, min: usize) -> Self {
        self.pool_config.min_connections = min;
        self
    }

    /// Set connection timeout in milliseconds.
    pub fn connection_timeout_ms(mut self, timeout: u64) -> Self {
        self.pool_config.connection_timeout_ms = timeout;
        self
    }

    /// Build the pool.
    ///
    /// Returns an error if no database configuration was provided, or if
    /// the configuration source (e.g. [`path`](Self::path) or
    /// [`url`](Self::url)) failed to produce one.
    pub async fn build(self) -> DuckDbResult<DuckDbPool> {
        let config = self
            .config
            .ok_or_else(|| DuckDbError::config("Database configuration required"))??;

        DuckDbPool::with_pool_config(config, self.pool_config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_creation() {
        let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await.unwrap();
        let status = pool.status();
        // In-memory pools are clamped to a single shared connection.
        assert_eq!(status.max_connections, 1);
        assert!(status.available_connections >= 1);
    }

    #[tokio::test]
    async fn test_pool_get_connection() {
        let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await.unwrap();
        let conn = pool.get().await.unwrap();

        // Execute a simple query
        let results = conn.query("SELECT 1 as value", &[]).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_pool_builder() {
        let pool = DuckDbPool::builder()
            .in_memory()
            .max_connections(5)
            .min_connections(2)
            .build()
            .await
            .unwrap();

        // In-memory pools are clamped to a single shared connection, so
        // the requested max(5)/min(2) are overridden.
        let status = pool.status();
        assert_eq!(status.max_connections, 1);
        assert_eq!(status.available_connections, 1);
    }

    #[tokio::test]
    async fn test_in_memory_pool_shares_single_database() {
        let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await.unwrap();

        // First checkout: create a table and write a row.
        {
            let conn = pool.get().await.unwrap();
            conn.execute("CREATE TABLE shared_writes (value INTEGER)", &[])
                .await
                .unwrap();
            conn.execute("INSERT INTO shared_writes VALUES (42)", &[])
                .await
                .unwrap();
        }

        // Second checkout: must see the first checkout's writes, proving
        // both checkouts used the same underlying in-memory database.
        {
            let conn = pool.get().await.unwrap();
            let rows = conn
                .query("SELECT value FROM shared_writes", &[])
                .await
                .unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0]["value"], serde_json::json!(42));
        }
    }

    #[tokio::test]
    async fn test_connection_returned_to_pool() {
        let pool = DuckDbPool::builder()
            .in_memory()
            .max_connections(2)
            .min_connections(0)
            .build()
            .await
            .unwrap();

        let initial_permits = pool.semaphore.available_permits();

        {
            let _conn = pool.get().await.unwrap();
            assert_eq!(pool.semaphore.available_permits(), initial_permits - 1);
        }

        // Connection should be returned
        assert_eq!(pool.semaphore.available_permits(), initial_permits);
    }

    #[tokio::test]
    async fn test_pool_get_times_out_when_saturated() {
        let pool = DuckDbPool::builder()
            .in_memory()
            .connection_timeout_ms(50)
            .build()
            .await
            .unwrap();

        // In-memory pools are clamped to a single shared connection, so
        // holding one checkout saturates the pool; the next get must
        // time out instead of blocking forever.
        let held = pool.get().await.unwrap();

        let err = pool.get().await.unwrap_err();
        assert!(
            matches!(err, DuckDbError::Timeout(_)),
            "expected a timeout error, got: {err:?}"
        );
        assert!(err.to_string().contains("timed out after 50ms"));

        // Releasing the held checkout frees the permit again.
        drop(held);
        pool.get().await.unwrap();
    }
}
