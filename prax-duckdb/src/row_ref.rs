//! Bridge between duckdb rows and prax_query::row::RowRef.
//!
//! Snapshots every column value out of a `duckdb::Row` into an owned
//! `duckdb::types::Value` so callers can read fields by name without
//! holding a borrow on the underlying statement.

use std::collections::HashMap;

use duckdb::Row;
use duckdb::types::{TimeUnit, Value, ValueRef};
use prax_query::row::{RowError, RowRef};

pub struct DuckDbRowRef {
    values: HashMap<String, Value>,
}

impl DuckDbRowRef {
    pub fn from_duckdb(row: &Row<'_>, column_names: &[String]) -> Result<Self, RowError> {
        let mut values = HashMap::with_capacity(column_names.len());
        for (i, name) in column_names.iter().enumerate() {
            let v: Value = match row.get_ref(i).map_err(|e| tc(name, e.to_string()))? {
                ValueRef::Null => Value::Null,
                ValueRef::Boolean(b) => Value::Boolean(b),
                ValueRef::TinyInt(i) => Value::TinyInt(i),
                ValueRef::SmallInt(i) => Value::SmallInt(i),
                ValueRef::Int(i) => Value::Int(i),
                ValueRef::BigInt(i) => Value::BigInt(i),
                ValueRef::UTinyInt(i) => Value::UTinyInt(i),
                ValueRef::USmallInt(i) => Value::USmallInt(i),
                ValueRef::UInt(i) => Value::UInt(i),
                ValueRef::UBigInt(i) => Value::UBigInt(i),
                ValueRef::Float(f) => Value::Float(f),
                ValueRef::Double(f) => Value::Double(f),
                ValueRef::Text(bytes) => Value::Text(String::from_utf8_lossy(bytes).into_owned()),
                ValueRef::Blob(bytes) => Value::Blob(bytes.to_vec()),
                other => other.to_owned(),
            };
            values.insert(name.clone(), v);
        }
        Ok(Self { values })
    }
}

fn tc(column: &str, msg: impl Into<String>) -> RowError {
    RowError::TypeConversion {
        column: column.into(),
        message: msg.into(),
    }
}

/// Convert a native `Timestamp(unit, v)` cell — an integer count of `unit`
/// since the Unix epoch — into a UTC datetime. Returns `None` when the count
/// falls outside chrono's representable range.
fn timestamp_to_datetime_utc(unit: TimeUnit, v: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    match unit {
        TimeUnit::Second => chrono::DateTime::from_timestamp(v, 0),
        TimeUnit::Millisecond => chrono::DateTime::from_timestamp_millis(v),
        TimeUnit::Microsecond => chrono::DateTime::from_timestamp_micros(v),
        TimeUnit::Nanosecond => Some(chrono::DateTime::from_timestamp_nanos(v)),
    }
}

/// Convert a native `Date32(days)` cell — days since the Unix epoch — into a
/// naive date. `719_163` is the day number of 1970-01-01 in chrono's
/// days-from-CE counting, matching `types::duckdb_value_to_json`.
/// `checked_add` keeps extreme `days` values from overflowing `i32`.
fn date32_to_naive_date(days: i32) -> Option<chrono::NaiveDate> {
    days.checked_add(719_163)
        .and_then(chrono::NaiveDate::from_num_days_from_ce_opt)
}

/// Convert a native `Time64(unit, v)` cell — an integer count of `unit`
/// since midnight — into a naive wall-clock time.
fn time64_to_naive_time(unit: TimeUnit, v: i64) -> Option<chrono::NaiveTime> {
    let (secs, nanos) = match unit {
        TimeUnit::Second => (v, 0),
        TimeUnit::Millisecond => (v / 1_000, (v % 1_000) * 1_000_000),
        TimeUnit::Microsecond => (v / 1_000_000, (v % 1_000_000) * 1_000),
        TimeUnit::Nanosecond => (v / 1_000_000_000, v % 1_000_000_000),
    };
    chrono::NaiveTime::from_num_seconds_from_midnight_opt(
        u32::try_from(secs).ok()?,
        u32::try_from(nanos).ok()?,
    )
}

