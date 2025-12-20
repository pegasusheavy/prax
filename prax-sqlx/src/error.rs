//! Error types for SQLx operations.

use prax_query::QueryError;
use thiserror::Error;

/// Result type alias for SQLx operations.
pub type SqlxResult<T> = Result<T, SqlxError>;

/// Errors that can occur during SQLx operations.
#[derive(Error, Debug)]
pub enum SqlxError {
    /// SQLx database error
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Query execution error
    #[error("Query error: {0}")]
    Query(String),

    /// Row deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Type conversion error
    #[error("Type conversion error: {0}")]
    TypeConversion(String),

    /// Pool error
    #[error("Pool error: {0}")]
    Pool(String),

    /// Timeout error
    #[error("Operation timed out after {0}ms")]
    Timeout(u64),

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<SqlxError> for QueryError {
    fn from(err: SqlxError) -> Self {
        match err {
            SqlxError::Sqlx(e) => {
                let msg = e.to_string();
                if msg.contains("connection") {
                    QueryError::connection(msg)
                } else if msg.contains("constraint") || msg.contains("duplicate") {
                    QueryError::constraint_violation("unknown", msg)
                } else {
                    QueryError::database(msg)
                }
            }
            SqlxError::Config(msg) => QueryError::connection(msg),
            SqlxError::Connection(msg) => QueryError::connection(msg),
            SqlxError::Query(msg) => QueryError::database(msg),
            SqlxError::Deserialization(msg) => QueryError::serialization(msg),
            SqlxError::TypeConversion(msg) => QueryError::serialization(msg),
            SqlxError::Pool(msg) => QueryError::connection(msg),
            SqlxError::Timeout(ms) => QueryError::timeout(ms),
            SqlxError::Migration(msg) => QueryError::database(msg),
            SqlxError::Internal(msg) => QueryError::internal(msg),
        }
    }
}

impl SqlxError {
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

    /// Create a pool error.
    pub fn pool(msg: impl Into<String>) -> Self {
        Self::Pool(msg.into())
    }

    /// Create a timeout error.
    pub fn timeout(ms: u64) -> Self {
        Self::Timeout(ms)
    }

    /// Create a type conversion error.
    pub fn type_conversion(msg: impl Into<String>) -> Self {
        Self::TypeConversion(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = SqlxError::config("test config error");
        assert!(matches!(err, SqlxError::Config(_)));

        let err = SqlxError::connection("test connection error");
        assert!(matches!(err, SqlxError::Connection(_)));

        let err = SqlxError::timeout(5000);
        assert!(matches!(err, SqlxError::Timeout(5000)));
    }

    #[test]
    fn test_error_to_query_error() {
        let err = SqlxError::connection("connection failed");
        let query_err: QueryError = err.into();
        assert!(query_err.to_string().contains("connection"));

        let err = SqlxError::timeout(1000);
        let query_err: QueryError = err.into();
        assert!(query_err.to_string().contains("timeout") || query_err.to_string().contains("1000"));
    }
}
