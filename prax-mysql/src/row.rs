//! Row deserialization traits for MySQL.

use mysql_async::Row;

/// Trait for converting a MySQL row to a Rust type.
///
/// This trait is implemented for types that can be deserialized from a MySQL row.
pub trait FromMysqlRow: Sized {
    /// Convert a MySQL row to this type.
    fn from_row(row: &Row) -> Result<Self, FromMysqlRowError>;
}

/// Error type for row deserialization.
#[derive(Debug)]
pub struct FromMysqlRowError {
    /// The error message.
    pub message: String,
    /// The column that caused the error, if known.
    pub column: Option<String>,
}

impl FromMysqlRowError {
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

impl std::fmt::Display for FromMysqlRowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref column) = self.column {
            write!(f, "column '{}': {}", column, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for FromMysqlRowError {}

/// Implement FromMysqlRow for common types using serde_json.
impl FromMysqlRow for serde_json::Value {
    fn from_row(row: &Row) -> Result<Self, FromMysqlRowError> {
        use serde_json::{Map, Value as JsonValue};

        use crate::types::from_mysql_value;

        let mut map = Map::new();

        for (i, column) in row.columns_ref().iter().enumerate() {
            let name = column.name_str().to_string();
            let value: Option<mysql_async::Value> = row.get(i);

            let json_value = match value {
                Some(v) => from_mysql_value(v),
                None => JsonValue::Null,
            };

            map.insert(name, json_value);
        }

        Ok(JsonValue::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_mysql_row_error_new() {
        let err = FromMysqlRowError::new("test error");
        assert_eq!(err.message, "test error");
        assert!(err.column.is_none());
    }

    #[test]
    fn test_from_mysql_row_error_with_column() {
        let err = FromMysqlRowError::with_column("invalid type", "user_id");
        assert_eq!(err.message, "invalid type");
        assert_eq!(err.column, Some("user_id".to_string()));
    }

    #[test]
    fn test_from_mysql_row_error_display() {
        let err = FromMysqlRowError::with_column("missing value", "email");
        let display = format!("{}", err);
        assert!(display.contains("email"));
        assert!(display.contains("missing value"));
    }
}
