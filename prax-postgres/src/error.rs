//! Error types for PostgreSQL operations.

use prax_query::QueryError;
use thiserror::Error;

/// Result type for PostgreSQL operations.
pub type PgResult<T> = Result<T, PgError>;

/// Errors that can occur during PostgreSQL operations.
#[derive(Error, Debug)]
pub enum PgError {
    /// Connection pool error.
    #[error("pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),

    /// PostgreSQL error.
    #[error("postgres error: {0}")]
    Postgres(#[from] tokio_postgres::Error),

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
}

impl PgError {
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

    /// Check if this is a connection error.
    pub fn is_connection_error(&self) -> bool {
        matches!(self, Self::Pool(_) | Self::Connection(_))
    }

    /// Check if this is a timeout error.
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout(_))
    }
}

impl From<PgError> for QueryError {
    fn from(err: PgError) -> Self {
        match err {
            PgError::Pool(e) => QueryError::Connection(e.to_string()),
            PgError::Postgres(e) => {
                // Try to categorize PostgreSQL errors
                let code = e.code();
                if let Some(code) = code {
                    let code_str = code.code();
                    // Unique violation
                    if code_str == "23505" {
                        return QueryError::ConstraintViolation {
                            model: String::new(),
                            message: e.to_string(),
                        };
                    }
                    // Foreign key violation
                    if code_str == "23503" {
                        return QueryError::ConstraintViolation {
                            model: String::new(),
                            message: e.to_string(),
                        };
                    }
                    // Not null violation
                    if code_str == "23502" {
                        return QueryError::InvalidInput {
                            field: String::new(),
                            message: e.to_string(),
                        };
                    }
                }
                QueryError::Database(e.to_string())
            }
            PgError::Config(msg) => QueryError::Connection(msg),
            PgError::Connection(msg) => QueryError::Connection(msg),
            PgError::Query(msg) => QueryError::Database(msg),
            PgError::Deserialization(msg) => QueryError::Serialization(msg),
            PgError::TypeConversion(msg) => QueryError::Serialization(msg),
            PgError::Timeout(ms) => QueryError::Timeout { duration_ms: ms },
            PgError::Internal(msg) => QueryError::Internal(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = PgError::config("invalid URL");
        assert!(matches!(err, PgError::Config(_)));

        let err = PgError::connection("connection refused");
        assert!(err.is_connection_error());

        let err = PgError::Timeout(5000);
        assert!(err.is_timeout());
    }

    #[test]
    fn test_into_query_error() {
        let pg_err = PgError::Timeout(1000);
        let query_err: QueryError = pg_err.into();
        assert!(query_err.is_timeout());
    }
}

