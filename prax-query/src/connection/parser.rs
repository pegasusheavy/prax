//! Connection string parser.

use super::{ConnectionError, ConnectionResult};
use std::collections::HashMap;
use tracing::debug;

/// Database driver type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Driver {
    /// PostgreSQL
    Postgres,
    /// MySQL / MariaDB
    MySql,
    /// SQLite
    Sqlite,
}

impl Driver {
    /// Get the default port for this driver.
    pub fn default_port(&self) -> Option<u16> {
        match self {
            Self::Postgres => Some(5432),
            Self::MySql => Some(3306),
            Self::Sqlite => None,
        }
    }

    /// Get the driver name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::MySql => "mysql",
            Self::Sqlite => "sqlite",
        }
    }

    /// Parse driver from URL scheme.
    pub fn from_scheme(scheme: &str) -> ConnectionResult<Self> {
        match scheme.to_lowercase().as_str() {
            "postgres" | "postgresql" => Ok(Self::Postgres),
            "mysql" | "mariadb" => Ok(Self::MySql),
            "sqlite" | "sqlite3" | "file" => Ok(Self::Sqlite),
            other => Err(ConnectionError::UnknownDriver(other.to_string())),
        }
    }
}

impl std::fmt::Display for Driver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A parsed database URL.
#[derive(Debug, Clone)]
pub struct ParsedUrl {
    /// Database driver.
    pub driver: Driver,
    /// Username (if any).
    pub user: Option<String>,
    /// Password (if any).
    pub password: Option<String>,
    /// Host (for network databases).
    pub host: Option<String>,
    /// Port (for network databases).
    pub port: Option<u16>,
    /// Database name or file path.
    pub database: Option<String>,
    /// Query parameters.
    pub params: HashMap<String, String>,
}

impl ParsedUrl {
    /// Check if this is an in-memory SQLite database.
    pub fn is_memory(&self) -> bool {
        self.driver == Driver::Sqlite
            && self
                .database
                .as_ref()
                .map_or(false, |d| d == ":memory:" || d.is_empty())
    }

    /// Get a query parameter.
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    /// Convert back to a URL string.
    pub fn to_url(&self) -> String {
        let mut url = format!("{}://", self.driver.name());

        // Add credentials
        if let Some(ref user) = self.user {
            url.push_str(&url_encode(user));
            if let Some(ref pass) = self.password {
                url.push(':');
                url.push_str(&url_encode(pass));
            }
            url.push('@');
        }

        // Add host/port
        if let Some(ref host) = self.host {
            url.push_str(host);
            if let Some(port) = self.port {
                url.push(':');
                url.push_str(&port.to_string());
            }
        }

        // Add database
        if let Some(ref db) = self.database {
            url.push('/');
            url.push_str(db);
        }

        // Add query params
        if !self.params.is_empty() {
            url.push('?');
            let params: Vec<_> = self
                .params
                .iter()
                .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
                .collect();
            url.push_str(&params.join("&"));
        }

        url
    }
}

/// Connection string parser.
#[derive(Debug, Clone)]
pub struct ConnectionString {
    parsed: ParsedUrl,
    original: String,
}

impl ConnectionString {
    /// Parse a connection URL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use prax_query::connection::ConnectionString;
    ///
    /// // PostgreSQL
    /// let conn = ConnectionString::parse("postgres://user:pass@localhost:5432/mydb").unwrap();
    ///
    /// // MySQL
    /// let conn = ConnectionString::parse("mysql://user:pass@localhost/mydb").unwrap();
    ///
    /// // SQLite
    /// let conn = ConnectionString::parse("sqlite://./data.db").unwrap();
    /// let conn = ConnectionString::parse("sqlite::memory:").unwrap();
    /// ```
    pub fn parse(url: &str) -> ConnectionResult<Self> {
        debug!(url_len = url.len(), "ConnectionString::parse()");
        let original = url.to_string();
        let parsed = parse_url(url)?;
        debug!(driver = %parsed.driver, host = ?parsed.host, database = ?parsed.database, "Connection parsed");
        Ok(Self { parsed, original })
    }

    /// Parse from environment variable.
    pub fn from_env(var: &str) -> ConnectionResult<Self> {
        let url = std::env::var(var).map_err(|_| ConnectionError::EnvNotFound(var.to_string()))?;
        Self::parse(&url)
    }

    /// Parse from DATABASE_URL environment variable.
    pub fn from_database_url() -> ConnectionResult<Self> {
        Self::from_env("DATABASE_URL")
    }

    /// Get the original URL string.
    pub fn as_str(&self) -> &str {
        &self.original
    }

    /// Get the database driver.
    pub fn driver(&self) -> Driver {
        self.parsed.driver
    }

    /// Get the username.
    pub fn user(&self) -> Option<&str> {
        self.parsed.user.as_deref()
    }

