//! SQLx configuration for database connections.

use crate::error::{SqlxError, SqlxResult};
use std::time::Duration;

/// Database backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBackend {
    /// PostgreSQL database
    Postgres,
    /// MySQL database
    MySql,
    /// SQLite database
    Sqlite,
}

impl DatabaseBackend {
    /// Parse backend from URL scheme.
    pub fn from_url(url: &str) -> SqlxResult<Self> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(Self::Postgres)
        } else if url.starts_with("mysql://") || url.starts_with("mariadb://") {
            Ok(Self::MySql)
        } else if url.starts_with("sqlite://") || url.starts_with("file:") {
            Ok(Self::Sqlite)
        } else {
            Err(SqlxError::config(
                "Unknown database URL scheme. Expected postgres://, mysql://, or sqlite://",
            ))
        }
    }
}

/// SQLx pool configuration.
#[derive(Debug, Clone)]
pub struct SqlxConfig {
    /// Database connection URL
    pub url: String,
    /// Database backend type (auto-detected from URL)
    pub backend: DatabaseBackend,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections to keep idle
    pub min_connections: u32,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Idle connection timeout
    pub idle_timeout: Option<Duration>,
    /// Maximum lifetime of a connection
    pub max_lifetime: Option<Duration>,
    /// Enable statement caching
    pub statement_cache_capacity: usize,
    /// Enable SSL/TLS
    pub ssl_mode: SslMode,
    /// Application name (for PostgreSQL)
    pub application_name: Option<String>,
}

/// SSL mode for connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SslMode {
    /// Disable SSL
    Disable,
    /// Prefer SSL but allow non-SSL
    #[default]
    Prefer,
    /// Require SSL
    Require,
    /// Require SSL and verify server certificate
    VerifyCa,
    /// Require SSL and verify server certificate and hostname
    VerifyFull,
}

impl Default for SqlxConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            backend: DatabaseBackend::Postgres,
            max_connections: 10,
            min_connections: 1,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
            statement_cache_capacity: 100,
            ssl_mode: SslMode::default(),
            application_name: None,
        }
    }
}

impl SqlxConfig {
    /// Create a new configuration from a database URL.
    ///
    /// # Example
    ///
    /// ```rust
    /// use prax_sqlx::SqlxConfig;
    ///
    /// let config = SqlxConfig::from_url("postgres://user:pass@localhost/mydb").unwrap();
    /// assert_eq!(config.max_connections, 10);
    /// ```
    pub fn from_url(url: impl Into<String>) -> SqlxResult<Self> {
        let url = url.into();
        let backend = DatabaseBackend::from_url(&url)?;

        Ok(Self {
            url,
            backend,
            ..Default::default()
        })
    }

    /// Create a builder for more detailed configuration.
    pub fn builder(url: impl Into<String>) -> SqlxConfigBuilder {
        SqlxConfigBuilder::new(url)
    }

    /// Set max connections.
    pub fn with_max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Set min connections.
    pub fn with_min_connections(mut self, min: u32) -> Self {
        self.min_connections = min;
        self
    }

    /// Set connection timeout.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set idle timeout.
    pub fn with_idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = Some(timeout);
        self
    }

    /// Set max lifetime.
    pub fn with_max_lifetime(mut self, lifetime: Duration) -> Self {
        self.max_lifetime = Some(lifetime);
        self
    }

    /// Set statement cache capacity.
    pub fn with_statement_cache(mut self, capacity: usize) -> Self {
        self.statement_cache_capacity = capacity;
        self
    }

    /// Set SSL mode.
    pub fn with_ssl_mode(mut self, mode: SslMode) -> Self {
        self.ssl_mode = mode;
        self
    }

    /// Set application name.
    pub fn with_application_name(mut self, name: impl Into<String>) -> Self {
        self.application_name = Some(name.into());
        self
    }
}

/// Builder for SQLx configuration.
pub struct SqlxConfigBuilder {
    config: SqlxConfig,
}

impl SqlxConfigBuilder {
    /// Create a new builder with a database URL.
    pub fn new(url: impl Into<String>) -> Self {
        let url = url.into();
        let backend = DatabaseBackend::from_url(&url).unwrap_or(DatabaseBackend::Postgres);

        Self {
            config: SqlxConfig {
                url,
                backend,
                ..Default::default()
            },
        }
    }

    /// Set max connections.
    pub fn max_connections(mut self, max: u32) -> Self {
        self.config.max_connections = max;
        self
    }

    /// Set min connections.
    pub fn min_connections(mut self, min: u32) -> Self {
        self.config.min_connections = min;
        self
    }

    /// Set connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    /// Set idle timeout.
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.config.idle_timeout = Some(timeout);
        self
    }

    /// Disable idle timeout.
    pub fn no_idle_timeout(mut self) -> Self {
        self.config.idle_timeout = None;
        self
    }

    /// Set max lifetime.
    pub fn max_lifetime(mut self, lifetime: Duration) -> Self {
        self.config.max_lifetime = Some(lifetime);
        self
    }

    /// Disable max lifetime.
    pub fn no_max_lifetime(mut self) -> Self {
        self.config.max_lifetime = None;
        self
    }

    /// Set statement cache capacity.
    pub fn statement_cache(mut self, capacity: usize) -> Self {
        self.config.statement_cache_capacity = capacity;
        self
    }

    /// Set SSL mode.
    pub fn ssl_mode(mut self, mode: SslMode) -> Self {
        self.config.ssl_mode = mode;
        self
    }

    /// Require SSL.
    pub fn require_ssl(mut self) -> Self {
        self.config.ssl_mode = SslMode::Require;
        self
    }

    /// Set application name.
    pub fn application_name(mut self, name: impl Into<String>) -> Self {
        self.config.application_name = Some(name.into());
        self
    }

    /// Build the configuration.
    pub fn build(self) -> SqlxConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_detection() {
        assert_eq!(
            DatabaseBackend::from_url("postgres://localhost/db").unwrap(),
            DatabaseBackend::Postgres
        );
        assert_eq!(
            DatabaseBackend::from_url("postgresql://localhost/db").unwrap(),
            DatabaseBackend::Postgres
        );
        assert_eq!(
            DatabaseBackend::from_url("mysql://localhost/db").unwrap(),
            DatabaseBackend::MySql
        );
        assert_eq!(
            DatabaseBackend::from_url("sqlite://./test.db").unwrap(),
            DatabaseBackend::Sqlite
        );
        assert_eq!(
            DatabaseBackend::from_url("file:./test.db").unwrap(),
            DatabaseBackend::Sqlite
        );

        assert!(DatabaseBackend::from_url("unknown://localhost").is_err());
    }

    #[test]
    fn test_config_from_url() {
        let config = SqlxConfig::from_url("postgres://user:pass@localhost:5432/mydb").unwrap();
        assert_eq!(config.backend, DatabaseBackend::Postgres);
        assert_eq!(config.max_connections, 10);
    }

    #[test]
    fn test_config_builder() {
        let config = SqlxConfig::builder("postgres://localhost/db")
            .max_connections(20)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(10))
            .require_ssl()
            .application_name("prax-app")
            .build();

        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.ssl_mode, SslMode::Require);
        assert_eq!(config.application_name, Some("prax-app".to_string()));
    }

    #[test]
    fn test_config_with_methods() {
        let config = SqlxConfig::from_url("mysql://localhost/db")
            .unwrap()
            .with_max_connections(50)
            .with_statement_cache(200);

        assert_eq!(config.max_connections, 50);
        assert_eq!(config.statement_cache_capacity, 200);
    }
}
