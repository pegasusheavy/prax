//! Microsoft SQL Server connection configuration.

use std::time::Duration;

use tiberius::{AuthMethod, Config, EncryptionLevel};

use crate::error::{MssqlError, MssqlResult};

/// Microsoft SQL Server connection configuration.
#[derive(Debug, Clone)]
pub struct MssqlConfig {
    /// Server host.
    pub host: String,
    /// Server port (default: 1433).
    pub port: u16,
    /// Database name.
    pub database: String,
    /// Username for SQL Server authentication.
    pub username: Option<String>,
    /// Password for SQL Server authentication.
    pub password: Option<String>,
    /// Use Windows Authentication (Integrated Security).
    pub windows_auth: bool,
    /// Encryption level.
    pub encryption: EncryptionMode,
    /// Trust server certificate.
    pub trust_cert: bool,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Application name (shown in sys.dm_exec_sessions).
    pub application_name: Option<String>,
    /// Instance name (for named instances).
    pub instance_name: Option<String>,
}

/// Encryption mode for connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EncryptionMode {
    /// Encryption is off.
    Off,
    /// Encryption is on.
    #[default]
    On,
    /// Encryption is required.
    Required,
    /// Don't use encryption.
    NotSupported,
}

impl From<EncryptionMode> for EncryptionLevel {
    fn from(mode: EncryptionMode) -> Self {
        match mode {
            EncryptionMode::Off => EncryptionLevel::Off,
            EncryptionMode::On => EncryptionLevel::On,
            EncryptionMode::Required => EncryptionLevel::Required,
            EncryptionMode::NotSupported => EncryptionLevel::NotSupported,
        }
    }
}

impl Default for MssqlConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 1433,
            database: String::new(),
            username: None,
            password: None,
            windows_auth: false,
            encryption: EncryptionMode::On,
            trust_cert: false,
            connect_timeout: Duration::from_secs(30),
            application_name: Some("prax".to_string()),
            instance_name: None,
        }
    }
}

impl MssqlConfig {
    /// Create a new configuration from a connection string.
    ///
    /// Supported connection string formats:
    /// - `mssql://user:pass@host:port/database`
    /// - `Server=host;Database=db;User Id=user;Password=pass;`
    pub fn from_connection_string(conn_str: impl Into<String>) -> MssqlResult<Self> {
        let conn_str = conn_str.into();

        // Try URL-style first
        if conn_str.starts_with("mssql://") || conn_str.starts_with("sqlserver://") {
            return Self::from_url(&conn_str);
        }

        // Parse ADO.NET style connection string
        Self::from_ado_string(&conn_str)
    }

    /// Parse URL-style connection string.
    fn from_url(url: &str) -> MssqlResult<Self> {
        let parsed = url::Url::parse(url)
            .map_err(|e| MssqlError::config(format!("invalid connection URL: {}", e)))?;

        if parsed.scheme() != "mssql" && parsed.scheme() != "sqlserver" {
            return Err(MssqlError::config(format!(
                "invalid scheme: expected 'mssql' or 'sqlserver', got '{}'",
                parsed.scheme()
            )));
        }

        let host = parsed
            .host_str()
            .ok_or_else(|| MssqlError::config("missing host in URL"))?
            .to_string();

        let port = parsed.port().unwrap_or(1433);

        let database = parsed.path().trim_start_matches('/').to_string();
        if database.is_empty() {
            return Err(MssqlError::config("missing database name in URL"));
        }

        let username = if parsed.username().is_empty() {
            None
        } else {
            Some(parsed.username().to_string())
        };

        let password = parsed.password().map(String::from);

        // Parse query parameters
        let mut encryption = EncryptionMode::On;
        let mut trust_cert = false;
        let mut connect_timeout = Duration::from_secs(30);
        let mut application_name = Some("prax".to_string());
        let mut instance_name = None;
        let mut windows_auth = false;

        for (key, value) in parsed.query_pairs() {
            match key.to_lowercase().as_str() {
                "encrypt" => {
                    encryption = match value.to_lowercase().as_str() {
                        "true" | "yes" | "on" => EncryptionMode::On,
                        "false" | "no" | "off" => EncryptionMode::Off,
                        "required" | "strict" => EncryptionMode::Required,
                        _ => EncryptionMode::On,
                    };
                }
                "trustservercertificate" | "trust_cert" => {
                    trust_cert = value.to_lowercase() == "true" || value.to_lowercase() == "yes";
                }
                "connecttimeout" | "connect_timeout" | "timeout" => {
                    if let Ok(secs) = value.parse::<u64>() {
                        connect_timeout = Duration::from_secs(secs);
                    }
                }
                "applicationname" | "application_name" | "app" => {
                    application_name = Some(value.to_string());
                }
                "instancename" | "instance" => {
                    instance_name = Some(value.to_string());
                }
                "integratedsecurity" | "trusted_connection" => {
                    windows_auth = value.to_lowercase() == "true" || value.to_lowercase() == "sspi";
                }
                _ => {}
            }
        }

        Ok(Self {
            host,
            port,
            database,
            username,
            password,
            windows_auth,
            encryption,
            trust_cert,
            connect_timeout,
            application_name,
            instance_name,
        })
    }