    /// Get the password.
    pub fn password(&self) -> Option<&str> {
        self.parsed.password.as_deref()
    }

    /// Get the host.
    pub fn host(&self) -> Option<&str> {
        self.parsed.host.as_deref()
    }

    /// Get the port.
    pub fn port(&self) -> Option<u16> {
        self.parsed.port
    }

    /// Get the port or the default for the driver.
    pub fn port_or_default(&self) -> Option<u16> {
        self.parsed
            .port
            .or_else(|| self.parsed.driver.default_port())
    }

    /// Get the database name.
    pub fn database(&self) -> Option<&str> {
        self.parsed.database.as_deref()
    }

    /// Get a query parameter.
    pub fn param(&self, key: &str) -> Option<&str> {
        self.parsed.param(key)
    }

    /// Get all query parameters.
    pub fn params(&self) -> &HashMap<String, String> {
        &self.parsed.params
    }

    /// Get the parsed URL.
    pub fn parsed(&self) -> &ParsedUrl {
        &self.parsed
    }

    /// Check if this is an in-memory SQLite database.
    pub fn is_memory(&self) -> bool {
        self.parsed.is_memory()
    }

    /// Build a new URL with modified parameters.
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parsed.params.insert(key.into(), value.into());
        self.original = self.parsed.to_url();
        self
    }

    /// Build a new URL without a specific parameter.
    pub fn without_param(mut self, key: &str) -> Self {
        self.parsed.params.remove(key);
        self.original = self.parsed.to_url();
        self
    }
}

/// Parse a database URL into its components.
fn parse_url(url: &str) -> ConnectionResult<ParsedUrl> {
    // Handle SQLite memory shorthand
    if url == "sqlite::memory:" || url == ":memory:" {
        return Ok(ParsedUrl {
            driver: Driver::Sqlite,
            user: None,
            password: None,
            host: None,
            port: None,
            database: Some(":memory:".to_string()),
            params: HashMap::new(),
        });
    }

    // Find scheme
    let (scheme, rest) = url.split_once("://").ok_or_else(|| {
        ConnectionError::InvalidUrl("Missing scheme (e.g., postgres://)".to_string())
    })?;

    let driver = Driver::from_scheme(scheme)?;

    // Handle SQLite specially (path-based)
    if driver == Driver::Sqlite {
        return parse_sqlite_url(rest);
    }

    // Parse network URL
    parse_network_url(driver, rest)
}

fn parse_sqlite_url(rest: &str) -> ConnectionResult<ParsedUrl> {
    // Split off query params
    let (path, params) = parse_query_params(rest);

    let database = if path.is_empty() || path == ":memory:" {
        Some(":memory:".to_string())
    } else {
        Some(url_decode(&path))
    };

    Ok(ParsedUrl {
        driver: Driver::Sqlite,
        user: None,
        password: None,
        host: None,
        port: None,
        database,
        params,
    })
}

fn parse_network_url(driver: Driver, rest: &str) -> ConnectionResult<ParsedUrl> {
    // Split off query params
    let (main, params) = parse_query_params(rest);

    // Split credentials from host
    let (creds, host_part) = if let Some(at_pos) = main.rfind('@') {
        (Some(&main[..at_pos]), &main[at_pos + 1..])
    } else {
        (None, main.as_str())
    };

    // Parse credentials
    let (user, password) = if let Some(creds) = creds {
        if let Some((u, p)) = creds.split_once(':') {
            (Some(url_decode(u)), Some(url_decode(p)))
        } else {
            (Some(url_decode(creds)), None)
        }
    } else {
        (None, None)
    };

    // Split host from database
    let (host_port, database) = if let Some(slash_pos) = host_part.find('/') {
        (
            &host_part[..slash_pos],
            Some(url_decode(&host_part[slash_pos + 1..])),
        )
    } else {
        (host_part, None)
    };

    // Parse host and port
    let (host, port) = if host_port.is_empty() {
        (None, None)
    } else if let Some(colon_pos) = host_port.rfind(':') {
        // Check if it's IPv6 address [::1]
        if host_port.starts_with('[') {
            if let Some(bracket_pos) = host_port.find(']') {
                if colon_pos > bracket_pos {
                    // Port after IPv6 address
                    let port = host_port[colon_pos + 1..].parse().map_err(|_| {
                        ConnectionError::InvalidUrl("Invalid port number".to_string())
                    })?;
                    (Some(host_port[..colon_pos].to_string()), Some(port))
                } else {
                    // No port, just IPv6 address
                    (Some(host_port.to_string()), None)
                }
            } else {
                return Err(ConnectionError::InvalidUrl(
                    "Invalid IPv6 address".to_string(),
                ));
            }
        } else {
            // Regular host:port
            let port = host_port[colon_pos + 1..]
                .parse()
                .map_err(|_| ConnectionError::InvalidUrl("Invalid port number".to_string()))?;
            (Some(host_port[..colon_pos].to_string()), Some(port))
        }
    } else {
        (Some(host_port.to_string()), None)
    };

    Ok(ParsedUrl {
        driver,
        user,
        password,
        host,
        port,
        database,
        params,
    })
}

