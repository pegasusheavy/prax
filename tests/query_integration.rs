//! Integration tests for the query builder module.
//!
//! These tests verify the query building functionality including:
//! - Filter construction
//! - Data building
//! - Connection string parsing
//! - Error handling

use prax_query::filter::{Filter, FilterValue, ScalarFilter};
use prax_query::data::{DataBuilder, FieldValue};
use prax_query::connection::{ConnectionString, Driver};
use prax_query::error::{QueryError, ErrorCode};
use prax_query::raw::Sql;

/// Test scalar filter construction with various value types
#[test]
fn test_scalar_filter_equals_int() {
    let filter = ScalarFilter::Equals(FilterValue::Int(42));
    assert!(matches!(filter, ScalarFilter::Equals(FilterValue::Int(42))));
}

#[test]
fn test_scalar_filter_equals_string() {
    let filter = ScalarFilter::Equals(FilterValue::String("test".into()));
    assert!(matches!(filter, ScalarFilter::Equals(FilterValue::String(_))));
}

#[test]
fn test_scalar_filter_equals_bool() {
    let filter = ScalarFilter::Equals(FilterValue::Bool(true));
    assert!(matches!(filter, ScalarFilter::Equals(FilterValue::Bool(true))));
}

#[test]
fn test_scalar_filter_string_contains() {
    let filter: ScalarFilter<FilterValue> = ScalarFilter::Contains(FilterValue::String("test".into()));
    assert!(matches!(filter, ScalarFilter::Contains(_)));
}

#[test]
fn test_scalar_filter_string_starts_with() {
    let filter: ScalarFilter<FilterValue> = ScalarFilter::StartsWith(FilterValue::String("prefix".into()));
    assert!(matches!(filter, ScalarFilter::StartsWith(_)));
}

#[test]
fn test_scalar_filter_string_ends_with() {
    let filter: ScalarFilter<FilterValue> = ScalarFilter::EndsWith(FilterValue::String("suffix".into()));
    assert!(matches!(filter, ScalarFilter::EndsWith(_)));
}

#[test]
fn test_scalar_filter_comparison_gt() {
    let filter: ScalarFilter<FilterValue> = ScalarFilter::Gt(FilterValue::Int(10));
    assert!(matches!(filter, ScalarFilter::Gt(_)));
}

#[test]
fn test_scalar_filter_comparison_gte() {
    let filter: ScalarFilter<FilterValue> = ScalarFilter::Gte(FilterValue::Int(10));
    assert!(matches!(filter, ScalarFilter::Gte(_)));
}

#[test]
fn test_scalar_filter_comparison_lt() {
    let filter: ScalarFilter<FilterValue> = ScalarFilter::Lt(FilterValue::Int(100));
    assert!(matches!(filter, ScalarFilter::Lt(_)));
}

#[test]
fn test_scalar_filter_comparison_lte() {
    let filter: ScalarFilter<FilterValue> = ScalarFilter::Lte(FilterValue::Int(100));
    assert!(matches!(filter, ScalarFilter::Lte(_)));
}

#[test]
fn test_scalar_filter_in_list() {
    let values = vec![
        FilterValue::String("a".into()),
        FilterValue::String("b".into()),
        FilterValue::String("c".into()),
    ];
    let filter: ScalarFilter<FilterValue> = ScalarFilter::In(values);
    assert!(matches!(filter, ScalarFilter::In(_)));
}

#[test]
fn test_scalar_filter_not_in_list() {
    let values = vec![FilterValue::Int(1), FilterValue::Int(2)];
    let filter: ScalarFilter<FilterValue> = ScalarFilter::NotIn(values);
    assert!(matches!(filter, ScalarFilter::NotIn(_)));
}

#[test]
fn test_scalar_filter_is_null() {
    let filter: ScalarFilter<FilterValue> = ScalarFilter::IsNull;
    assert!(matches!(filter, ScalarFilter::IsNull));
}

#[test]
fn test_scalar_filter_is_not_null() {
    let filter: ScalarFilter<FilterValue> = ScalarFilter::IsNotNull;
    assert!(matches!(filter, ScalarFilter::IsNotNull));
}