    /// Parse ADO.NET style connection string.
    fn from_ado_string(conn_str: &str) -> MssqlResult<Self> {
        let mut config = Self::default();

        for part in conn_str.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            let (key, value) = part.split_once('=').ok_or_else(|| {
                MssqlError::config(format!("invalid connection string part: {}", part))
            })?;

            let key = key.trim().to_lowercase();
            let value = value.trim();

            match key.as_str() {
                "server" | "data source" | "host" => {
                    // Parse server\instance or server,port format
                    if let Some((server, instance)) = value.split_once('\\') {
                        config.host = server.to_string();
                        config.instance_name = Some(instance.to_string());
                    } else if let Some((server, port)) = value.split_once(',') {
                        config.host = server.to_string();
                        config.port = port.parse().unwrap_or(1433);
                    } else {
                        config.host = value.to_string();
                    }
                }
                "database" | "initial catalog" => {
                    config.database = value.to_string();
                }
                "user id" | "uid" | "user" | "username" => {
                    config.username = Some(value.to_string());
                }
                "password" | "pwd" => {
                    config.password = Some(value.to_string());
                }
                "integrated security" | "trusted_connection" => {
                    config.windows_auth = value.to_lowercase() == "true"
                        || value.to_lowercase() == "sspi"
                        || value.to_lowercase() == "yes";
                }
                "encrypt" => {
                    config.encryption = match value.to_lowercase().as_str() {
                        "true" | "yes" | "on" | "mandatory" => EncryptionMode::On,
                        "false" | "no" | "off" | "optional" => EncryptionMode::Off,
                        "strict" => EncryptionMode::Required,
                        _ => EncryptionMode::On,
                    };
                }
                "trustservercertificate" | "trust server certificate" => {
                    config.trust_cert =
                        value.to_lowercase() == "true" || value.to_lowercase() == "yes";
                }
                "connect timeout" | "connection timeout" | "timeout" => {
                    if let Ok(secs) = value.parse::<u64>() {
                        config.connect_timeout = Duration::from_secs(secs);
                    }
                }
                "application name" | "app" => {
                    config.application_name = Some(value.to_string());
                }
                _ => {}
            }
        }

        if config.database.is_empty() {
            return Err(MssqlError::config("database name is required"));
        }

        Ok(config)
    }

    /// Create a builder for configuration.
    pub fn builder() -> MssqlConfigBuilder {
        MssqlConfigBuilder::new()
    }

    /// Convert to a Tiberius Config.
    pub fn to_tiberius_config(&self) -> MssqlResult<Config> {
        let mut config = Config::new();

        config.host(&self.host);
        config.port(self.port);
        config.database(&self.database);

        if let Some(ref app_name) = self.application_name {
            config.application_name(app_name);
        }

        if let Some(ref instance) = self.instance_name {
            config.instance_name(instance);
        }

        // Set authentication method
        if self.windows_auth {
            #[cfg(windows)]
            {
                config.authentication(AuthMethod::Integrated);
            }
            #[cfg(not(windows))]
            {
                return Err(MssqlError::config(
                    "Windows Authentication is only supported on Windows",
                ));
            }
        } else if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            config.authentication(AuthMethod::sql_server(user, pass));
        } else {
            return Err(MssqlError::config(
                "either username/password or Windows Authentication is required",
            ));
        }

        // Set encryption
        config.encryption(self.encryption.into());

        if self.trust_cert {
            config.trust_cert();
        }

        Ok(config)
    }

    /// Generate an ADO.NET style connection string.
    pub fn to_connection_string(&self) -> String {
        let mut parts = vec![
            format!("Server={}", self.host),
            format!("Database={}", self.database),
        ];

        if self.port != 1433 {
            parts[0] = format!("Server={},{}", self.host, self.port);
        }

        if let Some(ref instance) = self.instance_name {
            parts[0] = format!("Server={}\\{}", self.host, instance);
        }

        if self.windows_auth {
            parts.push("Integrated Security=SSPI".to_string());
        } else {
            if let Some(ref user) = self.username {
                parts.push(format!("User Id={}", user));
            }
            if let Some(ref pass) = self.password {
                parts.push(format!("Password={}", pass));
            }
        }

        match self.encryption {
            EncryptionMode::On => parts.push("Encrypt=True".to_string()),
            EncryptionMode::Off => parts.push("Encrypt=False".to_string()),
            EncryptionMode::Required => parts.push("Encrypt=Strict".to_string()),
            EncryptionMode::NotSupported => parts.push("Encrypt=False".to_string()),
        }

        if self.trust_cert {
            parts.push("TrustServerCertificate=True".to_string());
        }

        if let Some(ref app_name) = self.application_name {
            parts.push(format!("Application Name={}", app_name));
        }

        parts.join(";")
    }
}

