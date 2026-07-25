//! Connection pool management for SQLx.

use crate::config::{DatabaseBackend, SqlxConfig, SslMode};
use crate::error::{SqlxError, SqlxResult};

/// A wrapper around SQLx connection pools supporting multiple databases.
#[derive(Clone)]
pub enum SqlxPool {
    /// PostgreSQL connection pool
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
    /// MySQL connection pool
    #[cfg(feature = "mysql")]
    MySql(sqlx::MySqlPool),
    /// SQLite connection pool
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::SqlitePool),
}

/// Translate the crate's [`SslMode`] onto sqlx's `PgSslMode`.
///
/// Pure mapping (no I/O) so every mode can be asserted in unit tests.
#[cfg(feature = "postgres")]
fn pg_ssl_mode(mode: SslMode) -> sqlx::postgres::PgSslMode {
    use sqlx::postgres::PgSslMode;

    match mode {
        SslMode::Disable => PgSslMode::Disable,
        SslMode::Prefer => PgSslMode::Prefer,
        SslMode::Require => PgSslMode::Require,
        SslMode::VerifyCa => PgSslMode::VerifyCa,
        SslMode::VerifyFull => PgSslMode::VerifyFull,
    }
}

/// Translate the crate's [`SslMode`] onto sqlx's `MySqlSslMode`.
///
/// Note the `VerifyFull` → `VerifyIdentity` rename (MySQL's name for
/// hostname verification).
#[cfg(feature = "mysql")]
fn mysql_ssl_mode(mode: SslMode) -> sqlx::mysql::MySqlSslMode {
    use sqlx::mysql::MySqlSslMode;

    match mode {
        SslMode::Disable => MySqlSslMode::Disabled,
        SslMode::Prefer => MySqlSslMode::Preferred,
        SslMode::Require => MySqlSslMode::Required,
        SslMode::VerifyCa => MySqlSslMode::VerifyCa,
        SslMode::VerifyFull => MySqlSslMode::VerifyIdentity,
    }
}

impl SqlxPool {
    /// Create a new pool from configuration.
    pub async fn connect(config: &SqlxConfig) -> SqlxResult<Self> {
        match config.backend {
            #[cfg(feature = "postgres")]
            DatabaseBackend::Postgres => {
                use std::str::FromStr;

                use sqlx::postgres::PgConnectOptions;

                let mut options = PgConnectOptions::from_str(&config.url)?
                    .statement_cache_capacity(config.statement_cache_capacity)
                    .ssl_mode(pg_ssl_mode(config.ssl_mode));
                if let Some(name) = &config.application_name {
                    options = options.application_name(name);
                }

                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(config.max_connections)
                    .min_connections(config.min_connections)
                    .acquire_timeout(config.connect_timeout)
                    .idle_timeout(config.idle_timeout)
                    .max_lifetime(config.max_lifetime)
                    .connect_with(options)
                    .await?;
                Ok(Self::Postgres(pool))
            }
            #[cfg(feature = "mysql")]
            DatabaseBackend::MySql => {
                use std::str::FromStr;

                use sqlx::mysql::MySqlConnectOptions;

                let options = MySqlConnectOptions::from_str(&config.url)?
                    .statement_cache_capacity(config.statement_cache_capacity)
                    .ssl_mode(mysql_ssl_mode(config.ssl_mode));
                if config.application_name.is_some() {
                    tracing::warn!(
                        "application_name is not supported by the sqlx MySQL backend; ignoring"
                    );
                }

                let pool = sqlx::mysql::MySqlPoolOptions::new()
                    .max_connections(config.max_connections)
                    .min_connections(config.min_connections)
                    .acquire_timeout(config.connect_timeout)
                    .idle_timeout(config.idle_timeout)
                    .max_lifetime(config.max_lifetime)
                    .connect_with(options)
                    .await?;
                Ok(Self::MySql(pool))
            }
            #[cfg(feature = "sqlite")]
            DatabaseBackend::Sqlite => {
                use std::str::FromStr;

                use sqlx::sqlite::SqliteConnectOptions;

                let options = SqliteConnectOptions::from_str(&config.url)?
                    .statement_cache_capacity(config.statement_cache_capacity);
                if config.ssl_mode != SslMode::default() {
                    tracing::warn!("ssl_mode is not applicable to SQLite connections; ignoring");
                }
                if config.application_name.is_some() {
                    tracing::warn!(
                        "application_name is not supported by the sqlx SQLite backend; ignoring"
                    );
                }

                let pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(config.max_connections)
                    .min_connections(config.min_connections)
                    .acquire_timeout(config.connect_timeout)
                    .idle_timeout(config.idle_timeout)
                    .max_lifetime(config.max_lifetime)
                    .connect_with(options)
                    .await?;
                Ok(Self::Sqlite(pool))
            }
            #[allow(unreachable_patterns)]
            _ => Err(SqlxError::config(format!(
                "Database backend {:?} not enabled. Enable the corresponding feature.",
                config.backend
            ))),
        }
    }

