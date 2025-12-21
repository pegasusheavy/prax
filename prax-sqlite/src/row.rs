//! Row deserialization traits for SQLite.

use rusqlite::Row;
use serde_json::Value as JsonValue;

/// Trait for converting a SQLite row to a Rust type.
///
/// This trait is implemented for types that can be deserialized from a SQLite row.
pub trait FromSqliteRow: Sized {
    /// Convert a SQLite row to this type.
    fn from_row(row: &Row<'_>) -> Result<Self, FromSqliteRowError>;
}

/// Error type for row deserialization.
#[derive(Debug)]
pub struct FromSqliteRowError {
    /// The error message.
    pub message: String,
    /// The column that caused the error, if known.
    pub column: Option<String>,
}

impl FromSqliteRowError {
    /// Create a new error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            column: None,
        }
    }

    /// Create a new error with a column name.
    pub fn with_column(message: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            column: Some(column.into()),
        }
    }
}

impl std::fmt::Display for FromSqliteRowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref column) = self.column {
            write!(f, "column '{}': {}", column, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for FromSqliteRowError {}

impl From<rusqlite::Error> for FromSqliteRowError {
    fn from(err: rusqlite::Error) -> Self {
        Self::new(err.to_string())
    }
}

/// Implement FromSqliteRow for JSON values.
impl FromSqliteRow for JsonValue {
    fn from_row(row: &Row<'_>) -> Result<Self, FromSqliteRowError> {
        let column_count = row.as_ref().column_count();
        let mut map = serde_json::Map::new();

        for i in 0..column_count {
            let name = row
                .as_ref()
                .column_name(i)
                .map_err(|e| FromSqliteRowError::new(e.to_string()))?
                .to_string();
            let value = crate::types::get_value_at_index(row, i);
            map.insert(name, value);
        }

        Ok(JsonValue::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_sqlite_row_error_new() {
        let err = FromSqliteRowError::new("test error");
        assert_eq!(err.message, "test error");
        assert!(err.column.is_none());
    }

    #[test]
    fn test_from_sqlite_row_error_with_column() {
        let err = FromSqliteRowError::with_column("invalid type", "user_id");
        assert_eq!(err.message, "invalid type");
        assert_eq!(err.column, Some("user_id".to_string()));
    }

    #[test]
    fn test_from_sqlite_row_error_display() {
        let err = FromSqliteRowError::with_column("missing value", "email");
        let display = format!("{}", err);
        assert!(display.contains("email"));
        assert!(display.contains("missing value"));
    }
}
