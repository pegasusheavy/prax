//! Database configuration.

use super::{
    ConnectionError, ConnectionOptions, ConnectionResult, ConnectionString, Driver, MySqlOptions,
    PoolOptions, PostgresOptions, SqliteOptions, SslMode,
};
use std::collections::HashMap;
use std::time::Duration;
use tracing::info;

/// Complete database configuration.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database driver.
    pub driver: Driver,
    /// Connection URL.
    pub url: String,
    /// Host (if not in URL).
    pub host: Option<String>,
    /// Port (if not in URL).
    pub port: Option<u16>,
    /// Database name (if not in URL).
    pub database: Option<String>,
    /// Username (if not in URL).
    pub user: Option<String>,
    /// Password (if not in URL).
    pub password: Option<String>,
    /// Connection options.
    pub connection: ConnectionOptions,
    /// Pool options.
    pub pool: PoolOptions,
    /// PostgreSQL-specific options.
    pub postgres: Option<PostgresOptions>,
    /// MySQL-specific options.
    pub mysql: Option<MySqlOptions>,
    /// SQLite-specific options.
    pub sqlite: Option<SqliteOptions>,
}

impl DatabaseConfig {
    /// Create a new PostgreSQL configuration builder.
    pub fn postgres() -> DatabaseConfigBuilder {
        DatabaseConfigBuilder::new(Driver::Postgres)
    }

    /// Create a new MySQL configuration builder.
    pub fn mysql() -> DatabaseConfigBuilder {
        DatabaseConfigBuilder::new(Driver::MySql)
    }

    /// Create a new SQLite configuration builder.
    pub fn sqlite() -> DatabaseConfigBuilder {
        DatabaseConfigBuilder::new(Driver::Sqlite)
    }

    /// Create configuration from a connection string.
    pub fn from_url(url: &str) -> ConnectionResult<Self> {
        let conn = ConnectionString::parse(url)?;
        let opts = ConnectionOptions::from_params(conn.params());

        let config = Self {
            driver: conn.driver(),
            url: url.to_string(),
            host: conn.host().map(String::from),
            port: conn.port(),
            database: conn.database().map(String::from),
            user: conn.user().map(String::from),
            password: conn.password().map(String::from),
            connection: opts,
            pool: PoolOptions::default(),
            postgres: if conn.driver() == Driver::Postgres {
                Some(PostgresOptions::new())
            } else {
                None
            },
            mysql: if conn.driver() == Driver::MySql {
                Some(MySqlOptions::new())
            } else {
                None
            },
            sqlite: if conn.driver() == Driver::Sqlite {
                Some(SqliteOptions::new())
            } else {
                None
            },
        };

        info!(
            driver = %config.driver.name(),
            host = ?config.host,
            database = ?config.database,
            "DatabaseConfig loaded from URL"
        );

        Ok(config)
    }

    /// Create configuration from DATABASE_URL environment variable.
    pub fn from_env() -> ConnectionResult<Self> {
        info!("Loading database configuration from DATABASE_URL");
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| ConnectionError::EnvNotFound("DATABASE_URL".to_string()))?;
        Self::from_url(&url)
    }

    /// Build a connection URL from the configuration.
    pub fn to_url(&self) -> String {
        if !self.url.is_empty() {
            return self.url.clone();
        }

        let mut url = format!("{}://", self.driver.name());

        if let Some(ref user) = self.user {
            url.push_str(user);
            if let Some(ref pass) = self.password {
                url.push(':');
                url.push_str(pass);
            }
            url.push('@');
        }

        if let Some(ref host) = self.host {
            url.push_str(host);
            if let Some(port) = self.port {
                url.push(':');
                url.push_str(&port.to_string());
            }
        }

        if let Some(ref db) = self.database {
            url.push('/');
            url.push_str(db);
        }

        url
    }
}

/// Builder for database configuration.
pub struct DatabaseConfigBuilder {
    driver: Driver,
    url: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    user: Option<String>,
    password: Option<String>,
    connection: ConnectionOptions,
    pool: PoolOptions,
    postgres: Option<PostgresOptions>,
    mysql: Option<MySqlOptions>,
    sqlite: Option<SqliteOptions>,
}

