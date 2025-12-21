//! Type conversion utilities for SQLite.

use rusqlite::types::{FromSql, FromSqlError, Value, ValueRef};
use serde_json::Value as JsonValue;

use prax_query::filter::FilterValue;

/// Convert a FilterValue to a SQLite Value.
pub fn filter_value_to_sqlite(value: &FilterValue) -> Value {
    match value {
        FilterValue::Null => Value::Null,
        FilterValue::Bool(b) => Value::Integer(i64::from(*b)),
        FilterValue::Int(i) => Value::Integer(*i),
        FilterValue::Float(f) => Value::Real(*f),
        FilterValue::String(s) => Value::Text(s.clone()),
        FilterValue::Json(j) => Value::Text(j.to_string()),
        FilterValue::List(list) => {
            // Serialize list as JSON
            let json_array: Vec<JsonValue> = list
                .iter()
                .map(|v| match v {
                    FilterValue::Null => JsonValue::Null,
                    FilterValue::Bool(b) => JsonValue::Bool(*b),
                    FilterValue::Int(i) => JsonValue::Number((*i).into()),
                    FilterValue::Float(f) => serde_json::Number::from_f64(*f)
                        .map(JsonValue::Number)
                        .unwrap_or(JsonValue::Null),
                    FilterValue::String(s) => JsonValue::String(s.clone()),
                    FilterValue::Json(j) => j.clone(),
                    FilterValue::List(_) => JsonValue::Null, // Nested lists not directly supported
                })
                .collect();
            Value::Text(serde_json::to_string(&json_array).unwrap_or_default())
        }
    }
}

/// Convert a SQLite ValueRef to a JSON Value.
pub fn from_sqlite_value(value: ValueRef<'_>) -> JsonValue {
    match value {
        ValueRef::Null => JsonValue::Null,
        ValueRef::Integer(i) => JsonValue::Number(i.into()),
        ValueRef::Real(f) => serde_json::Number::from_f64(f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        ValueRef::Text(bytes) => {
            let s = String::from_utf8_lossy(bytes).to_string();
            // Try to parse as JSON
            if s.starts_with('{') || s.starts_with('[') {
                serde_json::from_str(&s).unwrap_or(JsonValue::String(s))
            } else {
                JsonValue::String(s)
            }
        }
        ValueRef::Blob(bytes) => {
            // Try to decode as UTF-8 string first
            match std::str::from_utf8(bytes) {
                Ok(s) => JsonValue::String(s.to_string()),
                Err(_) => {
                    // Binary data, encode as base64
                    JsonValue::String(base64_encode(bytes))
                }
            }
        }
    }
}

/// Get a JSON value from a row at the given column index.
pub fn get_value_at_index(row: &rusqlite::Row<'_>, index: usize) -> JsonValue {
    // Try to get the value as each type
    if let Ok(v) = row.get_ref(index) {
        from_sqlite_value(v)
    } else {
        JsonValue::Null
    }
}

/// Simple base64 encoding for binary data.
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut i = 0;

    while i < data.len() {
        let b0 = data[i];
        let b1 = data.get(i + 1).copied().unwrap_or(0);
        let b2 = data.get(i + 2).copied().unwrap_or(0);

        result.push(ALPHABET[(b0 >> 2) as usize] as char);
        result.push(ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if i + 1 < data.len() {
            result.push(ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }

        if i + 2 < data.len() {
            result.push(ALPHABET[(b2 & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

/// A newtype wrapper for JSON values stored in SQLite.
#[derive(Debug, Clone)]
pub struct JsonColumn(pub JsonValue);

impl FromSql for JsonColumn {
    fn column_result(value: ValueRef<'_>) -> Result<Self, FromSqlError> {
        match value {
            ValueRef::Text(bytes) => {
                let s = std::str::from_utf8(bytes).map_err(|e| FromSqlError::Other(Box::new(e)))?;
                let json: JsonValue =
                    serde_json::from_str(s).map_err(|e| FromSqlError::Other(Box::new(e)))?;
                Ok(JsonColumn(json))
            }
            ValueRef::Null => Ok(JsonColumn(JsonValue::Null)),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_value_to_sqlite_null() {
        let result = filter_value_to_sqlite(&FilterValue::Null);
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn test_filter_value_to_sqlite_bool() {
        let result = filter_value_to_sqlite(&FilterValue::Bool(true));
        assert!(matches!(result, Value::Integer(1)));

        let result = filter_value_to_sqlite(&FilterValue::Bool(false));
        assert!(matches!(result, Value::Integer(0)));
    }

    #[test]
    fn test_filter_value_to_sqlite_int() {
        let result = filter_value_to_sqlite(&FilterValue::Int(42));
        assert!(matches!(result, Value::Integer(42)));
    }

    #[test]
    fn test_filter_value_to_sqlite_float() {
        let result = filter_value_to_sqlite(&FilterValue::Float(3.14));
        match result {
            Value::Real(f) => assert!((f - 3.14).abs() < f64::EPSILON),
            _ => panic!("Expected Real"),
        }
    }

    #[test]
    fn test_filter_value_to_sqlite_string() {
        let result = filter_value_to_sqlite(&FilterValue::String("hello".to_string()));
        assert!(matches!(result, Value::Text(s) if s == "hello"));
    }

    #[test]
    fn test_from_sqlite_value_null() {
        let result = from_sqlite_value(ValueRef::Null);
        assert_eq!(result, JsonValue::Null);
    }

    #[test]
    fn test_from_sqlite_value_integer() {
        let result = from_sqlite_value(ValueRef::Integer(42));
        assert_eq!(result, JsonValue::Number(42.into()));
    }

    #[test]
    fn test_from_sqlite_value_real() {
        let result = from_sqlite_value(ValueRef::Real(3.14));
        if let JsonValue::Number(n) = result {
            assert!((n.as_f64().unwrap() - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("Expected Number");
        }
    }

    #[test]
    fn test_from_sqlite_value_text() {
        let result = from_sqlite_value(ValueRef::Text(b"hello"));
        assert_eq!(result, JsonValue::String("hello".to_string()));
    }

    #[test]
    fn test_from_sqlite_value_json_text() {
        let result = from_sqlite_value(ValueRef::Text(b"{\"key\": \"value\"}"));
        if let JsonValue::Object(map) = result {
            assert_eq!(
                map.get("key"),
                Some(&JsonValue::String("value".to_string()))
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_base64_encode() {
        let result = base64_encode(b"Hello");
        assert_eq!(result, "SGVsbG8=");
    }
}
