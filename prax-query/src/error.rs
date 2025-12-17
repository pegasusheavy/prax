//! Error types for query operations.

use thiserror::Error;

/// Result type for query operations.
pub type QueryResult<T> = Result<T, QueryError>;

/// Errors that can occur during query operations.
#[derive(Error, Debug)]
pub enum QueryError {
    /// Record not found.
    #[error("record not found: {model}")]
    NotFound {
        /// The model that was queried.
        model: String,
    },

    /// Multiple records found when expecting one.
    #[error("expected unique record but found multiple: {model}")]
    NotUnique {
        /// The model that was queried.
        model: String,
    },

    /// Constraint violation (unique, foreign key, etc.).
    #[error("constraint violation on {model}: {message}")]
    ConstraintViolation {
        /// The model affected.
        model: String,
        /// Description of the constraint that was violated.
        message: String,
    },

    /// Invalid input data.
    #[error("invalid input for {field}: {message}")]
    InvalidInput {
        /// The field with invalid input.
        field: String,
        /// Description of what was wrong.
        message: String,
    },

    /// Connection error.
    #[error("connection error: {0}")]
    Connection(String),

    /// Transaction error.
    #[error("transaction error: {0}")]
    Transaction(String),

    /// Query timeout.
    #[error("query timed out after {duration_ms}ms")]
    Timeout {
        /// Duration in milliseconds before timeout.
        duration_ms: u64,
    },

    /// SQL generation error.
    #[error("SQL generation error: {0}")]
    SqlGeneration(String),

    /// Database error.
    #[error("database error: {0}")]
    Database(String),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl QueryError {
    /// Create a not found error.
    pub fn not_found(model: impl Into<String>) -> Self {
        Self::NotFound {
            model: model.into(),
        }
    }

    /// Create a not unique error.
    pub fn not_unique(model: impl Into<String>) -> Self {
        Self::NotUnique {
            model: model.into(),
        }
    }

    /// Create a constraint violation error.
    pub fn constraint_violation(model: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ConstraintViolation {
            model: model.into(),
            message: message.into(),
        }
    }

    /// Create an invalid input error.
    pub fn invalid_input(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidInput {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a connection error.
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection(message.into())
    }

    /// Create a timeout error.
    pub fn timeout(duration_ms: u64) -> Self {
        Self::Timeout { duration_ms }
    }

    /// Check if this is a not found error.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    /// Check if this is a constraint violation.
    pub fn is_constraint_violation(&self) -> bool {
        matches!(self, Self::ConstraintViolation { .. })
    }

    /// Check if this is a timeout error.
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = QueryError::not_found("User");
        assert!(err.is_not_found());
        assert!(err.to_string().contains("User"));
    }

    #[test]
    fn test_constraint_violation() {
        let err = QueryError::constraint_violation("User", "email must be unique");
        assert!(err.is_constraint_violation());
        assert!(err.to_string().contains("email must be unique"));
    }

    #[test]
    fn test_timeout_error() {
        let err = QueryError::timeout(5000);
        assert!(err.is_timeout());
        assert!(err.to_string().contains("5000"));
    }
}

