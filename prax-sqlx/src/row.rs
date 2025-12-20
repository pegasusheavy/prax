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

// Helper functions to convert rows to JSON
#[cfg(feature = "postgres")]
fn row_to_json_pg(row: &sqlx::postgres::PgRow) -> SqlxResult<JsonValue> {
    use sqlx::Column;
    let mut obj = serde_json::Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let name = col.name().to_string();
        let value: Option<JsonValue> = row.try_get(i).ok();
        obj.insert(name, value.unwrap_or(JsonValue::Null));
    }
    Ok(JsonValue::Object(obj))
}

#[cfg(feature = "mysql")]
fn row_to_json_mysql(row: &sqlx::mysql::MySqlRow) -> SqlxResult<JsonValue> {
    use sqlx::Column;
    let mut obj = serde_json::Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let name = col.name().to_string();
        let value: Option<JsonValue> = row.try_get(i).ok();
        obj.insert(name, value.unwrap_or(JsonValue::Null));
    }
    Ok(JsonValue::Object(obj))
}

#[cfg(feature = "sqlite")]
fn row_to_json_sqlite(row: &sqlx::sqlite::SqliteRow) -> SqlxResult<JsonValue> {
    use sqlx::Column;
    let mut obj = serde_json::Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let name = col.name().to_string();
        // SQLite doesn't directly support JSON, so we try common types
        if let Ok(v) = row.try_get::<String, _>(i) {
            obj.insert(name, JsonValue::String(v));
        } else if let Ok(v) = row.try_get::<i64, _>(i) {
            obj.insert(name, JsonValue::Number(v.into()));
        } else if let Ok(v) = row.try_get::<f64, _>(i) {
            if let Some(n) = serde_json::Number::from_f64(v) {
                obj.insert(name, JsonValue::Number(n));
            } else {
                obj.insert(name, JsonValue::Null);
            }
        } else if let Ok(v) = row.try_get::<bool, _>(i) {
            obj.insert(name, JsonValue::Bool(v));
        } else {
            obj.insert(name, JsonValue::Null);
        }
    }
    Ok(JsonValue::Object(obj))
}

/// Trait for converting SQL rows to model types.
pub trait FromSqlxRow: Sized {
    /// Convert a row to a model instance.
    fn from_row(row: SqlxRow) -> SqlxResult<Self>;
}