/// Test Filter enum constructors
#[test]
fn test_filter_equals() {
    let filter = Filter::Equals("email".into(), FilterValue::String("test@example.com".into()));
    assert!(matches!(filter, Filter::Equals(_, _)));
}

#[test]
fn test_filter_not_equals() {
    let filter = Filter::NotEquals("status".into(), FilterValue::String("deleted".into()));
    assert!(matches!(filter, Filter::NotEquals(_, _)));
}

#[test]
fn test_filter_contains() {
    let filter = Filter::Contains("email".into(), FilterValue::String("@example.com".into()));
    assert!(matches!(filter, Filter::Contains(_, _)));
}

#[test]
fn test_filter_and_combination() {
    let filter1 = Filter::Contains("email".into(), FilterValue::String("@example.com".into()));
    let filter2 = Filter::Equals("active".into(), FilterValue::Bool(true));

    let combined = Filter::and([filter1, filter2]);
    assert!(matches!(combined, Filter::And(_)));
}

#[test]
fn test_filter_or_combination() {
    let filter1 = Filter::Equals("role".into(), FilterValue::String("admin".into()));
    let filter2 = Filter::Equals("role".into(), FilterValue::String("moderator".into()));

    let combined = Filter::or([filter1, filter2]);
    assert!(matches!(combined, Filter::Or(_)));
}

#[test]
fn test_filter_not() {
    let inner = Filter::Equals("deleted".into(), FilterValue::Bool(true));
    let negated = Filter::Not(Box::new(inner));
    assert!(matches!(negated, Filter::Not(_)));
}

/// Test raw SQL
#[test]
fn test_raw_sql_construction() {
    let sql = Sql::new("SELECT * FROM users WHERE id = $1");
    assert_eq!(sql.sql(), "SELECT * FROM users WHERE id = $1");
}

#[test]
fn test_raw_sql_with_params() {
    let sql = Sql::new("SELECT * FROM users WHERE id = $1 AND active = $2")
        .bind(42i64)
        .bind(true);

    assert_eq!(sql.params().len(), 2);
}

/// Test data builder
#[test]
fn test_data_builder_set_string() {
    let builder = DataBuilder::new()
        .set("email", FieldValue::String("test@example.com".into()));

    let fields = builder.into_fields();
    assert!(fields.contains_key("email"));
}

#[test]
fn test_data_builder_set_multiple_fields() {
    let builder = DataBuilder::new()
        .set("email", FieldValue::String("test@example.com".into()))
        .set("name", FieldValue::String("Test User".into()))
        .set("age", FieldValue::Int(25));

    let fields = builder.into_fields();
    assert_eq!(fields.len(), 3);
}

#[test]
fn test_data_builder_set_null() {
    let builder = DataBuilder::new()
        .set("bio", FieldValue::Null);

    let fields = builder.into_fields();
    assert!(matches!(fields.get("bio"), Some(FieldValue::Null)));
}

/// Test connection string parsing
#[test]
fn test_connection_string_postgresql() {
    let conn = ConnectionString::parse("postgresql://user:pass@localhost:5432/mydb")
        .expect("Failed to parse connection string");

    assert!(matches!(conn.driver(), Driver::Postgres));
    assert_eq!(conn.host(), Some("localhost"));
    assert_eq!(conn.port(), Some(5432));
    assert_eq!(conn.database(), Some("mydb"));
    assert_eq!(conn.user(), Some("user"));
}

#[test]
fn test_connection_string_postgres_short() {
    let conn = ConnectionString::parse("postgres://user:pass@localhost:5432/mydb")
        .expect("Failed to parse connection string");

    assert!(matches!(conn.driver(), Driver::Postgres));
}

#[test]
fn test_connection_string_mysql() {
    let conn = ConnectionString::parse("mysql://user:pass@localhost:3306/mydb")
        .expect("Failed to parse connection string");

    assert!(matches!(conn.driver(), Driver::MySql));
}

#[test]
fn test_connection_string_sqlite_file() {
    let conn = ConnectionString::parse("sqlite:///path/to/db.sqlite")
        .expect("Failed to parse connection string");

    assert!(matches!(conn.driver(), Driver::Sqlite));
}

