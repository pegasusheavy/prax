//! Error types for MySQL operations.

use std::fmt;

use prax_query::error::QueryError;

/// Result type for MySQL operations.
pub type MysqlResult<T> = Result<T, MysqlError>;

/// Error type for MySQL operations.
#[derive(Debug)]
pub enum MysqlError {
    /// Pool error.
    Pool(String),
    /// MySQL driver error.
    Mysql(mysql_async::Error),
    /// Configuration error.
    Config(String),
    /// Connection error.
    Connection(String),
    /// Query error.
    Query(String),
    /// Deserialization error.
    Deserialization(String),
    /// Type conversion error.
    TypeConversion(String),
    /// Timeout error.
    Timeout(String),
    /// Internal error.
    Internal(String),
}

impl MysqlError {
    /// Create a pool error.
    pub fn pool(msg: impl Into<String>) -> Self {
        Self::Pool(msg.into())
    }

    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a connection error.
    pub fn connection(msg: impl Into<String>) -> Self {
        Self::Connection(msg.into())
    }

    /// Create a query error.
    pub fn query(msg: impl Into<String>) -> Self {
        Self::Query(msg.into())
    }

    /// Create a deserialization error.
    pub fn deserialization(msg: impl Into<String>) -> Self {
        Self::Deserialization(msg.into())
    }

    /// Create a type conversion error.
    pub fn type_conversion(msg: impl Into<String>) -> Self {
        Self::TypeConversion(msg.into())
    }

    /// Create a timeout error.
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

impl fmt::Display for MysqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(msg) => write!(f, "Pool error: {}", msg),
            Self::Mysql(e) => write!(f, "MySQL error: {}", e),
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Connection(msg) => write!(f, "Connection error: {}", msg),
            Self::Query(msg) => write!(f, "Query error: {}", msg),
            Self::Deserialization(msg) => write!(f, "Deserialization error: {}", msg),
            Self::TypeConversion(msg) => write!(f, "Type conversion error: {}", msg),
            Self::Timeout(msg) => write!(f, "Timeout error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for MysqlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Mysql(e) => Some(e),
            _ => None,
        }
    }
}

impl From<mysql_async::Error> for MysqlError {
    fn from(err: mysql_async::Error) -> Self {
        Self::Mysql(err)
    }
}

impl From<MysqlError> for QueryError {
    fn from(err: MysqlError) -> Self {
        match err {
            MysqlError::Pool(msg) => QueryError::connection(msg),
            MysqlError::Mysql(e) => QueryError::database(e.to_string()),
            MysqlError::Config(msg) => QueryError::internal(format!("config: {}", msg)),
            MysqlError::Connection(msg) => QueryError::connection(msg),
            MysqlError::Query(msg) => QueryError::database(msg),
            MysqlError::Deserialization(msg) => QueryError::serialization(msg),
            MysqlError::TypeConversion(msg) => QueryError::serialization(format!("type: {}", msg)),
            MysqlError::Timeout(_) => QueryError::timeout(5000), // Default timeout duration
            MysqlError::Internal(msg) => QueryError::internal(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = MysqlError::config("invalid url");
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("invalid url"));
    }

    #[test]
    fn test_error_constructors() {
        assert!(matches!(MysqlError::pool("test"), MysqlError::Pool(_)));
        assert!(matches!(MysqlError::config("test"), MysqlError::Config(_)));
        assert!(matches!(
            MysqlError::connection("test"),
            MysqlError::Connection(_)
        ));
        assert!(matches!(MysqlError::query("test"), MysqlError::Query(_)));
    }

    #[test]
    fn test_error_conversion() {
        let err = MysqlError::timeout("connection timed out");
        let query_err: QueryError = err.into();
        assert!(query_err.is_timeout());
    }
}
