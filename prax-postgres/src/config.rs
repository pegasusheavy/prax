//! PostgreSQL connection configuration.

use std::time::Duration;

use crate::error::{PgError, PgResult};

/// PostgreSQL connection configuration.
#[derive(Debug, Clone)]
pub struct PgConfig {
    /// Database URL.
    pub url: String,
    /// Host (extracted from URL or explicit).
    pub host: String,
    /// Port (default: 5432).
    pub port: u16,
    /// Database name.
    pub database: String,
    /// Username.
    pub user: String,
    /// Password.
    pub password: Option<String>,
    /// SSL mode.
    pub ssl_mode: SslMode,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Statement timeout.
    pub statement_timeout: Option<Duration>,
    /// Application name (shown in pg_stat_activity).
    pub application_name: Option<String>,
    /// Additional options.
    pub options: Vec<(String, String)>,
}

/// SSL mode for connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SslMode {
    /// Disable SSL.
    Disable,
    /// Prefer SSL but allow non-SSL.
    #[default]
    Prefer,
    /// Require SSL.
    Require,
}

impl PgConfig {
    /// Create a new configuration from a database URL.
    pub fn from_url(url: impl Into<String>) -> PgResult<Self> {
        let url = url.into();
        let parsed = url::Url::parse(&url)
            .map_err(|e| PgError::config(format!("invalid database URL: {}", e)))?;

        if parsed.scheme() != "postgresql" && parsed.scheme() != "postgres" {
            return Err(PgError::config(format!(
                "invalid scheme: expected 'postgresql' or 'postgres', got '{}'",
                parsed.scheme()
            )));
        }

        let host = parsed
            .host_str()
            .ok_or_else(|| PgError::config("missing host in URL"))?
            .to_string();

        let port = parsed.port().unwrap_or(5432);

        let database = parsed.path().trim_start_matches('/').to_string();

        if database.is_empty() {
            return Err(PgError::config("missing database name in URL"));
        }

        let user = if parsed.username().is_empty() {
            "postgres".to_string()
        } else {
            parsed.username().to_string()
        };

        let password = parsed.password().map(String::from);

        // Parse query parameters
        let mut ssl_mode = SslMode::Prefer;
        let mut connect_timeout = Duration::from_secs(30);
        let mut statement_timeout = None;
        let mut application_name = None;
        let mut options = Vec::new();

        for (key, value) in parsed.query_pairs() {
            let key_str: &str = &key;
            let value_str: &str = &value;
            match key_str {
                "sslmode" => {
                    ssl_mode = match value_str {
                        "disable" => SslMode::Disable,
                        "prefer" => SslMode::Prefer,
                        "require" => SslMode::Require,
                        other => {
                            return Err(PgError::config(format!("invalid sslmode: {}", other)));
                        }
                    };
                }
                "connect_timeout" => {
                    let secs: u64 = value_str
                        .parse()
                        .map_err(|_| PgError::config("invalid connect_timeout"))?;
                    connect_timeout = Duration::from_secs(secs);
                }
                "statement_timeout" => {
                    let ms: u64 = value_str
                        .parse()
                        .map_err(|_| PgError::config("invalid statement_timeout"))?;
                    statement_timeout = Some(Duration::from_millis(ms));
                }
                "application_name" => {
                    application_name = Some(value_str.to_string());
                }
                _ => {
                    options.push((key_str.to_string(), value_str.to_string()));
                }
            }
        }

        Ok(Self {
            url,
            host,
            port,
            database,
            user,
            password,
            ssl_mode,
            connect_timeout,
            statement_timeout,
            application_name,
            options,
        })
    }

    /// Create a builder for configuration.
    pub fn builder() -> PgConfigBuilder {
        PgConfigBuilder::new()
    }

    /// Convert to tokio-postgres config.
    pub fn to_pg_config(&self) -> tokio_postgres::Config {
        let mut config = tokio_postgres::Config::new();
        config.host(&self.host);
        config.port(self.port);
        config.dbname(&self.database);
        config.user(&self.user);

        if let Some(ref password) = self.password {
            config.password(password);
        }

        if let Some(ref app_name) = self.application_name {
            config.application_name(app_name);
        }

        config.connect_timeout(self.connect_timeout);

        config
    }
}

