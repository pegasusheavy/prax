//! Row types and conversion utilities for SQLx.

use crate::config::DatabaseBackend;
use crate::error::{SqlxError, SqlxResult};
use serde_json::Value as JsonValue;
use sqlx::Row;

/// A generic row wrapper that works across databases.
pub enum SqlxRow {
    /// PostgreSQL row
    #[cfg(feature = "postgres")]
    Postgres(sqlx::postgres::PgRow),
    /// MySQL row
    #[cfg(feature = "mysql")]
    MySql(sqlx::mysql::MySqlRow),
    /// SQLite row
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::sqlite::SqliteRow),
}

impl SqlxRow {
    /// Get the database backend type.
    pub fn backend(&self) -> DatabaseBackend {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(_) => DatabaseBackend::Postgres,
            #[cfg(feature = "mysql")]
            Self::MySql(_) => DatabaseBackend::MySql,
            #[cfg(feature = "sqlite")]
            Self::Sqlite(_) => DatabaseBackend::Sqlite,
        }
    }

    /// Get a column value by index.
    pub fn get<T>(&self, index: usize) -> SqlxResult<T>
    where
        T: SqlxDecode,
    {
        T::decode_from_row(self, index)
    }

    /// Get a column value by name.
    pub fn get_by_name<T>(&self, name: &str) -> SqlxResult<T>
    where
        T: SqlxDecodeNamed,
    {
        T::decode_by_name(self, name)
    }

    /// Try to get a nullable column value by index.
    pub fn try_get<T>(&self, index: usize) -> SqlxResult<Option<T>>
    where
        T: SqlxDecode,
    {
        match T::decode_from_row(self, index) {
            Ok(v) => Ok(Some(v)),
            Err(SqlxError::Sqlx(sqlx::Error::ColumnNotFound(_))) => Ok(None),
            // Decoding a SQL NULL into a non-`Option` type surfaces as a
            // `ColumnDecode` wrapping `UnexpectedNullError`; treat it as "no
            // value" rather than a hard error.
            Err(SqlxError::Sqlx(sqlx::Error::ColumnDecode { ref source, .. }))
                if source.is::<sqlx::error::UnexpectedNullError>() =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Get the number of columns.
    pub fn len(&self) -> usize {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(row) => row.len(),
            #[cfg(feature = "mysql")]
            Self::MySql(row) => row.len(),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(row) => row.len(),
        }
    }

    /// Check if the row is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convert the row to a JSON value.
    pub fn to_json(&self) -> SqlxResult<JsonValue> {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(row) => row_to_json_pg(row),
            #[cfg(feature = "mysql")]
            Self::MySql(row) => row_to_json_mysql(row),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(row) => row_to_json_sqlite(row),
        }
    }
}

/// Trait for decoding values from a row by index.
pub trait SqlxDecode: Sized {
    /// Decode a value from a row at the given index.
    fn decode_from_row(row: &SqlxRow, index: usize) -> SqlxResult<Self>;
}

/// Trait for decoding values from a row by column name.
pub trait SqlxDecodeNamed: Sized {
    /// Decode a value from a row by column name.
    fn decode_by_name(row: &SqlxRow, name: &str) -> SqlxResult<Self>;
}

// Implement SqlxDecode for common types
macro_rules! impl_decode {
    ($ty:ty) => {
        impl SqlxDecode for $ty {
            fn decode_from_row(row: &SqlxRow, index: usize) -> SqlxResult<Self> {
                match row {
                    #[cfg(feature = "postgres")]
                    SqlxRow::Postgres(r) => r.try_get(index).map_err(SqlxError::from),
                    #[cfg(feature = "mysql")]
                    SqlxRow::MySql(r) => r.try_get(index).map_err(SqlxError::from),
                    #[cfg(feature = "sqlite")]
                    SqlxRow::Sqlite(r) => r.try_get(index).map_err(SqlxError::from),
                }
            }
        }

        impl SqlxDecodeNamed for $ty {
            fn decode_by_name(row: &SqlxRow, name: &str) -> SqlxResult<Self> {
                match row {
                    #[cfg(feature = "postgres")]
                    SqlxRow::Postgres(r) => r.try_get(name).map_err(SqlxError::from),
                    #[cfg(feature = "mysql")]
                    SqlxRow::MySql(r) => r.try_get(name).map_err(SqlxError::from),
                    #[cfg(feature = "sqlite")]
                    SqlxRow::Sqlite(r) => r.try_get(name).map_err(SqlxError::from),
                }
            }
        }
    };
}

