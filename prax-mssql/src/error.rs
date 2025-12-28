//! Error types for Microsoft SQL Server operations.

use prax_query::QueryError;
use thiserror::Error;

/// Result type for MSSQL operations.
pub type MssqlResult<T> = Result<T, MssqlError>;

/// Errors that can occur during MSSQL operations.
#[derive(Error, Debug)]
pub enum MssqlError {
    /// Connection pool error.
    #[error("pool error: {0}")]
    Pool(String),

    /// Tiberius/SQL Server error.
    #[error("sql server error: {0}")]
    SqlServer(#[from] tiberius::error::Error),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),

    /// Connection error.
    #[error("connection error: {0}")]
    Connection(String),

    /// Query execution error.
    #[error("query error: {0}")]
    Query(String),

    /// Row deserialization error.
    #[error("deserialization error: {0}")]
    Deserialization(String),

    /// Type conversion error.
    #[error("type conversion error: {0}")]
    TypeConversion(String),

    /// Timeout error.
    #[error("operation timed out after {0}ms")]
    Timeout(u64),

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),

    /// RLS policy error.
    #[error("rls policy error: {0}")]
    RlsPolicy(String),
}

impl MssqlError {
    /// Create a pool error.
    pub fn pool(message: impl Into<String>) -> Self {
        Self::Pool(message.into())
    }

    /// Create a configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Create a connection error.
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection(message.into())
    }

    /// Create a query error.
    pub fn query(message: impl Into<String>) -> Self {
        Self::Query(message.into())
    }

    /// Create a deserialization error.
    pub fn deserialization(message: impl Into<String>) -> Self {
        Self::Deserialization(message.into())
    }

    /// Create a type conversion error.
    pub fn type_conversion(message: impl Into<String>) -> Self {
        Self::TypeConversion(message.into())
    }

    /// Create an RLS policy error.
    pub fn rls_policy(message: impl Into<String>) -> Self {
        Self::RlsPolicy(message.into())
    }

    /// Check if this is a connection error.
    pub fn is_connection_error(&self) -> bool {
        matches!(self, Self::Pool(_) | Self::Connection(_))
    }

    /// Check if this is a timeout error.
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout(_))
    }
}

impl<E> From<bb8::RunError<E>> for MssqlError
where
    E: std::error::Error,
{
    fn from(err: bb8::RunError<E>) -> Self {
        match err {
            bb8::RunError::User(e) => MssqlError::Pool(e.to_string()),
            bb8::RunError::TimedOut => MssqlError::Timeout(30000), // Default 30s
        }
    }
}

impl From<MssqlError> for QueryError {
    fn from(err: MssqlError) -> Self {
        match err {
            MssqlError::Pool(msg) => QueryError::connection(msg),
            MssqlError::SqlServer(e) => {
                // Try to categorize SQL Server errors by error number
                let msg = e.to_string();

                // Unique constraint violation (error 2627)
                if msg.contains("2627") || msg.contains("unique") || msg.contains("duplicate") {
                    return QueryError::constraint_violation("", msg);
                }

                // Foreign key violation (error 547)
                if msg.contains("547") || msg.contains("foreign key") {
                    return QueryError::constraint_violation("", msg);
                }

                // Not null violation (error 515)
                if msg.contains("515") || msg.contains("cannot insert") {
                    return QueryError::invalid_input("", msg);
                }

                QueryError::database(msg)
            }
            MssqlError::Config(msg) => QueryError::connection(msg),
            MssqlError::Connection(msg) => QueryError::connection(msg),
            MssqlError::Query(msg) => QueryError::database(msg),
            MssqlError::Deserialization(msg) => QueryError::serialization(msg),
            MssqlError::TypeConversion(msg) => QueryError::serialization(msg),
            MssqlError::Timeout(ms) => QueryError::timeout(ms),
            MssqlError::Internal(msg) => QueryError::internal(msg),
            MssqlError::RlsPolicy(msg) => QueryError::database(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = MssqlError::config("invalid connection string");
        assert!(matches!(err, MssqlError::Config(_)));

        let err = MssqlError::connection("connection refused");
        assert!(err.is_connection_error());

        let err = MssqlError::Timeout(5000);
        assert!(err.is_timeout());
    }

    #[test]
    fn test_into_query_error() {
        let mssql_err = MssqlError::Timeout(1000);
        let query_err: QueryError = mssql_err.into();
        assert!(query_err.is_timeout());
    }

    #[test]
    fn test_error_display() {
        let err = MssqlError::config("test error");
        assert_eq!(err.to_string(), "configuration error: test error");

        let err = MssqlError::Pool("pool exhausted".to_string());
        assert_eq!(err.to_string(), "pool error: pool exhausted");
    }
}