#[test]
fn test_connection_string_sqlite_memory() {
    let conn = ConnectionString::parse("sqlite::memory:")
        .expect("Failed to parse connection string");

    assert!(matches!(conn.driver(), Driver::Sqlite));
}

#[test]
fn test_connection_string_with_options() {
    let conn = ConnectionString::parse(
        "postgresql://user:pass@localhost:5432/mydb?sslmode=require&connect_timeout=10"
    ).expect("Failed to parse connection string");

    // Just verify parsing succeeds with query params
    assert!(matches!(conn.driver(), Driver::Postgres));
}

#[test]
fn test_connection_string_invalid() {
    let result = ConnectionString::parse("invalid://not/a/valid/url");
    assert!(result.is_err());
}

/// Test error types
#[test]
fn test_error_code_not_found() {
    let err = QueryError::not_found("User");
    assert_eq!(err.code, ErrorCode::RecordNotFound);
}

#[test]
fn test_error_with_context() {
    let err = QueryError::not_found("User")
        .with_context("Finding user by email");

    // Error should have context set
    assert!(err.message.contains("User"));
}

#[test]
fn test_error_display() {
    let err = QueryError::not_found("User");
    let display = format!("{}", err);
    assert!(!display.is_empty());
}

/// Test filter value conversions
#[test]
fn test_filter_value_from_i32() {
    let value: FilterValue = 42i32.into();
    assert!(matches!(value, FilterValue::Int(42)));
}

#[test]
fn test_filter_value_from_i64() {
    let value: FilterValue = 42i64.into();
    assert!(matches!(value, FilterValue::Int(42)));
}

#[test]
fn test_filter_value_from_f64() {
    let value: FilterValue = 3.14f64.into();
    assert!(matches!(value, FilterValue::Float(_)));
}

#[test]
fn test_filter_value_from_bool() {
    let value: FilterValue = true.into();
    assert!(matches!(value, FilterValue::Bool(true)));
}

#[test]
fn test_filter_value_from_str() {
    let value: FilterValue = "hello".into();
    assert!(matches!(value, FilterValue::String(_)));
}

#[test]
fn test_filter_value_from_string() {
    let value: FilterValue = String::from("world").into();
    assert!(matches!(value, FilterValue::String(_)));
}

#[test]
fn test_filter_value_from_option_some() {
    let value: FilterValue = Some(42i32).into();
    assert!(matches!(value, FilterValue::Int(42)));
}

#[test]
fn test_filter_value_from_option_none() {
    let value: FilterValue = None::<i32>.into();
    assert!(matches!(value, FilterValue::Null));
}

#[test]
fn test_filter_value_is_null() {
    let null_value = FilterValue::Null;
    assert!(null_value.is_null());

    let non_null = FilterValue::Int(42);
    assert!(!non_null.is_null());
}

/// Test nested filters
#[test]
fn test_deeply_nested_filters() {
    let inner_and = Filter::and([
        Filter::Equals("status".into(), FilterValue::String("active".into())),
        Filter::Equals("verified".into(), FilterValue::Bool(true)),
    ]);

    let inner_or = Filter::or([
        Filter::Equals("role".into(), FilterValue::String("admin".into())),
        Filter::Equals("role".into(), FilterValue::String("super_admin".into())),
    ]);

    let combined = Filter::and([inner_and, inner_or]);
    assert!(matches!(combined, Filter::And(_)));
}

/// Test field value types
#[test]
fn test_field_value_types() {
    let null_val = FieldValue::Null;
    let bool_val = FieldValue::Bool(true);
    let int_val = FieldValue::Int(42);
    let float_val = FieldValue::Float(3.14);
    let string_val = FieldValue::String("hello".to_string());

    assert!(matches!(null_val, FieldValue::Null));
    assert!(matches!(bool_val, FieldValue::Bool(true)));
    assert!(matches!(int_val, FieldValue::Int(42)));
    assert!(matches!(float_val, FieldValue::Float(_)));
    assert!(matches!(string_val, FieldValue::String(_)));
}

/// Test connection URL parsing edge cases
#[test]
fn test_connection_string_no_port() {
    let conn = ConnectionString::parse("postgresql://user:pass@localhost/mydb")
        .expect("Failed to parse connection string");

    assert_eq!(conn.host(), Some("localhost"));
    // Port should be None or default
}