/// Read an integer cell, coercing across DuckDB's width-specific variants.
/// Returns `UnexpectedNull` for NULL, `ColumnNotFound` for an absent column.
fn as_i64(v: Option<&Value>, column: &str) -> Result<i64, RowError> {
    match v.ok_or_else(|| RowError::ColumnNotFound(column.into()))? {
        Value::TinyInt(i) => Ok(*i as i64),
        Value::SmallInt(i) => Ok(*i as i64),
        Value::Int(i) => Ok(*i as i64),
        Value::BigInt(i) => Ok(*i),
        Value::UTinyInt(i) => Ok(*i as i64),
        Value::USmallInt(i) => Ok(*i as i64),
        Value::UInt(i) => Ok(*i as i64),
        Value::UBigInt(i) => i64::try_from(*i).map_err(|_| tc(column, "u64 exceeds i64::MAX")),
        Value::Null => Err(RowError::UnexpectedNull(column.into())),
        _ => Err(tc(column, "not an integer")),
    }
}

fn as_i64_opt(v: Option<&Value>, column: &str) -> Result<Option<i64>, RowError> {
    match v {
        None => Err(RowError::ColumnNotFound(column.into())),
        Some(Value::Null) => Ok(None),
        Some(other) => as_i64(Some(other), column).map(Some),
    }
}