impl_decode!(i32);
impl_decode!(i64);
impl_decode!(f32);
impl_decode!(f64);
impl_decode!(bool);
impl_decode!(String);
impl_decode!(Vec<u8>);

// Helper functions to convert rows to JSON.
//
// Each cell is probed against a series of candidate types, mirroring the
// probe order used by `row_ref.rs`. sqlx rejects type-incompatible decodes
// before looking at the value, so probes simply fall through until one
// matches. A probe returning `Ok(None)` means the cell is a genuine SQL
// NULL, which is the only case that becomes a JSON null (besides values of
// types we do not recognise).

/// Map an `f64` to a JSON number; NaN and infinities have no JSON
/// counterpart and become null.
fn f64_to_json(f: f64) -> JsonValue {
    serde_json::Number::from_f64(f).map_or(JsonValue::Null, JsonValue::Number)
}

/// Map a decimal to its JSON representation.
#[cfg(any(feature = "postgres", feature = "mysql"))]
fn decimal_to_json(d: rust_decimal::Decimal) -> JsonValue {
    serde_json::to_value(d).unwrap_or(JsonValue::Null)
}

/// Encode raw bytes as a base64 string; binary data has no JSON counterpart.
fn bytes_to_json(bytes: Vec<u8>) -> JsonValue {
    use prax_query::base64::Engine as _;
    JsonValue::String(prax_query::base64::engine::general_purpose::STANDARD.encode(bytes))
}

#[cfg(feature = "postgres")]
fn row_to_json_pg(row: &sqlx::postgres::PgRow) -> SqlxResult<JsonValue> {
    use sqlx::Column;
    let mut obj = serde_json::Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let name = col.name().to_string();
        obj.insert(name, pg_cell_to_json(row, i));
    }
    Ok(JsonValue::Object(obj))
}

#[cfg(feature = "postgres")]
fn pg_cell_to_json(row: &sqlx::postgres::PgRow, i: usize) -> JsonValue {
    if let Ok(Some(s)) = row.try_get::<Option<String>, _>(i) {
        return JsonValue::String(s);
    }
    if let Ok(Some(b)) = row.try_get::<Option<bool>, _>(i) {
        return JsonValue::Bool(b);
    }
    if let Ok(Some(n)) = row.try_get::<Option<i64>, _>(i) {
        return JsonValue::Number(n.into());
    }
    if let Ok(Some(n)) = row.try_get::<Option<i32>, _>(i) {
        return JsonValue::Number(n.into());
    }
    if let Ok(Some(n)) = row.try_get::<Option<i16>, _>(i) {
        return JsonValue::Number(n.into());
    }
    if let Ok(Some(f)) = row.try_get::<Option<f64>, _>(i) {
        return f64_to_json(f);
    }
    if let Ok(Some(f)) = row.try_get::<Option<f32>, _>(i) {
        return f64_to_json(f64::from(f));
    }
    if let Ok(Some(d)) = row.try_get::<Option<rust_decimal::Decimal>, _>(i) {
        return decimal_to_json(d);
    }
    if let Ok(Some(ts)) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(i) {
        return JsonValue::String(ts.to_rfc3339());
    }
    if let Ok(Some(ts)) = row.try_get::<Option<chrono::NaiveDateTime>, _>(i) {
        return JsonValue::String(ts.to_string());
    }
    if let Ok(Some(d)) = row.try_get::<Option<chrono::NaiveDate>, _>(i) {
        return JsonValue::String(d.to_string());
    }
    if let Ok(Some(t)) = row.try_get::<Option<chrono::NaiveTime>, _>(i) {
        return JsonValue::String(t.to_string());
    }
    if let Ok(Some(u)) = row.try_get::<Option<uuid::Uuid>, _>(i) {
        return JsonValue::String(u.to_string());
    }
    if let Ok(Some(j)) = row.try_get::<Option<JsonValue>, _>(i) {
        return j;
    }
    if let Ok(Some(b)) = row.try_get::<Option<Vec<u8>>, _>(i) {
        return bytes_to_json(b);
    }
    JsonValue::Null
}

