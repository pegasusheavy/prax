//! Row deserialization for `ScyllaDB` results.

use scylla::frame::response::result::{CqlValue, Row};
use serde::de::DeserializeOwned;

use crate::error::{ScyllaError, ScyllaResult};
use crate::types::ScyllaValue;

/// Trait for types that can be constructed from a `ScyllaDB` row.
pub trait FromScyllaRow: Sized {
    /// Construct an instance from a row.
    fn from_row(row: &Row) -> ScyllaResult<Self>;
}

/// A helper for extracting values from a row by index.
pub struct RowAccessor<'a> {
    row: &'a Row,
}

impl<'a> RowAccessor<'a> {
    /// Create a new accessor for a row.
    #[must_use]
    pub fn new(row: &'a Row) -> Self {
        Self { row }
    }

    /// Get a value by column index.
    pub fn get<T: FromCqlValue>(&self, index: usize) -> ScyllaResult<T> {
        self.row
            .columns
            .get(index)
            .ok_or_else(|| {
                ScyllaError::deserialization(format!("Column index {index} out of bounds"))
            })?
            .as_ref()
            .map(|v| T::from_cql(v))
            .transpose()?
            .ok_or_else(|| ScyllaError::deserialization(format!("Column {index} is null")))
    }

    /// Get an optional value by column index.
    pub fn get_opt<T: FromCqlValue>(&self, index: usize) -> ScyllaResult<Option<T>> {
        match self.row.columns.get(index) {
            Some(Some(value)) => Ok(Some(T::from_cql(value)?)),
            Some(None) | None => Ok(None),
        }
    }

    /// Get the number of columns.
    #[must_use]
    pub fn len(&self) -> usize {
        self.row.columns.len()
    }

    /// Check if the row has no columns.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.row.columns.is_empty()
    }
}

/// Trait for types that can be extracted from a CQL value.
pub trait FromCqlValue: Sized {
    /// Extract a value from a CQL value.
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self>;
}

impl FromCqlValue for bool {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::Boolean(v) => Ok(*v),
            _ => Err(ScyllaError::type_conversion("Expected boolean")),
        }
    }
}

impl FromCqlValue for i8 {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::TinyInt(v) => Ok(*v),
            _ => Err(ScyllaError::type_conversion("Expected tinyint")),
        }
    }
}

impl FromCqlValue for i16 {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::SmallInt(v) => Ok(*v),
            _ => Err(ScyllaError::type_conversion("Expected smallint")),
        }
    }
}

impl FromCqlValue for i32 {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::Int(v) => Ok(*v),
            _ => Err(ScyllaError::type_conversion("Expected int")),
        }
    }
}

impl FromCqlValue for i64 {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::BigInt(v) => Ok(*v),
            CqlValue::Counter(v) => Ok(v.0),
            _ => Err(ScyllaError::type_conversion("Expected bigint")),
        }
    }
}

impl FromCqlValue for f32 {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::Float(v) => Ok(*v),
            _ => Err(ScyllaError::type_conversion("Expected float")),
        }
    }
}

impl FromCqlValue for f64 {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::Double(v) => Ok(*v),
            CqlValue::Float(v) => Ok(f64::from(*v)),
            _ => Err(ScyllaError::type_conversion("Expected double")),
        }
    }
}

impl FromCqlValue for String {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::Text(v) | CqlValue::Ascii(v) => Ok(v.clone()),
            _ => Err(ScyllaError::type_conversion("Expected text")),
        }
    }
}

impl FromCqlValue for Vec<u8> {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::Blob(v) => Ok(v.clone()),
            _ => Err(ScyllaError::type_conversion("Expected blob")),
        }
    }
}

impl FromCqlValue for uuid::Uuid {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::Uuid(v) => Ok(*v),
            CqlValue::Timeuuid(v) => Ok((*v).into()),
            _ => Err(ScyllaError::type_conversion("Expected uuid")),
        }
    }
}

