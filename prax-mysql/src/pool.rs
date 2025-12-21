//! Connection pool for MySQL.

use std::sync::Arc;
use std::time::Duration;

use mysql_async::{Opts, Pool};
use tracing::{debug, info};

use crate::config::MysqlConfig;
use crate::connection::MysqlConnection;
use crate::error::{MysqlError, MysqlResult};

/// A connection pool for MySQL.
#[derive(Clone)]
pub struct MysqlPool {
    inner: Pool,
    config: Arc<MysqlConfig>,
}

impl MysqlPool {
    /// Create a new connection pool from configuration.
    pub async fn new(config: MysqlConfig) -> MysqlResult<Self> {
        Self::with_pool_config(config, PoolConfig::default()).await
    }

    /// Create a new connection pool with custom pool configuration.
    pub async fn with_pool_config(
        config: MysqlConfig,
        pool_config: PoolConfig,
    ) -> MysqlResult<Self> {
        let opts = config.to_opts_builder().pool_opts(
            mysql_async::PoolOpts::new().with_constraints(
                mysql_async::PoolConstraints::new(
                    pool_config.min_connections,
                    pool_config.max_connections,
                )
                .unwrap_or_default(),
            ),
        );

        let pool = Pool::new(Opts::from(opts));

        info!(
            host = %config.host,
            port = %config.port,
            database = %config.database,
            max_connections = %pool_config.max_connections,
            "MySQL connection pool created"
        );

        Ok(Self {
            inner: pool,
            config: Arc::new(config),
        })
    }

    /// Get a connection from the pool.
    pub async fn get(&self) -> MysqlResult<MysqlConnection> {
        debug!("Acquiring connection from pool");
        let conn = self.inner.get_conn().await?;
        Ok(MysqlConnection::new(conn))
    }

    /// Get the pool configuration.
    pub fn config(&self) -> &MysqlConfig {
        &self.config
    }

    /// Check if the pool is healthy by attempting to get a connection.
    pub async fn is_healthy(&self) -> bool {
        use mysql_async::prelude::*;
        match self.inner.get_conn().await {
            Ok(mut conn) => conn.query_drop("SELECT 1").await.is_ok(),
            Err(_) => false,
        }
    }

    /// Disconnect all connections and close the pool.
    pub async fn disconnect(self) -> MysqlResult<()> {
        self.inner.disconnect().await?;
        info!("MySQL connection pool closed");
        Ok(())
    }

    /// Create a builder for configuring the pool.
    pub fn builder() -> MysqlPoolBuilder {
        MysqlPoolBuilder::new()
    }
}

/// Configuration for the connection pool.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool.
    pub max_connections: usize,
    /// Minimum number of connections to keep alive.
    pub min_connections: usize,
    /// Maximum time to wait for a connection.
    pub connection_timeout: Option<Duration>,
    /// Maximum idle time before a connection is closed.
    pub idle_timeout: Option<Duration>,
    /// Maximum lifetime of a connection.
    pub max_lifetime: Option<Duration>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Some(Duration::from_secs(30)),
            idle_timeout: Some(Duration::from_secs(600)), // 10 minutes
            max_lifetime: Some(Duration::from_secs(1800)), // 30 minutes
        }
    }
}

/// Builder for creating a connection pool.
#[derive(Debug, Default)]
pub struct MysqlPoolBuilder {
    config: Option<MysqlConfig>,
    url: Option<String>,
    pool_config: PoolConfig,
}

impl MysqlPoolBuilder {
    /// Create a new pool builder.
    pub fn new() -> Self {
        Self {
            config: None,
            url: None,
            pool_config: PoolConfig::default(),
        }
    }

    /// Set the database URL.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the configuration.
    pub fn config(mut self, config: MysqlConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the maximum number of connections.
    pub fn max_connections(mut self, n: usize) -> Self {
        self.pool_config.max_connections = n;
        self
    }

    /// Set the minimum number of connections.
    pub fn min_connections(mut self, n: usize) -> Self {
        self.pool_config.min_connections = n;
        self
    }

    /// Set the connection timeout.
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.pool_config.connection_timeout = Some(timeout);
        self
    }

    /// Set the idle timeout.
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.pool_config.idle_timeout = Some(timeout);
        self
    }

    /// Set the maximum connection lifetime.
    pub fn max_lifetime(mut self, lifetime: Duration) -> Self {
        self.pool_config.max_lifetime = Some(lifetime);
        self
    }

    /// Build the connection pool.
    pub async fn build(self) -> MysqlResult<MysqlPool> {
        let config = if let Some(config) = self.config {
            config
        } else if let Some(url) = self.url {
            MysqlConfig::from_url(url)?
        } else {
            return Err(MysqlError::config("no database URL or config provided"));
        };

        MysqlPool::with_pool_config(config, self.pool_config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
    }

    #[test]
    fn test_pool_builder() {
        let builder = MysqlPoolBuilder::new()
            .url("mysql://localhost/test")
            .max_connections(20);

        assert!(builder.url.is_some());
        assert_eq!(builder.pool_config.max_connections, 20);
    }
}