impl RowRef for DuckDbRowRef {
    fn get_i32(&self, c: &str) -> Result<i32, RowError> {
        let i = as_i64(self.values.get(c), c)?;
        i32::try_from(i).map_err(|_| tc(c, "i64 overflow"))
    }
    fn get_i32_opt(&self, c: &str) -> Result<Option<i32>, RowError> {
        match as_i64_opt(self.values.get(c), c)? {
            None => Ok(None),
            Some(i) => i32::try_from(i).map(Some).map_err(|_| tc(c, "overflow")),
        }
    }
    fn get_i64(&self, c: &str) -> Result<i64, RowError> {
        as_i64(self.values.get(c), c)
    }
    fn get_i64_opt(&self, c: &str) -> Result<Option<i64>, RowError> {
        as_i64_opt(self.values.get(c), c)
    }
    fn get_f64(&self, c: &str) -> Result<f64, RowError> {
        match self
            .values
            .get(c)
            .ok_or_else(|| RowError::ColumnNotFound(c.into()))?
        {
            Value::Double(f) => Ok(*f),
            Value::Float(f) => Ok(*f as f64),
            Value::TinyInt(i) => Ok(*i as f64),
            Value::SmallInt(i) => Ok(*i as f64),
            Value::Int(i) => Ok(*i as f64),
            Value::BigInt(i) => Ok(*i as f64),
            Value::Null => Err(RowError::UnexpectedNull(c.into())),
            _ => Err(tc(c, "not a number")),
        }
    }
    fn get_f64_opt(&self, c: &str) -> Result<Option<f64>, RowError> {
        match self.values.get(c) {
            None => Err(RowError::ColumnNotFound(c.into())),
            Some(Value::Null) => Ok(None),
            Some(_) => self.get_f64(c).map(Some),
        }
    }
    fn get_bool(&self, c: &str) -> Result<bool, RowError> {
        match self
            .values
            .get(c)
            .ok_or_else(|| RowError::ColumnNotFound(c.into()))?
        {
            Value::Boolean(b) => Ok(*b),
            Value::Null => Err(RowError::UnexpectedNull(c.into())),
            _ => Err(tc(c, "not a boolean")),
        }
    }
    fn get_bool_opt(&self, c: &str) -> Result<Option<bool>, RowError> {
        match self.values.get(c) {
            None => Err(RowError::ColumnNotFound(c.into())),
            Some(Value::Null) => Ok(None),
            Some(Value::Boolean(b)) => Ok(Some(*b)),
            Some(_) => Err(tc(c, "not a boolean")),
        }
    }
    fn get_str(&self, c: &str) -> Result<&str, RowError> {
        match self
            .values
            .get(c)
            .ok_or_else(|| RowError::ColumnNotFound(c.into()))?
        {
            Value::Text(s) => Ok(s.as_str()),
            Value::Null => Err(RowError::UnexpectedNull(c.into())),
            _ => Err(tc(c, "not text")),
        }
    }
    fn get_str_opt(&self, c: &str) -> Result<Option<&str>, RowError> {
        match self.values.get(c) {
            None => Err(RowError::ColumnNotFound(c.into())),
            Some(Value::Null) => Ok(None),
            Some(Value::Text(s)) => Ok(Some(s.as_str())),
            Some(_) => Err(tc(c, "not text")),
        }
    }
    fn get_bytes(&self, c: &str) -> Result<&[u8], RowError> {
        match self
            .values
            .get(c)
            .ok_or_else(|| RowError::ColumnNotFound(c.into()))?
        {
            Value::Blob(b) => Ok(b.as_slice()),
            Value::Text(s) => Ok(s.as_bytes()),
            Value::Null => Err(RowError::UnexpectedNull(c.into())),
            _ => Err(tc(c, "not blob")),
        }
    }
    fn get_bytes_opt(&self, c: &str) -> Result<Option<&[u8]>, RowError> {
        match self.values.get(c) {
            None => Err(RowError::ColumnNotFound(c.into())),
            Some(Value::Null) => Ok(None),
            Some(Value::Blob(b)) => Ok(Some(b.as_slice())),
            Some(Value::Text(s)) => Ok(Some(s.as_bytes())),
            Some(_) => Err(tc(c, "not blob")),
        }
    }
    fn get_datetime_utc(&self, c: &str) -> Result<chrono::DateTime<chrono::Utc>, RowError> {
        match self
            .values
            .get(c)
            .ok_or_else(|| RowError::ColumnNotFound(c.into()))?
        {
            Value::Timestamp(unit, v) => {
                timestamp_to_datetime_utc(*unit, *v).ok_or_else(|| tc(c, "timestamp out of range"))
            }
            Value::Text(s) => chrono::DateTime::parse_from_rfc3339(s)
                .map(|d| d.with_timezone(&chrono::Utc))
                .map_err(|e| tc(c, e.to_string())),
            Value::Null => Err(RowError::UnexpectedNull(c.into())),
            _ => Err(tc(c, "not a timestamp")),
        }
    }
    fn get_datetime_utc_opt(
        &self,
        c: &str,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, RowError> {
        match self.values.get(c) {
            None => Err(RowError::ColumnNotFound(c.into())),
            Some(Value::Null) => Ok(None),
            Some(_) => self.get_datetime_utc(c).map(Some),
        }
    }
    fn get_naive_datetime(&self, c: &str) -> Result<chrono::NaiveDateTime, RowError> {
        match self
            .values
            .get(c)
            .ok_or_else(|| RowError::ColumnNotFound(c.into()))?
        {
            Value::Timestamp(unit, v) => timestamp_to_datetime_utc(*unit, *v)
                .map(|d| d.naive_utc())
                .ok_or_else(|| tc(c, "timestamp out of range")),
            Value::Text(s) => s
                .parse::<chrono::NaiveDateTime>()
                .map_err(|e| tc(c, e.to_string())),
            Value::Null => Err(RowError::UnexpectedNull(c.into())),
            _ => Err(tc(c, "not a timestamp")),
        }
    }
    fn get_naive_datetime_opt(&self, c: &str) -> Result<Option<chrono::NaiveDateTime>, RowError> {
        match self.values.get(c) {
            None => Err(RowError::ColumnNotFound(c.into())),
            Some(Value::Null) => Ok(None),
            Some(_) => self.get_naive_datetime(c).map(Some),
        }
    }
    fn get_naive_date(&self, c: &str) -> Result<chrono::NaiveDate, RowError> {
        match self
            .values
            .get(c)
            .ok_or_else(|| RowError::ColumnNotFound(c.into()))?
        {
            Value::Date32(days) => {
                date32_to_naive_date(*days).ok_or_else(|| tc(c, "date out of range"))
            }
            Value::Text(s) => s
                .parse::<chrono::NaiveDate>()
                .map_err(|e| tc(c, e.to_string())),
            Value::Null => Err(RowError::UnexpectedNull(c.into())),
            _ => Err(tc(c, "not a date")),
        }
    }
    fn get_naive_date_opt(&self, c: &str) -> Result<Option<chrono::NaiveDate>, RowError> {
        match self.values.get(c) {
            None => Err(RowError::ColumnNotFound(c.into())),
            Some(Value::Null) => Ok(None),
            Some(_) => self.get_naive_date(c).map(Some),
        }
    }
    fn get_naive_time(&self, c: &str) -> Result<chrono::NaiveTime, RowError> {
        match self
            .values
            .get(c)
            .ok_or_else(|| RowError::ColumnNotFound(c.into()))?
        {
            Value::Time64(unit, v) => {
                time64_to_naive_time(*unit, *v).ok_or_else(|| tc(c, "time out of range"))
            }
            Value::Text(s) => s
                .parse::<chrono::NaiveTime>()
                .map_err(|e| tc(c, e.to_string())),
            Value::Null => Err(RowError::UnexpectedNull(c.into())),
            _ => Err(tc(c, "not a time")),
        }
    }
    fn get_naive_time_opt(&self, c: &str) -> Result<Option<chrono::NaiveTime>, RowError> {
        match self.values.get(c) {
            None => Err(RowError::ColumnNotFound(c.into())),
            Some(Value::Null) => Ok(None),
            Some(_) => self.get_naive_time(c).map(Some),
        }
    }
    fn get_uuid(&self, c: &str) -> Result<uuid::Uuid, RowError> {
        uuid::Uuid::parse_str(self.get_str(c)?).map_err(|e| tc(c, e.to_string()))
    }
    fn get_uuid_opt(&self, c: &str) -> Result<Option<uuid::Uuid>, RowError> {
        match self.get_str_opt(c)? {
            None => Ok(None),
            Some(s) => uuid::Uuid::parse_str(s)
                .map(Some)
                .map_err(|e| tc(c, e.to_string())),
        }
    }
    fn get_json(&self, c: &str) -> Result<serde_json::Value, RowError> {
        serde_json::from_str(self.get_str(c)?).map_err(|e| tc(c, e.to_string()))
    }
    fn get_json_opt(&self, c: &str) -> Result<Option<serde_json::Value>, RowError> {
        match self.get_str_opt(c)? {
            None => Ok(None),
            Some(s) => serde_json::from_str(s)
                .map(Some)
                .map_err(|e| tc(c, e.to_string())),
        }
    }
    fn get_decimal(&self, c: &str) -> Result<rust_decimal::Decimal, RowError> {
        match self
            .values
            .get(c)
            .ok_or_else(|| RowError::ColumnNotFound(c.into()))?
        {
            Value::Decimal(d) => Ok(*d),
            Value::Text(s) => s
                .parse::<rust_decimal::Decimal>()
                .map_err(|e| tc(c, e.to_string())),
            Value::Null => Err(RowError::UnexpectedNull(c.into())),
            _ => Err(tc(c, "not a decimal")),
        }
    }
    fn get_decimal_opt(&self, c: &str) -> Result<Option<rust_decimal::Decimal>, RowError> {
        match self.values.get(c) {
            None => Err(RowError::ColumnNotFound(c.into())),
            Some(Value::Null) => Ok(None),
            Some(_) => self.get_decimal(c).map(Some),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip native TIMESTAMP and DECIMAL columns through a real
    /// in-memory DuckDB and read them back with the typed getters.
    #[test]
    fn test_native_timestamp_and_decimal_roundtrip() {
        let conn = crate::connection::DuckDbConnection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE events (id INTEGER, ts TIMESTAMP, amount DECIMAL(18, 4));
             INSERT INTO events VALUES (1, TIMESTAMP '2024-03-15 10:30:45.123456', 12345.6789);",
        )
        .unwrap();

        let rows = conn
            .query_rows("SELECT id, ts, amount FROM events", &[])
            .unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];

        assert_eq!(row.get_i32("id").unwrap(), 1);

        let ts = row.get_datetime_utc("ts").unwrap();
        let expected = chrono::DateTime::parse_from_rfc3339("2024-03-15T10:30:45.123456+00:00")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(ts, expected);
        assert_eq!(
            row.get_naive_datetime("ts").unwrap(),
            chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
                .unwrap()
                .and_hms_micro_opt(10, 30, 45, 123456)
                .unwrap()
        );

        let amount = row.get_decimal("amount").unwrap();
        assert_eq!(
            amount,
            "12345.6789".parse::<rust_decimal::Decimal>().unwrap()
        );
    }
}