impl FromCqlValue for chrono::DateTime<chrono::Utc> {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::Timestamp(ts) => chrono::DateTime::from_timestamp_millis(ts.0)
                .ok_or_else(|| ScyllaError::type_conversion("Invalid timestamp")),
            _ => Err(ScyllaError::type_conversion("Expected timestamp")),
        }
    }
}

impl FromCqlValue for chrono::NaiveDate {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            // CQL `date` is an unsigned day count offset by 2^31 from the Unix
            // epoch; chrono counts days from 0001-01-01 (1970-01-01 = 719_163).
            CqlValue::Date(d) => {
                let days_from_ce = i64::from(d.0) - (1i64 << 31) + 719_163;
                i32::try_from(days_from_ce)
                    .ok()
                    .and_then(chrono::NaiveDate::from_num_days_from_ce_opt)
                    .ok_or_else(|| ScyllaError::type_conversion("Invalid date"))
            }
            _ => Err(ScyllaError::type_conversion("Expected date")),
        }
    }
}

impl FromCqlValue for std::net::IpAddr {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::Inet(v) => Ok(*v),
            _ => Err(ScyllaError::type_conversion("Expected inet")),
        }
    }
}

impl FromCqlValue for ScyllaValue {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        Ok(value.clone().into())
    }
}

impl<T: FromCqlValue> FromCqlValue for Option<T> {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::Empty => Ok(None),
            _ => Ok(Some(T::from_cql(value)?)),
        }
    }
}

impl<T: FromCqlValue> FromCqlValue for Vec<T> {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        match value {
            CqlValue::List(items) | CqlValue::Set(items) => {
                items.iter().map(|v| T::from_cql(v)).collect()
            }
            _ => Err(ScyllaError::type_conversion("Expected list or set")),
        }
    }
}

impl FromCqlValue for serde_json::Value {
    fn from_cql(value: &CqlValue) -> ScyllaResult<Self> {
        let scylla_value: ScyllaValue = value.clone().into();
        Ok(scylla_value.into())
    }
}

/// Implement `FromScyllaRow` for types that implement `DeserializeOwned`.
///
/// This requires converting the row to JSON first, which may not be efficient
/// for all use cases.
///
/// The driver's `Row` carries only bare column values — no column names or
/// other metadata — so values are deserialized **positionally**: tuple
/// structs and sequences map directly, and named-field structs are matched
/// in field-declaration order via serde's sequence support. The selected
/// columns must therefore line up exactly with the target type's field
/// order; prefer explicit column lists over `SELECT *`, and use
/// [`RowAccessor`] or [`impl_from_row!`](crate::impl_from_row) for
/// index-based extraction independent of declaration order.
impl<T: DeserializeOwned> FromScyllaRow for T {
    fn from_row(row: &Row) -> ScyllaResult<Self> {
        // Convert the row to a positional JSON array for serde. Column names
        // are unavailable here (`Row` has no metadata), so a name-keyed JSON
        // object can't be built; serde maps the array onto tuple structs
        // directly and onto named structs in field-declaration order.
        let values: Vec<serde_json::Value> = row
            .columns
            .iter()
            .map(|col| {
                col.as_ref().map_or(serde_json::Value::Null, |v| {
                    let sv: ScyllaValue = v.clone().into();
                    sv.into()
                })
            })
            .collect();

        serde_json::from_value(serde_json::Value::Array(values))
            .map_err(|e| ScyllaError::deserialization(e.to_string()))
    }
}

