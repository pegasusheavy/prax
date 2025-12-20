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

        use mysql_async::Value;
        use serde_json::{Map, Value as JsonValue};

        let mut map = Map::new();

        for (i, column) in row.columns_ref().iter().enumerate() {
            let name = column.name_str().to_string();
            let value: Option<Value> = row.get(i);

            let json_value = match value {
                Some(Value::NULL) | None => JsonValue::Null,
                Some(Value::Bytes(bytes)) => {
                    match String::from_utf8(bytes) {
                        Ok(s) => {
                            // Try to parse as JSON
                            serde_json::from_str(&s).unwrap_or(JsonValue::String(s))
                        }
                        Err(e) => {
                            // Binary data
                            JsonValue::String(format!("<binary {} bytes>", e.into_bytes().len()))
                        }
                    }
                }
                Some(Value::Int(i)) => JsonValue::Number(i.into()),
                Some(Value::UInt(u)) => JsonValue::Number(u.into()),
                Some(Value::Float(f)) => {
                    serde_json::Number::from_f64(f64::from(f))
                        .map(JsonValue::Number)
                        .unwrap_or(JsonValue::Null)
                }
                Some(Value::Double(d)) => {
                    serde_json::Number::from_f64(d)
                        .map(JsonValue::Number)
                        .unwrap_or(JsonValue::Null)
                }
                Some(Value::Date(year, month, day, hour, minute, second, micro)) => {
                    let datetime = format!(
                        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}",
                        year, month, day, hour, minute, second, micro
                    );
                    JsonValue::String(datetime)
                }
                Some(Value::Time(is_neg, days, hours, minutes, seconds, micro)) => {
                    let sign = if is_neg { "-" } else { "" };
                    let time = format!(
                        "{}{}:{:02}:{:02}.{:06}",
                        sign,
                        days * 24 + u32::from(hours),
                        minutes,
                        seconds,
                        micro
                    );
                    JsonValue::String(time)
                }
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