impl DatabaseConfigBuilder {
    /// Create a new builder for the given driver.
    pub fn new(driver: Driver) -> Self {
        Self {
            driver,
            url: None,
            host: None,
            port: None,
            database: None,
            user: None,
            password: None,
            connection: ConnectionOptions::default(),
            pool: PoolOptions::default(),
            postgres: if driver == Driver::Postgres {
                Some(PostgresOptions::new())
            } else {
                None
            },
            mysql: if driver == Driver::MySql {
                Some(MySqlOptions::new())
            } else {
                None
            },
            sqlite: if driver == Driver::Sqlite {
                Some(SqliteOptions::new())
            } else {
                None
            },
        }
    }

    /// Set the connection URL (overrides other connection settings).
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the host.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Set the port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set the database name.
    pub fn database(mut self, db: impl Into<String>) -> Self {
        self.database = Some(db.into());
        self
    }

    /// Set the username.
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Set the password.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connection.connect_timeout = timeout;
        self
    }

    /// Set SSL mode.
    pub fn ssl_mode(mut self, mode: SslMode) -> Self {
        self.connection.ssl.mode = mode;
        self
    }

    /// Set application name.
    pub fn application_name(mut self, name: impl Into<String>) -> Self {
        self.connection.application_name = Some(name.into());
        self
    }

    /// Set max connections.
    pub fn max_connections(mut self, n: u32) -> Self {
        self.pool.max_connections = n;
        self
    }

    /// Set min connections.
    pub fn min_connections(mut self, n: u32) -> Self {
        self.pool.min_connections = n;
        self
    }

    /// Set idle timeout.
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.pool.idle_timeout = Some(timeout);
        self
    }

    /// Set max lifetime.
    pub fn max_lifetime(mut self, lifetime: Duration) -> Self {
        self.pool.max_lifetime = Some(lifetime);
        self
    }

    /// Configure PostgreSQL options.
    pub fn postgres_options<F>(mut self, f: F) -> Self
    where
        F: FnOnce(PostgresOptions) -> PostgresOptions,
    {
        if let Some(opts) = self.postgres.take() {
            self.postgres = Some(f(opts));
        }
        self
    }

    /// Configure MySQL options.
    pub fn mysql_options<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MySqlOptions) -> MySqlOptions,
    {
        if let Some(opts) = self.mysql.take() {
            self.mysql = Some(f(opts));
        }
        self
    }

    /// Configure SQLite options.
    pub fn sqlite_options<F>(mut self, f: F) -> Self
    where
        F: FnOnce(SqliteOptions) -> SqliteOptions,
    {
        if let Some(opts) = self.sqlite.take() {
            self.sqlite = Some(f(opts));
        }
        self
    }

    /// Build the configuration.
    pub fn build(self) -> ConnectionResult<DatabaseConfig> {
        // Validate required fields based on driver
        if self.url.is_none() {
            match self.driver {
                Driver::Postgres | Driver::MySql => {
                    if self.host.is_none() {
                        return Err(ConnectionError::MissingField("host".to_string()));
                    }
                }
                Driver::Sqlite => {
                    if self.database.is_none() {
                        return Err(ConnectionError::MissingField(
                            "database (file path)".to_string(),
                        ));
                    }
                }
            }
        }

        Ok(DatabaseConfig {
            driver: self.driver,
            url: self.url.unwrap_or_default(),
            host: self.host,
            port: self.port,
            database: self.database,
            user: self.user,
            password: self.password,
            connection: self.connection,
            pool: self.pool,
            postgres: self.postgres,
            mysql: self.mysql,
            sqlite: self.sqlite,
        })
    }
}

/// Configuration for multiple databases.
#[derive(Debug, Clone, Default)]
pub struct MultiDatabaseConfig {
    /// Primary database configuration.
    pub primary: Option<DatabaseConfig>,
    /// Read replica configurations.
    pub replicas: Vec<DatabaseConfig>,
    /// Named database configurations.
    pub databases: HashMap<String, DatabaseConfig>,
    /// Load balancing strategy for replicas.
    pub load_balance: LoadBalanceStrategy,
}

/// Load balancing strategy for read replicas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadBalanceStrategy {
    /// Round-robin between replicas.
    #[default]
    RoundRobin,
    /// Random selection.
    Random,
    /// Use the first available replica.
    First,
    /// Use the replica with lowest latency.
    LeastLatency,
}

impl MultiDatabaseConfig {
    /// Create a new multi-database configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the primary database.
    pub fn primary(mut self, config: DatabaseConfig) -> Self {
        self.primary = Some(config);
        self
    }