    /// Get the database backend type.
    pub fn backend(&self) -> DatabaseBackend {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(_) => DatabaseBackend::Postgres,
            #[cfg(feature = "mysql")]
            Self::MySql(_) => DatabaseBackend::MySql,
            #[cfg(feature = "sqlite")]
            Self::Sqlite(_) => DatabaseBackend::Sqlite,
        }
    }

    /// Close the pool.
    pub async fn close(&self) {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(pool) => pool.close().await,
            #[cfg(feature = "mysql")]
            Self::MySql(pool) => pool.close().await,
            #[cfg(feature = "sqlite")]
            Self::Sqlite(pool) => pool.close().await,
        }
    }

    /// Check if the pool is closed.
    pub fn is_closed(&self) -> bool {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(pool) => pool.is_closed(),
            #[cfg(feature = "mysql")]
            Self::MySql(pool) => pool.is_closed(),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(pool) => pool.is_closed(),
        }
    }

    /// Get pool statistics.
    pub fn size(&self) -> u32 {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(pool) => pool.size(),
            #[cfg(feature = "mysql")]
            Self::MySql(pool) => pool.size(),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(pool) => pool.size(),
        }
    }

    /// Get number of idle connections.
    pub fn num_idle(&self) -> usize {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(pool) => pool.num_idle(),
            #[cfg(feature = "mysql")]
            Self::MySql(pool) => pool.num_idle(),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(pool) => pool.num_idle(),
        }
    }

    /// Get the underlying PostgreSQL pool.
    #[cfg(feature = "postgres")]
    pub fn as_postgres(&self) -> Option<&sqlx::PgPool> {
        match self {
            Self::Postgres(pool) => Some(pool),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    /// Get the underlying MySQL pool.
    #[cfg(feature = "mysql")]
    pub fn as_mysql(&self) -> Option<&sqlx::MySqlPool> {
        match self {
            Self::MySql(pool) => Some(pool),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    /// Get the underlying SQLite pool.
    #[cfg(feature = "sqlite")]
    pub fn as_sqlite(&self) -> Option<&sqlx::SqlitePool> {
        match self {
            Self::Sqlite(pool) => Some(pool),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}

/// Pool builder for SQLx.
pub struct SqlxPoolBuilder {
    config: SqlxConfig,
}

impl SqlxPoolBuilder {
    /// Create a new pool builder from a configuration.
    pub fn new(config: SqlxConfig) -> Self {
        Self { config }
    }

    /// Create a new pool builder from a URL.
    pub fn from_url(url: impl Into<String>) -> SqlxResult<Self> {
        let config = SqlxConfig::from_url(url)?;
        Ok(Self { config })
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

    /// Build and connect the pool.
    pub async fn build(self) -> SqlxResult<SqlxPool> {
        SqlxPool::connect(&self.config).await
    }
}

/// Pool status information.
#[derive(Debug, Clone)]
pub struct PoolStatus {
    /// Total pool size
    pub size: u32,
    /// Number of idle connections
    pub idle: usize,
    /// Whether the pool is closed
    pub is_closed: bool,
    /// Database backend type
    pub backend: DatabaseBackend,
}

impl SqlxPool {
    /// Get the pool status.
    pub fn status(&self) -> PoolStatus {
        PoolStatus {
            size: self.size(),
            idle: self.num_idle(),
            is_closed: self.is_closed(),
            backend: self.backend(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_builder() {
        let builder = SqlxPoolBuilder::from_url("postgres://localhost/test").unwrap();
        let builder = builder.max_connections(20).min_connections(5);
        assert_eq!(builder.config.max_connections, 20);
        assert_eq!(builder.config.min_connections, 5);
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_pg_ssl_mode_mapping() {
        use sqlx::postgres::PgSslMode;

        assert!(matches!(pg_ssl_mode(SslMode::Disable), PgSslMode::Disable));
        assert!(matches!(pg_ssl_mode(SslMode::Prefer), PgSslMode::Prefer));
        assert!(matches!(pg_ssl_mode(SslMode::Require), PgSslMode::Require));
        assert!(matches!(
            pg_ssl_mode(SslMode::VerifyCa),
            PgSslMode::VerifyCa
        ));
        assert!(matches!(
            pg_ssl_mode(SslMode::VerifyFull),
            PgSslMode::VerifyFull
        ));
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_mysql_ssl_mode_mapping() {
        use sqlx::mysql::MySqlSslMode;

        assert!(matches!(
            mysql_ssl_mode(SslMode::Disable),
            MySqlSslMode::Disabled
        ));
        assert!(matches!(
            mysql_ssl_mode(SslMode::Prefer),
            MySqlSslMode::Preferred
        ));
        assert!(matches!(
            mysql_ssl_mode(SslMode::Require),
            MySqlSslMode::Required
        ));
        assert!(matches!(
            mysql_ssl_mode(SslMode::VerifyCa),
            MySqlSslMode::VerifyCa
        ));
        assert!(matches!(
            mysql_ssl_mode(SslMode::VerifyFull),
            MySqlSslMode::VerifyIdentity
        ));
    }
}