/// Builder for PostgreSQL configuration.
#[derive(Debug, Default)]
pub struct PgConfigBuilder {
    url: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    user: Option<String>,
    password: Option<String>,
    ssl_mode: Option<SslMode>,
    connect_timeout: Option<Duration>,
    statement_timeout: Option<Duration>,
    application_name: Option<String>,
}

impl PgConfigBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the database URL (parses all connection parameters).
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
    pub fn database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
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

    /// Set the SSL mode.
    pub fn ssl_mode(mut self, mode: SslMode) -> Self {
        self.ssl_mode = Some(mode);
        self
    }

    /// Set the connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Set the statement timeout.
    pub fn statement_timeout(mut self, timeout: Duration) -> Self {
        self.statement_timeout = Some(timeout);
        self
    }

    /// Set the application name.
    pub fn application_name(mut self, name: impl Into<String>) -> Self {
        self.application_name = Some(name.into());
        self
    }

    /// Build the configuration.
    pub fn build(self) -> PgResult<PgConfig> {
        if let Some(url) = self.url {
            let mut config = PgConfig::from_url(url)?;

            // Override with explicit values
            if let Some(host) = self.host {
                config.host = host;
            }
            if let Some(port) = self.port {
                config.port = port;
            }
            if let Some(database) = self.database {
                config.database = database;
            }
            if let Some(user) = self.user {
                config.user = user;
            }
            if let Some(password) = self.password {
                config.password = Some(password);
            }
            if let Some(ssl_mode) = self.ssl_mode {
                config.ssl_mode = ssl_mode;
            }
            if let Some(timeout) = self.connect_timeout {
                config.connect_timeout = timeout;
            }
            if let Some(timeout) = self.statement_timeout {
                config.statement_timeout = Some(timeout);
            }
            if let Some(name) = self.application_name {
                config.application_name = Some(name);
            }

            Ok(config)
        } else {
            // Build from individual components
            let host = self.host.unwrap_or_else(|| "localhost".to_string());
            let port = self.port.unwrap_or(5432);
            let database = self
                .database
                .ok_or_else(|| PgError::config("database name is required"))?;
            let user = self.user.unwrap_or_else(|| "postgres".to_string());

            let url = format!(
                "postgresql://{}{}@{}:{}/{}",
                user,
                self.password
                    .as_ref()
                    .map(|p| format!(":{}", p))
                    .unwrap_or_default(),
                host,
                port,
                database
            );

            Ok(PgConfig {
                url,
                host,
                port,
                database,
                user,
                password: self.password,
                ssl_mode: self.ssl_mode.unwrap_or_default(),
                connect_timeout: self.connect_timeout.unwrap_or(Duration::from_secs(30)),
                statement_timeout: self.statement_timeout,
                application_name: self.application_name,
                options: Vec::new(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_url() {
        let config = PgConfig::from_url("postgresql://user:pass@localhost:5432/mydb").unwrap();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.database, "mydb");
        assert_eq!(config.user, "user");
        assert_eq!(config.password, Some("pass".to_string()));
    }

    #[test]
    fn test_config_from_url_with_params() {
        let config =
            PgConfig::from_url("postgresql://localhost/mydb?sslmode=require&application_name=prax")
                .unwrap();
        assert_eq!(config.ssl_mode, SslMode::Require);
        assert_eq!(config.application_name, Some("prax".to_string()));
    }

    #[test]
    fn test_config_builder() {
        let config = PgConfig::builder()
            .host("localhost")
            .port(5432)
            .database("mydb")
            .user("postgres")
            .build()
            .unwrap();

        assert_eq!(config.host, "localhost");
        assert_eq!(config.database, "mydb");
    }

    #[test]
    fn test_config_invalid_scheme() {
        let result = PgConfig::from_url("mysql://localhost/db");
        assert!(result.is_err());
    }
}
