//! Fuzz target for the Prax query builder.
//!
//! This target generates arbitrary SQL queries using the Sql builder
//! to find crashes and panics.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_query_builder
//! ```

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use prax_query::filter::FilterValue;
use prax_query::raw::Sql;
use prax_query::sql::DatabaseType;

/// Operations that can be performed on the SQL builder.
#[derive(Debug, Arbitrary)]
enum SqlOperation {
    Push(String),
    Bind(FuzzFilterValue),
    PushBind(String, FuzzFilterValue),
    PushIf(bool, String),
    BindIf(bool, FuzzFilterValue),
}

/// A fuzzable filter value.
#[derive(Debug, Arbitrary)]
enum FuzzFilterValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl From<FuzzFilterValue> for FilterValue {
    fn from(val: FuzzFilterValue) -> Self {
        match val {
            FuzzFilterValue::Null => FilterValue::Null,
            FuzzFilterValue::Bool(b) => FilterValue::Bool(b),
            FuzzFilterValue::Int(i) => FilterValue::Int(i),
            FuzzFilterValue::Float(f) => FilterValue::Float(f),
            FuzzFilterValue::String(s) => FilterValue::String(s),
        }
    }
}

/// A database type for fuzzing.
#[derive(Debug, Arbitrary)]
enum FuzzDatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
}

impl From<FuzzDatabaseType> for DatabaseType {
    fn from(val: FuzzDatabaseType) -> Self {
        match val {
            FuzzDatabaseType::PostgreSQL => DatabaseType::PostgreSQL,
            FuzzDatabaseType::MySQL => DatabaseType::MySQL,
            FuzzDatabaseType::SQLite => DatabaseType::SQLite,
        }
    }
}

/// A fuzzing session for the SQL builder.
#[derive(Debug, Arbitrary)]
struct FuzzSqlBuilder {
    db_type: FuzzDatabaseType,
    initial_sql: String,
    operations: Vec<SqlOperation>,
}

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);

    if let Ok(fuzz_builder) = FuzzSqlBuilder::arbitrary(&mut unstructured) {
        // Create the SQL builder
        let mut sql = Sql::new(&fuzz_builder.initial_sql)
            .with_db_type(fuzz_builder.db_type.into());

        // Apply operations
        for op in fuzz_builder.operations {
            sql = match op {
                SqlOperation::Push(s) => sql.push(s),
                SqlOperation::Bind(v) => sql.bind(FilterValue::from(v)),
                SqlOperation::PushBind(s, v) => sql.push(s).bind(FilterValue::from(v)),
                SqlOperation::PushIf(cond, s) => sql.push_if(cond, s),
                SqlOperation::BindIf(cond, v) => sql.bind_if(cond, FilterValue::from(v)),
            };
        }

        // Build should never panic
        let (query, params) = sql.build();

        // Verify the output is valid
        let _ = query.len();
        let _ = params.len();
    }
});