/// Builder for Microsoft SQL Server configuration.
#[derive(Debug, Default)]
pub struct MssqlConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    username: Option<String>,
    password: Option<String>,
    windows_auth: bool,
    encryption: Option<EncryptionMode>,
    trust_cert: bool,
    connect_timeout: Option<Duration>,
    application_name: Option<String>,
    instance_name: Option<String>,
}

impl MssqlConfigBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the server host.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Set the server port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set the database name.
    pub fn database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Set the username for SQL Server authentication.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the password for SQL Server authentication.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Use Windows Authentication (Integrated Security).
    pub fn windows_auth(mut self, enabled: bool) -> Self {
        self.windows_auth = enabled;
        self
    }

    /// Set the encryption mode.
    pub fn encryption(mut self, mode: EncryptionMode) -> Self {
        self.encryption = Some(mode);
        self
    }

    /// Trust the server certificate.
    pub fn trust_cert(mut self, trust: bool) -> Self {
        self.trust_cert = trust;
        self
    }

    /// Set the connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Set the application name.
    pub fn application_name(mut self, name: impl Into<String>) -> Self {
        self.application_name = Some(name.into());
        self
    }

    /// Set the instance name (for named instances).
    pub fn instance_name(mut self, name: impl Into<String>) -> Self {
        self.instance_name = Some(name.into());
        self
    }

    /// Build the configuration.
    pub fn build(self) -> MssqlResult<MssqlConfig> {
        let database = self
            .database
            .ok_or_else(|| MssqlError::config("database name is required"))?;

        if !self.windows_auth && (self.username.is_none() || self.password.is_none()) {
            return Err(MssqlError::config(
                "username and password are required for SQL Server authentication",
            ));
        }

        Ok(MssqlConfig {
            host: self.host.unwrap_or_else(|| "localhost".to_string()),
            port: self.port.unwrap_or(1433),
            database,
            username: self.username,
            password: self.password,
            windows_auth: self.windows_auth,
            encryption: self.encryption.unwrap_or_default(),
            trust_cert: self.trust_cert,
            connect_timeout: self.connect_timeout.unwrap_or(Duration::from_secs(30)),
            application_name: self.application_name.or(Some("prax".to_string())),
            instance_name: self.instance_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_url() {
        let config =
            MssqlConfig::from_connection_string("mssql://sa:Password123@localhost:1433/mydb")
                .unwrap();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 1433);
        assert_eq!(config.database, "mydb");
        assert_eq!(config.username, Some("sa".to_string()));
        assert_eq!(config.password, Some("Password123".to_string()));
    }

    #[test]
    fn test_config_from_ado_string() {
        let config = MssqlConfig::from_connection_string(
            "Server=localhost;Database=mydb;User Id=sa;Password=Password123;",
        )
        .unwrap();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.database, "mydb");
        assert_eq!(config.username, Some("sa".to_string()));
    }

    #[test]
    fn test_config_from_ado_string_with_instance() {
        let config = MssqlConfig::from_connection_string(
            "Server=localhost\\SQLEXPRESS;Database=mydb;User Id=sa;Password=pass;",
        )
        .unwrap();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.instance_name, Some("SQLEXPRESS".to_string()));
    }

    #[test]
    fn test_config_from_ado_string_with_port() {
        let config = MssqlConfig::from_connection_string(
            "Server=localhost,1434;Database=mydb;User Id=sa;Password=pass;",
        )
        .unwrap();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 1434);
    }

    #[test]
    fn test_config_builder() {
        let config = MssqlConfig::builder()
            .host("myserver")
            .port(1434)
            .database("mydb")
            .username("sa")
            .password("Password123!")
            .trust_cert(true)
            .build()
            .unwrap();

        assert_eq!(config.host, "myserver");
        assert_eq!(config.port, 1434);
        assert_eq!(config.database, "mydb");
        assert!(config.trust_cert);
    }

    #[test]
    fn test_config_builder_missing_database() {
        let result = MssqlConfig::builder()
            .host("localhost")
            .username("sa")
            .password("pass")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_config_to_connection_string() {
        let config = MssqlConfig::builder()
            .host("localhost")
            .database("mydb")
            .username("sa")
            .password("pass")
            .build()
            .unwrap();

        let conn_str = config.to_connection_string();
        assert!(conn_str.contains("Server=localhost"));
        assert!(conn_str.contains("Database=mydb"));
        assert!(conn_str.contains("User Id=sa"));
    }

    #[test]
    fn test_encryption_mode_conversion() {
        assert_eq!(
            EncryptionLevel::from(EncryptionMode::On),
            EncryptionLevel::On
        );
        assert_eq!(
            EncryptionLevel::from(EncryptionMode::Off),
            EncryptionLevel::Off
        );
        assert_eq!(
            EncryptionLevel::from(EncryptionMode::Required),
            EncryptionLevel::Required
        );
    }
}