#[cfg(feature = "mysql")]
fn row_to_json_mysql(row: &sqlx::mysql::MySqlRow) -> SqlxResult<JsonValue> {
    use sqlx::Column;
    let mut obj = serde_json::Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let name = col.name().to_string();
        obj.insert(name, mysql_cell_to_json(row, i));
    }
    Ok(JsonValue::Object(obj))
}

#[cfg(feature = "mysql")]
fn mysql_cell_to_json(row: &sqlx::mysql::MySqlRow, i: usize) -> JsonValue {
    if let Ok(Some(s)) = row.try_get::<Option<String>, _>(i) {
        return JsonValue::String(s);
    }
    if let Ok(Some(b)) = row.try_get::<Option<bool>, _>(i) {
        return JsonValue::Bool(b);
    }
    if let Ok(Some(n)) = row.try_get::<Option<i64>, _>(i) {
        return JsonValue::Number(n.into());
    }
    if let Ok(Some(f)) = row.try_get::<Option<f64>, _>(i) {
        return f64_to_json(f);
    }
    if let Ok(Some(d)) = row.try_get::<Option<rust_decimal::Decimal>, _>(i) {
        return decimal_to_json(d);
    }
    if let Ok(Some(ts)) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(i) {
        return JsonValue::String(ts.to_rfc3339());
    }
    if let Ok(Some(ts)) = row.try_get::<Option<chrono::NaiveDateTime>, _>(i) {
        return JsonValue::String(ts.to_string());
    }
    if let Ok(Some(d)) = row.try_get::<Option<chrono::NaiveDate>, _>(i) {
        return JsonValue::String(d.to_string());
    }
    if let Ok(Some(t)) = row.try_get::<Option<chrono::NaiveTime>, _>(i) {
        return JsonValue::String(t.to_string());
    }
    if let Ok(Some(j)) = row.try_get::<Option<JsonValue>, _>(i) {
        return j;
    }
    if let Ok(Some(b)) = row.try_get::<Option<Vec<u8>>, _>(i) {
        return bytes_to_json(b);
    }
    JsonValue::Null
}

#[cfg(feature = "sqlite")]
fn row_to_json_sqlite(row: &sqlx::sqlite::SqliteRow) -> SqlxResult<JsonValue> {
    use sqlx::Column;
    let mut obj = serde_json::Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let name = col.name().to_string();
        obj.insert(name, sqlite_cell_to_json(row, i));
    }
    Ok(JsonValue::Object(obj))
}

#[cfg(feature = "sqlite")]
fn sqlite_cell_to_json(row: &sqlx::sqlite::SqliteRow, i: usize) -> JsonValue {
    // SQLite is dynamically typed, so the value's storage class drives the
    // decode; there are no separate JSON or datetime storage classes (those
    // come back as TEXT and are caught by the String probe).
    if let Ok(Some(s)) = row.try_get::<Option<String>, _>(i) {
        return JsonValue::String(s);
    }
    if let Ok(Some(n)) = row.try_get::<Option<i64>, _>(i) {
        return JsonValue::Number(n.into());
    }
    if let Ok(Some(f)) = row.try_get::<Option<f64>, _>(i) {
        return f64_to_json(f);
    }
    if let Ok(Some(b)) = row.try_get::<Option<bool>, _>(i) {
        return JsonValue::Bool(b);
    }
    if let Ok(Some(b)) = row.try_get::<Option<Vec<u8>>, _>(i) {
        return bytes_to_json(b);
    }
    JsonValue::Null
}

/// Trait for converting SQL rows to model types.
pub trait FromSqlxRow: Sized {
    /// Convert a row to a model instance.
    fn from_row(row: SqlxRow) -> SqlxResult<Self>;
}
