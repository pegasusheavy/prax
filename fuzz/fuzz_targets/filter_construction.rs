//! Fuzz target for filter construction.
//!
//! This target generates arbitrary filter trees to find crashes
//! and panics in filter operations.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_filter_construction
//! ```

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use prax_query::filter::{Filter, FilterValue, ScalarFilter};

/// A fuzzable filter value.
#[derive(Debug, Arbitrary, Clone)]
enum FuzzFilterValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<FuzzFilterValue>),
}

impl From<FuzzFilterValue> for FilterValue {
    fn from(val: FuzzFilterValue) -> Self {
        match val {
            FuzzFilterValue::Null => FilterValue::Null,
            FuzzFilterValue::Bool(b) => FilterValue::Bool(b),
            FuzzFilterValue::Int(i) => FilterValue::Int(i),
            FuzzFilterValue::Float(f) => FilterValue::Float(f),
            FuzzFilterValue::String(s) => FilterValue::String(s),
            FuzzFilterValue::List(list) => {
                FilterValue::List(list.into_iter().map(FilterValue::from).collect())
            }
        }
    }
}

/// A fuzzable filter.
#[derive(Debug, Arbitrary)]
enum FuzzFilter {
    None,
    Equals(String, FuzzFilterValue),
    NotEquals(String, FuzzFilterValue),
    Lt(String, FuzzFilterValue),
    Lte(String, FuzzFilterValue),
    Gt(String, FuzzFilterValue),
    Gte(String, FuzzFilterValue),
    In(String, Vec<FuzzFilterValue>),
    NotIn(String, Vec<FuzzFilterValue>),
    Contains(String, FuzzFilterValue),
    StartsWith(String, FuzzFilterValue),
    EndsWith(String, FuzzFilterValue),
    IsNull(String),
    IsNotNull(String),
    And(Vec<FuzzFilter>),
    Or(Vec<FuzzFilter>),
    Not(Box<FuzzFilter>),
}

impl FuzzFilter {
    fn to_filter(self, depth: usize) -> Filter {
        // Limit recursion depth to prevent stack overflow
        if depth > 10 {
            return Filter::None;
        }

        match self {
            FuzzFilter::None => Filter::None,
            FuzzFilter::Equals(field, val) => {
                Filter::Equals(field, val.into())
            }
            FuzzFilter::NotEquals(field, val) => {
                Filter::NotEquals(field, val.into())
            }
            FuzzFilter::Lt(field, val) => {
                Filter::Lt(field, val.into())
            }
            FuzzFilter::Lte(field, val) => {
                Filter::Lte(field, val.into())
            }
            FuzzFilter::Gt(field, val) => {
                Filter::Gt(field, val.into())
            }
            FuzzFilter::Gte(field, val) => {
                Filter::Gte(field, val.into())
            }
            FuzzFilter::In(field, vals) => {
                Filter::In(field, vals.into_iter().map(FilterValue::from).collect())
            }
            FuzzFilter::NotIn(field, vals) => {
                Filter::NotIn(field, vals.into_iter().map(FilterValue::from).collect())
            }
            FuzzFilter::Contains(field, val) => {
                Filter::Contains(field, val.into())
            }
            FuzzFilter::StartsWith(field, val) => {
                Filter::StartsWith(field, val.into())
            }
            FuzzFilter::EndsWith(field, val) => {
                Filter::EndsWith(field, val.into())
            }
            FuzzFilter::IsNull(field) => {
                Filter::IsNull(field)
            }
            FuzzFilter::IsNotNull(field) => {
                Filter::IsNotNull(field)
            }
            FuzzFilter::And(filters) => {
                Filter::And(
                    filters
                        .into_iter()
                        .take(10) // Limit children
                        .map(|f| f.to_filter(depth + 1))
                        .collect()
                )
            }
            FuzzFilter::Or(filters) => {
                Filter::Or(
                    filters
                        .into_iter()
                        .take(10) // Limit children
                        .map(|f| f.to_filter(depth + 1))
                        .collect()
                )
            }
            FuzzFilter::Not(filter) => {
                Filter::Not(Box::new(filter.to_filter(depth + 1)))
            }
        }
    }
}

/// A fuzzable scalar filter.
#[derive(Debug, Arbitrary)]
enum FuzzScalarFilter {
    Equals(i64),
    Not(i64),
    In(Vec<i64>),
    NotIn(Vec<i64>),
    Lt(i64),
    Lte(i64),
    Gt(i64),
    Gte(i64),
    Contains(String),
    StartsWith(String),
    EndsWith(String),
}

impl FuzzScalarFilter {
    fn to_scalar_filter_int(self) -> ScalarFilter<i64> {
        match self {
            FuzzScalarFilter::Equals(v) => ScalarFilter::Equals(v),
            FuzzScalarFilter::Not(v) => ScalarFilter::Not(Box::new(v)),
            FuzzScalarFilter::In(vals) => ScalarFilter::In(vals),
            FuzzScalarFilter::NotIn(vals) => ScalarFilter::NotIn(vals),
            FuzzScalarFilter::Lt(v) => ScalarFilter::Lt(v),
            FuzzScalarFilter::Lte(v) => ScalarFilter::Lte(v),
            FuzzScalarFilter::Gt(v) => ScalarFilter::Gt(v),
            FuzzScalarFilter::Gte(v) => ScalarFilter::Gte(v),
            _ => ScalarFilter::Equals(0), // Default for string ops on int
        }
    }

    fn to_scalar_filter_string(self) -> ScalarFilter<String> {
        match self {
            FuzzScalarFilter::Contains(s) => ScalarFilter::Contains(s),
            FuzzScalarFilter::StartsWith(s) => ScalarFilter::StartsWith(s),
            FuzzScalarFilter::EndsWith(s) => ScalarFilter::EndsWith(s),
            _ => ScalarFilter::Equals(String::new()),
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);

    // Test filter construction
    if let Ok(fuzz_filter) = FuzzFilter::arbitrary(&mut unstructured) {
        let filter = fuzz_filter.to_filter(0);

        // These operations should never panic
        let _ = filter.clone();
        let _ = format!("{:?}", filter);
    }

    // Test scalar filter construction
    if let Ok(fuzz_scalar) = FuzzScalarFilter::arbitrary(&mut unstructured) {
        let int_filter = fuzz_scalar.to_scalar_filter_int();
        let _ = format!("{:?}", int_filter);
    }

    if let Ok(fuzz_scalar) = FuzzScalarFilter::arbitrary(&mut unstructured) {
        let str_filter = fuzz_scalar.to_scalar_filter_string();
        let _ = format!("{:?}", str_filter);
    }
});

