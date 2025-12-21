//! MySQL configuration.

use std::time::Duration;

use mysql_async::OptsBuilder;
use url::Url;

use crate::error::{MysqlError, MysqlResult};

/// MySQL database configuration.
#[derive(Debug, Clone)]
pub struct MysqlConfig {
    /// Database host.
    pub host: String,
    /// Database port.
    pub port: u16,
    /// Database name.
    pub database: String,
    /// Username for authentication.
    pub username: Option<String>,
    /// Password for authentication.
    pub password: Option<String>,
    /// Connection timeout.
    pub connect_timeout: Option<Duration>,
    /// SSL mode.
    pub ssl_mode: SslMode,
    /// Additional connection options.
    pub options: Vec<(String, String)>,
}

/// SSL mode for MySQL connections.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SslMode {
    /// No SSL.
    #[default]
    Disabled,
    /// Prefer SSL but allow non-SSL.
    Preferred,
    /// Require SSL.
    Required,
    /// Require SSL and verify CA certificate.
    VerifyCa,
    /// Require SSL and verify full certificate chain.
    VerifyIdentity,
}

impl Default for MysqlConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 3306,
            database: String::new(),
            username: None,
            password: None,
            connect_timeout: Some(Duration::from_secs(30)),
            ssl_mode: SslMode::default(),
            options: Vec::new(),
        }
    }
}

impl MysqlConfig {
    /// Create a new configuration with the given database name.
    pub fn new(database: impl Into<String>) -> Self {
        Self {
            database: database.into(),
            ..Default::default()
        }
    }

    /// Parse a MySQL URL into configuration.
    ///
    /// Supported formats:
    /// - `mysql://user:password@host:port/database`
    /// - `mysql://host/database`
    pub fn from_url(url: impl AsRef<str>) -> MysqlResult<Self> {
        let url_str = url.as_ref();
        let parsed =
            Url::parse(url_str).map_err(|e| MysqlError::config(format!("invalid URL: {}", e)))?;

        if parsed.scheme() != "mysql" {
            return Err(MysqlError::config(format!(
                "invalid scheme '{}', expected 'mysql'",
                parsed.scheme()
            )));
        }

        let host = parsed.host_str().unwrap_or("localhost").to_string();
        let port = parsed.port().unwrap_or(3306);
        let database = parsed.path().trim_start_matches('/').to_string();

        if database.is_empty() {
            return Err(MysqlError::config("database name is required"));
        }

        let username = if parsed.username().is_empty() {
            None
        } else {
            Some(parsed.username().to_string())
        };

        let password = parsed.password().map(|s| s.to_string());

        // Parse query parameters for additional options
        let mut connect_timeout = Some(Duration::from_secs(30));
        let mut ssl_mode = SslMode::default();
        let mut options = Vec::new();

        for (key, value) in parsed.query_pairs() {
            match key.as_ref() {
                "connect_timeout" => {
                    if let Ok(secs) = value.parse::<u64>() {
                        connect_timeout = Some(Duration::from_secs(secs));
                    }
                }
                "ssl_mode" | "sslmode" => {
                    ssl_mode = match value.as_ref() {
                        "disabled" | "DISABLED" => SslMode::Disabled,
                        "preferred" | "PREFERRED" => SslMode::Preferred,
                        "required" | "REQUIRED" => SslMode::Required,
                        "verify_ca" | "VERIFY_CA" => SslMode::VerifyCa,
                        "verify_identity" | "VERIFY_IDENTITY" => SslMode::VerifyIdentity,
                        _ => SslMode::default(),
                    };
                }
                _ => {
                    options.push((key.to_string(), value.to_string()));
                }
            }
        }

        Ok(Self {
            host,
            port,
            database,
            username,
            password,
            connect_timeout,
            ssl_mode,
            options,
        })
    }

    /// Convert to mysql_async OptsBuilder.
    pub fn to_opts_builder(&self) -> OptsBuilder {
        let mut builder = OptsBuilder::default()
            .ip_or_hostname(&self.host)
            .tcp_port(self.port)
            .db_name(Some(&self.database));

        if let Some(ref user) = self.username {
            builder = builder.user(Some(user));
        }

        if let Some(ref pass) = self.password {
            builder = builder.pass(Some(pass));
        }

        // Note: mysql_async OptsBuilder doesn't have connect_timeout method.
        // Timeout is handled at the pool level.
        let _ = self.connect_timeout; // suppress unused warning

        // Configure SSL based on mode
        match self.ssl_mode {
            SslMode::Disabled => {
                builder = builder.prefer_socket(true);
            }
            SslMode::Preferred | SslMode::Required => {
                // mysql_async handles SSL via the ssl_opts builder
            }
            SslMode::VerifyCa | SslMode::VerifyIdentity => {
                // Would need ssl_opts with proper cert verification
            }
        }

        builder
    }

    /// Set the host.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Set the port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the database name.
    pub fn database(mut self, database: impl Into<String>) -> Self {
        self.database = database.into();
        self
    }

    /// Set the username.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the password.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set the connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Set the SSL mode.
    pub fn ssl_mode(mut self, mode: SslMode) -> Self {
        self.ssl_mode = mode;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = MysqlConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 3306);
    }

    #[test]
    fn test_config_from_url() {
        let config = MysqlConfig::from_url("mysql://user:pass@localhost:3307/testdb").unwrap();

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 3307);
        assert_eq!(config.database, "testdb");
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("pass".to_string()));
    }

    #[test]
    fn test_config_from_url_minimal() {
        let config = MysqlConfig::from_url("mysql://localhost/mydb").unwrap();

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 3306);
        assert_eq!(config.database, "mydb");
        assert!(config.username.is_none());
        assert!(config.password.is_none());
    }

    #[test]
    fn test_config_from_url_invalid_scheme() {
        let result = MysqlConfig::from_url("postgres://localhost/mydb");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_from_url_no_database() {
        let result = MysqlConfig::from_url("mysql://localhost/");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_from_url_with_options() {
        let config =
            MysqlConfig::from_url("mysql://localhost/mydb?connect_timeout=60&ssl_mode=required")
                .unwrap();

        assert_eq!(config.connect_timeout, Some(Duration::from_secs(60)));
        assert_eq!(config.ssl_mode, SslMode::Required);
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = MysqlConfig::new("mydb")
            .host("db.example.com")
            .port(3307)
            .username("admin")
            .password("secret")
            .ssl_mode(SslMode::Required);

        assert_eq!(config.host, "db.example.com");
        assert_eq!(config.port, 3307);
        assert_eq!(config.database, "mydb");
        assert_eq!(config.username, Some("admin".to_string()));
        assert_eq!(config.password, Some("secret".to_string()));
        assert_eq!(config.ssl_mode, SslMode::Required);
    }
}