    /// Add a read replica.
    pub fn replica(mut self, config: DatabaseConfig) -> Self {
        self.replicas.push(config);
        self
    }

    /// Add a named database.
    pub fn database(mut self, name: impl Into<String>, config: DatabaseConfig) -> Self {
        self.databases.insert(name.into(), config);
        self
    }

    /// Set load balancing strategy.
    pub fn load_balance(mut self, strategy: LoadBalanceStrategy) -> Self {
        self.load_balance = strategy;
        self
    }

    /// Get the primary database configuration.
    pub fn get_primary(&self) -> Option<&DatabaseConfig> {
        self.primary.as_ref()
    }

    /// Get a named database configuration.
    pub fn get(&self, name: &str) -> Option<&DatabaseConfig> {
        self.databases.get(name)
    }

    /// Check if replicas are configured.
    pub fn has_replicas(&self) -> bool {
        !self.replicas.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_url() {
        let config =
            DatabaseConfig::from_url("postgres://user:pass@localhost:5432/mydb?sslmode=require")
                .unwrap();

        assert_eq!(config.driver, Driver::Postgres);
        assert_eq!(config.host, Some("localhost".to_string()));
        assert_eq!(config.port, Some(5432));
        assert_eq!(config.database, Some("mydb".to_string()));
        assert_eq!(config.user, Some("user".to_string()));
        assert!(config.postgres.is_some());
    }

    #[test]
    fn test_postgres_builder() {
        let config = DatabaseConfig::postgres()
            .host("localhost")
            .port(5432)
            .database("mydb")
            .user("user")
            .password("pass")
            .max_connections(20)
            .ssl_mode(SslMode::Require)
            .build()
            .unwrap();

        assert_eq!(config.driver, Driver::Postgres);
        assert_eq!(config.host, Some("localhost".to_string()));
        assert_eq!(config.pool.max_connections, 20);
        assert_eq!(config.connection.ssl.mode, SslMode::Require);
    }

    #[test]
    fn test_mysql_builder() {
        let config = DatabaseConfig::mysql()
            .host("127.0.0.1")
            .database("testdb")
            .user("root")
            .mysql_options(|opts| opts.charset("utf8mb4"))
            .build()
            .unwrap();

        assert_eq!(config.driver, Driver::MySql);
        assert!(config.mysql.is_some());
        assert_eq!(config.mysql.unwrap().charset, Some("utf8mb4".to_string()));
    }

    #[test]
    fn test_sqlite_builder() {
        let config = DatabaseConfig::sqlite()
            .database("./data/app.db")
            .sqlite_options(|opts| opts.foreign_keys(true))
            .build()
            .unwrap();

        assert_eq!(config.driver, Driver::Sqlite);
        assert!(config.sqlite.is_some());
        assert!(config.sqlite.unwrap().foreign_keys);
    }

    #[test]
    fn test_builder_validation() {
        // Missing host for PostgreSQL
        let result = DatabaseConfig::postgres().database("mydb").build();
        assert!(result.is_err());

        // Missing database for SQLite
        let result = DatabaseConfig::sqlite().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_multi_database_config() {
        let config = MultiDatabaseConfig::new()
            .primary(DatabaseConfig::from_url("postgres://localhost/primary").unwrap())
            .replica(DatabaseConfig::from_url("postgres://localhost/replica1").unwrap())
            .replica(DatabaseConfig::from_url("postgres://localhost/replica2").unwrap())
            .database(
                "analytics",
                DatabaseConfig::from_url("postgres://localhost/analytics").unwrap(),
            )
            .load_balance(LoadBalanceStrategy::RoundRobin);

        assert!(config.get_primary().is_some());
        assert_eq!(config.replicas.len(), 2);
        assert!(config.get("analytics").is_some());
        assert!(config.has_replicas());
    }

    #[test]
    fn test_to_url() {
        let config = DatabaseConfig::postgres()
            .host("localhost")
            .port(5432)
            .database("mydb")
            .user("user")
            .password("pass")
            .build()
            .unwrap();

        let url = config.to_url();
        assert!(url.contains("postgres://"));
        assert!(url.contains("user:pass@"));
        assert!(url.contains("localhost:5432"));
        assert!(url.contains("/mydb"));
    }
}
