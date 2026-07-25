//! MySQL configuration.

use std::time::Duration;

use mysql_async::{OptsBuilder, SslOpts};
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
    ///
    /// Not currently applied: `mysql_async` 0.36 does not expose a TCP
    /// connect-timeout option on `OptsBuilder`, so this value is parsed and
    /// stored but has no effect on connections built by
    /// [`MysqlConfig::to_opts_builder`].
    pub connect_timeout: Option<Duration>,
    /// SSL mode.
    pub ssl_mode: SslMode,
    /// Additional connection options.
    pub options: Vec<(String, String)>,
}

/// SSL mode for MySQL connections.
///
/// Behavior note: `mysql_async` requests the `CLIENT_SSL` capability whenever
/// any `SslOpts` are set, and the handshake fails if the server does not
/// support TLS. Every mode except [`SslMode::Disabled`] therefore *requires*
/// TLS at connect time — there is no opportunistic "try TLS, fall back to
/// plaintext" mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SslMode {
    /// No TLS; plaintext connection (deliberate).
    #[default]
    Disabled,
    /// TLS encryption without certificate verification.
    ///
    /// Unlike MySQL's `PREFERRED`, this does *not* fall back to plaintext:
    /// the connection fails if the server does not support TLS (see the
    /// enum-level note).
    Preferred,
    /// Require TLS encryption, but do not verify the server certificate.
    Required,
    /// Require TLS and verify the certificate chain against the built-in
    /// webpki root certificates; the server hostname is not verified.
    VerifyCa,
    /// Require TLS and fully verify the certificate chain (against the
    /// built-in webpki roots) and the server hostname.
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

        // `connect_timeout` is not applied here: mysql_async 0.36 has no TCP
        // connect-timeout option on `OptsBuilder` (see the field docs).

        // Configure SSL based on mode.
        if let Some(ssl_opts) = ssl_opts_for_mode(self.ssl_mode) {
            builder = builder.ssl_opts(ssl_opts);
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

/// Build the `mysql_async` SSL options corresponding to `mode`.
///
/// Returns `None` when TLS must not be used at all. See the [`SslMode`]
/// docs for the exact semantics of each mode.
fn ssl_opts_for_mode(mode: SslMode) -> Option<SslOpts> {
    match mode {
        SslMode::Disabled => None,
        // Encryption without any verification. mysql_async has no
        // opportunistic TLS mode, so `Preferred` maps to the same options
        // as `Required`: the handshake fails if the server lacks TLS.
        SslMode::Preferred | SslMode::Required => Some(
            SslOpts::default()
                .with_danger_accept_invalid_certs(true)
                .with_danger_skip_domain_validation(true),
        ),
        // Chain verified against the built-in webpki roots; hostname skipped.
        SslMode::VerifyCa => Some(SslOpts::default().with_danger_skip_domain_validation(true)),
        // Chain and hostname verified against the built-in webpki roots.
        SslMode::VerifyIdentity => Some(SslOpts::default()),
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

        // `connect_timeout` cannot be introspected on the built opts because
        // mysql_async 0.36 does not expose (or apply) a TCP connect timeout;
        // assert only that building opts does not panic.
        let _ = config.to_opts_builder();
    }

    #[test]
    fn test_to_opts_builder_honors_ssl_mode() {
        use mysql_async::Opts;

        // Disabled: no TLS, and the unrelated prefer_socket knob untouched.
        let config = MysqlConfig::new("mydb").ssl_mode(SslMode::Disabled);
        let opts = Opts::from(config.to_opts_builder());
        assert!(opts.ssl_opts().is_none());
        let default_opts = Opts::from(OptsBuilder::default());
        assert_eq!(opts.prefer_socket(), default_opts.prefer_socket());

        // Preferred/Required: encryption without certificate verification.
        for mode in [SslMode::Preferred, SslMode::Required] {
            let config = MysqlConfig::new("mydb").ssl_mode(mode);
            let opts = Opts::from(config.to_opts_builder());
            let ssl = opts.ssl_opts().expect("ssl_opts must be set");
            assert!(ssl.accept_invalid_certs());
            assert!(ssl.skip_domain_validation());
        }

        // VerifyCa: chain verified against the built-in webpki roots,
        // hostname check skipped.
        let config = MysqlConfig::new("mydb").ssl_mode(SslMode::VerifyCa);
        let opts = Opts::from(config.to_opts_builder());
        let ssl = opts.ssl_opts().expect("ssl_opts must be set");
        assert!(!ssl.accept_invalid_certs());
        assert!(ssl.skip_domain_validation());
        assert!(!ssl.disable_built_in_roots());
        assert!(ssl.root_certs().is_empty());

        // VerifyIdentity: full chain + hostname verification.
        let config = MysqlConfig::new("mydb").ssl_mode(SslMode::VerifyIdentity);
        let opts = Opts::from(config.to_opts_builder());
        let ssl = opts.ssl_opts().expect("ssl_opts must be set");
        assert!(!ssl.accept_invalid_certs());
        assert!(!ssl.skip_domain_validation());
        assert!(!ssl.disable_built_in_roots());
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