fn parse_query_params(input: &str) -> (String, HashMap<String, String>) {
    if let Some((main, query)) = input.split_once('?') {
        let params = query
            .split('&')
            .filter_map(|pair| {
                let (key, value) = pair.split_once('=')?;
                Some((url_decode(key), url_decode(value)))
            })
            .collect();
        (main.to_string(), params)
    } else {
        (input.to_string(), HashMap::new())
    }
}

fn url_decode(s: &str) -> String {
    // Simple percent decoding
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    result
}

fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            _ => {
                for byte in c.to_string().bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_postgres_full() {
        let conn = ConnectionString::parse("postgres://user:pass@localhost:5432/mydb").unwrap();
        assert_eq!(conn.driver(), Driver::Postgres);
        assert_eq!(conn.user(), Some("user"));
        assert_eq!(conn.password(), Some("pass"));
        assert_eq!(conn.host(), Some("localhost"));
        assert_eq!(conn.port(), Some(5432));
        assert_eq!(conn.database(), Some("mydb"));
    }

    #[test]
    fn test_parse_postgres_with_params() {
        let conn = ConnectionString::parse(
            "postgres://user:pass@localhost/mydb?sslmode=require&connect_timeout=10",
        )
        .unwrap();
        assert_eq!(conn.param("sslmode"), Some("require"));
        assert_eq!(conn.param("connect_timeout"), Some("10"));
    }

    #[test]
    fn test_parse_postgres_no_password() {
        let conn = ConnectionString::parse("postgres://user@localhost/mydb").unwrap();
        assert_eq!(conn.user(), Some("user"));
        assert_eq!(conn.password(), None);
    }

    #[test]
    fn test_parse_mysql() {
        let conn = ConnectionString::parse("mysql://root:secret@127.0.0.1:3306/testdb").unwrap();
        assert_eq!(conn.driver(), Driver::MySql);
        assert_eq!(conn.host(), Some("127.0.0.1"));
        assert_eq!(conn.port(), Some(3306));
    }

    #[test]
    fn test_parse_mariadb() {
        let conn = ConnectionString::parse("mariadb://user:pass@localhost/db").unwrap();
        assert_eq!(conn.driver(), Driver::MySql);
    }

    #[test]
    fn test_parse_sqlite_file() {
        let conn = ConnectionString::parse("sqlite://./data/app.db").unwrap();
        assert_eq!(conn.driver(), Driver::Sqlite);
        assert_eq!(conn.database(), Some("./data/app.db"));
    }

    #[test]
    fn test_parse_sqlite_memory() {
        let conn = ConnectionString::parse("sqlite::memory:").unwrap();
        assert_eq!(conn.driver(), Driver::Sqlite);
        assert!(conn.is_memory());

        let conn = ConnectionString::parse("sqlite://:memory:").unwrap();
        assert!(conn.is_memory());
    }

    #[test]
    fn test_parse_special_characters() {
        let conn = ConnectionString::parse("postgres://user:p%40ss%3Aword@localhost/db").unwrap();
        assert_eq!(conn.password(), Some("p@ss:word"));
    }

    #[test]
    fn test_default_port() {
        assert_eq!(Driver::Postgres.default_port(), Some(5432));
        assert_eq!(Driver::MySql.default_port(), Some(3306));
        assert_eq!(Driver::Sqlite.default_port(), None);
    }

    #[test]
    fn test_port_or_default() {
        let conn = ConnectionString::parse("postgres://localhost/db").unwrap();
        assert_eq!(conn.port(), None);
        assert_eq!(conn.port_or_default(), Some(5432));
    }

    #[test]
    fn test_with_param() {
        let conn = ConnectionString::parse("postgres://localhost/db").unwrap();
        let conn = conn.with_param("sslmode", "require");
        assert_eq!(conn.param("sslmode"), Some("require"));
    }

    #[test]
    fn test_to_url_roundtrip() {
        let original = "postgres://user:pass@localhost:5432/mydb?sslmode=require";
        let conn = ConnectionString::parse(original).unwrap();
        let rebuilt = conn.parsed().to_url();
        assert!(rebuilt.contains("postgres://"));
        assert!(rebuilt.contains("localhost:5432"));
        assert!(rebuilt.contains("sslmode=require"));
    }

    #[test]
    fn test_invalid_url() {
        assert!(ConnectionString::parse("not-a-url").is_err());
        assert!(ConnectionString::parse("unknown://localhost").is_err());
    }
}
