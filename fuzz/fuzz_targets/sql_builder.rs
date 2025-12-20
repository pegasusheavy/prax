//! Fuzz target for the SQL builder utilities.
//!
//! This target tests SQL generation utilities for robustness.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_sql_builder
//! ```

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use prax_query::sql::{escape_identifier, needs_quoting, quote_identifier, DatabaseType, SqlBuilder};
use prax_query::filter::FilterValue;

/// Test identifier handling with arbitrary strings.
#[derive(Debug, Arbitrary)]
struct FuzzIdentifier {
    name: String,
}

/// Test SQL builder with arbitrary operations.
#[derive(Debug, Arbitrary)]
struct FuzzSqlBuilderSession {
    db_type: FuzzDatabaseType,
    operations: Vec<FuzzSqlBuilderOp>,
}

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

#[derive(Debug, Arbitrary)]
enum FuzzSqlBuilderOp {
    Push(String),
    PushParam(FuzzFilterValue),
    PushIdentifier(String),
    PushSep(String),
}

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

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);

    // Test identifier escaping with arbitrary strings
    if let Ok(fuzz_id) = FuzzIdentifier::arbitrary(&mut unstructured) {
        // These functions should never panic
        let _ = escape_identifier(&fuzz_id.name);
        let _ = needs_quoting(&fuzz_id.name);
        let _ = quote_identifier(&fuzz_id.name);
    }

    // Test database placeholder generation
    if let Ok(db_type) = FuzzDatabaseType::arbitrary(&mut unstructured) {
        let db: DatabaseType = db_type.into();
        for i in 0..100 {
            let _ = db.placeholder(i);
        }
    }

    // Test SQL builder with arbitrary operations
    if let Ok(session) = FuzzSqlBuilderSession::arbitrary(&mut unstructured) {
        let mut builder = SqlBuilder::new(session.db_type.into());

        for op in session.operations.into_iter().take(100) {
            match op {
                FuzzSqlBuilderOp::Push(s) => {
                    builder.push(&s);
                }
                FuzzSqlBuilderOp::PushParam(v) => {
                    builder.push_param(FilterValue::from(v));
                }
                FuzzSqlBuilderOp::PushIdentifier(s) => {
                    builder.push_identifier(&s);
                }
                FuzzSqlBuilderOp::PushSep(s) => {
                    builder.push_sep(&s);
                }
            }
        }

        // Build should never panic
        let (sql, params) = builder.build();
        let _ = sql.len();
        let _ = params.len();

        // Also test the accessors
        let mut builder2 = SqlBuilder::new(DatabaseType::PostgreSQL);
        builder2.push("SELECT ");
        let _ = builder2.sql();
        let _ = builder2.params();
        let _ = builder2.next_param_index();
    }
});