/// A macro to implement `FromScyllaRow` for a struct with named fields.
///
/// Usage:
/// ```text
/// impl_from_row!(User {
///     id: uuid::Uuid,
///     email: String,
///     name: Option<String>,
///     created_at: chrono::DateTime<chrono::Utc>,
/// });
/// ```
#[macro_export]
macro_rules! impl_from_row {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        impl $crate::row::FromScyllaRow for $name {
            fn from_row(row: &scylla::frame::response::result::Row) -> $crate::error::ScyllaResult<Self> {
                let accessor = $crate::row::RowAccessor::new(row);
                let mut idx = 0;
                Ok(Self {
                    $(
                        $field: {
                            let val = accessor.get::<$ty>(idx)?;
                            idx += 1;
                            val
                        },
                    )*
                })
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_cql_primitives() {
        assert!(bool::from_cql(&CqlValue::Boolean(true)).unwrap());
        assert_eq!(i32::from_cql(&CqlValue::Int(42)).unwrap(), 42);
        assert_eq!(i64::from_cql(&CqlValue::BigInt(100)).unwrap(), 100);
        assert!((f64::from_cql(&CqlValue::Double(3.14)).unwrap() - 3.14).abs() < f64::EPSILON);
        assert_eq!(
            String::from_cql(&CqlValue::Text("hello".into())).unwrap(),
            "hello"
        );
    }

    #[test]
    fn test_from_cql_optional() {
        let result: Option<i32> = Option::<i32>::from_cql(&CqlValue::Int(42)).unwrap();
        assert_eq!(result, Some(42));

        let result: Option<i32> = Option::<i32>::from_cql(&CqlValue::Empty).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_from_cql_list() {
        let list = CqlValue::List(vec![CqlValue::Int(1), CqlValue::Int(2), CqlValue::Int(3)]);
        let result: Vec<i32> = Vec::<i32>::from_cql(&list).unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_from_cql_date_round_trip() {
        use chrono::NaiveDate;
        use scylla::frame::value::CqlDate;

        // CQL encodes `date` as unsigned days since 1970-01-01 offset by 2^31,
        // so the epoch boundary itself is exactly 2^31 on the wire.
        let epoch = NaiveDate::from_cql(&CqlValue::Date(CqlDate(1u32 << 31))).unwrap();
        assert_eq!(epoch, NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

        // Modern date: 2000-01-01 is 10_957 days after the epoch.
        let modern = NaiveDate::from_cql(&CqlValue::Date(CqlDate((1u32 << 31) + 10_957))).unwrap();
        assert_eq!(modern, NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

        // Pre-1970 date: 1900-01-01 is 25_567 days before the epoch.
        let past = NaiveDate::from_cql(&CqlValue::Date(CqlDate((1u32 << 31) - 25_567))).unwrap();
        assert_eq!(past, NaiveDate::from_ymd_opt(1900, 1, 1).unwrap());
    }

    #[test]
    fn test_from_cql_date_out_of_range() {
        use chrono::NaiveDate;
        use scylla::frame::value::CqlDate;

        // Raw value 0 is ~5.9M years BCE, far outside chrono's range.
        assert!(NaiveDate::from_cql(&CqlValue::Date(CqlDate(0))).is_err());
    }

    #[test]
    fn test_from_scylla_row_named_struct() {
        #[derive(Debug, PartialEq, serde::Deserialize)]
        struct UserRow {
            id: i32,
            name: String,
            email: Option<String>,
        }

        // Columns map onto fields positionally, in declaration order.
        let row = Row {
            columns: vec![
                Some(CqlValue::Int(7)),
                Some(CqlValue::Text("alice".into())),
                None,
            ],
        };

        let user = UserRow::from_row(&row).unwrap();
        assert_eq!(
            user,
            UserRow {
                id: 7,
                name: "alice".to_string(),
                email: None,
            }
        );
    }

    #[test]
    fn test_from_scylla_row_tuple_struct() {
        #[derive(Debug, PartialEq, serde::Deserialize)]
        struct Pair(i32, String);

        let row = Row {
            columns: vec![Some(CqlValue::Int(1)), Some(CqlValue::Text("one".into()))],
        };

        assert_eq!(Pair::from_row(&row).unwrap(), Pair(1, "one".to_string()));
    }
}
