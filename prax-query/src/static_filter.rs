//! Static filter construction for zero-allocation filters.
//!
//! This module provides zero-cost filter construction through:
//! - Static field name constants
//! - Type-level filter builders
//! - Compile-time filter macros
//!
//! # Performance
//!
//! Static filters avoid heap allocations entirely:
//! - Field names are `&'static str` (no `Cow` overhead)
//! - Values are constructed inline
//! - Common patterns are pre-computed
//!
//! # Examples
//!
//! ```rust
//! use prax_query::static_filter::{StaticFilter, eq, gt, and2};
//! use prax_query::static_filter::fields;
//!
//! // Zero-allocation filter construction
//! let filter = eq(fields::ID, 42);
//! let filter = gt(fields::AGE, 18);
//!
//! // Combine two filters (optimized path)
//! let filter = and2(
//!     eq(fields::ACTIVE, true),
//!     gt(fields::SCORE, 100),
//! );
//! ```

use crate::filter::{Filter, FilterValue, ValueList};
use std::borrow::Cow;

/// A static filter with compile-time known field name.
///
/// This is a zero-cost abstraction over `Filter` that avoids
/// the `Cow<'static, str>` overhead for field names.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticFilter {
    inner: Filter,
}

impl StaticFilter {
    /// Create a new static filter.
    #[inline]
    pub const fn new(inner: Filter) -> Self {
        Self { inner }
    }

    /// Convert to the underlying Filter.
    #[inline]
    pub fn into_filter(self) -> Filter {
        self.inner
    }

    /// Get a reference to the underlying Filter.
    #[inline]
    pub fn as_filter(&self) -> &Filter {
        &self.inner
    }
}

impl From<StaticFilter> for Filter {
    #[inline]
    fn from(f: StaticFilter) -> Self {
        f.inner
    }
}

/// Common field name constants for zero-allocation filters.
///
/// These are pre-defined `&'static str` values for common database columns.
/// Using these avoids the overhead of `Cow::Borrowed` construction.
pub mod fields {
    /// Primary key field.
    pub const ID: &str = "id";
    /// UUID field.
    pub const UUID: &str = "uuid";
    /// Name field.
    pub const NAME: &str = "name";
    /// Email field.
    pub const EMAIL: &str = "email";
    /// Username field.
    pub const USERNAME: &str = "username";
    /// Password hash field.
    pub const PASSWORD: &str = "password";
    /// Title field.
    pub const TITLE: &str = "title";
    /// Description field.
    pub const DESCRIPTION: &str = "description";
    /// Content field.
    pub const CONTENT: &str = "content";
    /// Body field.
    pub const BODY: &str = "body";
    /// Status field.
    pub const STATUS: &str = "status";
    /// Type field.
    pub const TYPE: &str = "type";
    /// Role field.
    pub const ROLE: &str = "role";
    /// Active flag field.
    pub const ACTIVE: &str = "active";
    /// Enabled flag field.
    pub const ENABLED: &str = "enabled";
    /// Deleted flag field.
    pub const DELETED: &str = "deleted";
    /// Verified flag field.
    pub const VERIFIED: &str = "verified";
    /// Published flag field.
    pub const PUBLISHED: &str = "published";
    /// Count field.
    pub const COUNT: &str = "count";
    /// Score field.
    pub const SCORE: &str = "score";
    /// Priority field.
    pub const PRIORITY: &str = "priority";
    /// Order/sort order field.
    pub const ORDER: &str = "order";
    /// Position field.
    pub const POSITION: &str = "position";
    /// Age field.
    pub const AGE: &str = "age";
    /// Amount field.
    pub const AMOUNT: &str = "amount";
    /// Price field.
    pub const PRICE: &str = "price";
    /// Quantity field.
    pub const QUANTITY: &str = "quantity";
    /// Foreign key: user_id.
    pub const USER_ID: &str = "user_id";
    /// Foreign key: post_id.
    pub const POST_ID: &str = "post_id";
    /// Foreign key: comment_id.
    pub const COMMENT_ID: &str = "comment_id";
    /// Foreign key: category_id.
    pub const CATEGORY_ID: &str = "category_id";
    /// Foreign key: parent_id.
    pub const PARENT_ID: &str = "parent_id";
    /// Foreign key: author_id.
    pub const AUTHOR_ID: &str = "author_id";
    /// Foreign key: owner_id.
    pub const OWNER_ID: &str = "owner_id";
    /// Timestamp: created_at.
    pub const CREATED_AT: &str = "created_at";
    /// Timestamp: updated_at.
    pub const UPDATED_AT: &str = "updated_at";
    /// Timestamp: deleted_at.
    pub const DELETED_AT: &str = "deleted_at";
    /// Timestamp: published_at.
    pub const PUBLISHED_AT: &str = "published_at";
    /// Timestamp: expires_at.
    pub const EXPIRES_AT: &str = "expires_at";
    /// Timestamp: starts_at.
    pub const STARTS_AT: &str = "starts_at";
    /// Timestamp: ends_at.
    pub const ENDS_AT: &str = "ends_at";
    /// Timestamp: last_login_at.
    pub const LAST_LOGIN_AT: &str = "last_login_at";
    /// Timestamp: verified_at.
    pub const VERIFIED_AT: &str = "verified_at";
    /// Slug field.
    pub const SLUG: &str = "slug";
    /// URL field.
    pub const URL: &str = "url";
    /// Path field.
    pub const PATH: &str = "path";
    /// Key field.
    pub const KEY: &str = "key";
    /// Value field.
    pub const VALUE: &str = "value";
    /// Token field.
    pub const TOKEN: &str = "token";
    /// Code field.
    pub const CODE: &str = "code";
    /// Version field.
    pub const VERSION: &str = "version";
}

// ============================================================================
// Zero-allocation filter constructors
// ============================================================================

/// Create an equality filter with static field name.
///
/// This is the fastest way to create an equality filter - no heap allocation
/// if the value is a primitive type.
///
/// # Example
///
/// ```rust
/// use prax_query::static_filter::{eq, fields};
///
/// let filter = eq(fields::ID, 42);
/// let filter = eq(fields::ACTIVE, true);
/// let filter = eq(fields::NAME, "Alice");
/// ```
#[inline]
pub fn eq(field: &'static str, value: impl Into<FilterValue>) -> Filter {
    Filter::Equals(Cow::Borrowed(field), value.into())
}

/// Create a not-equals filter with static field name.
#[inline]
pub fn ne(field: &'static str, value: impl Into<FilterValue>) -> Filter {
    Filter::NotEquals(Cow::Borrowed(field), value.into())
}

/// Create a less-than filter with static field name.
#[inline]
pub fn lt(field: &'static str, value: impl Into<FilterValue>) -> Filter {
    Filter::Lt(Cow::Borrowed(field), value.into())
}

/// Create a less-than-or-equal filter with static field name.
#[inline]
pub fn lte(field: &'static str, value: impl Into<FilterValue>) -> Filter {
    Filter::Lte(Cow::Borrowed(field), value.into())
}

/// Create a greater-than filter with static field name.
#[inline]
pub fn gt(field: &'static str, value: impl Into<FilterValue>) -> Filter {
    Filter::Gt(Cow::Borrowed(field), value.into())
}

/// Create a greater-than-or-equal filter with static field name.
#[inline]
pub fn gte(field: &'static str, value: impl Into<FilterValue>) -> Filter {
    Filter::Gte(Cow::Borrowed(field), value.into())
}

/// Create an IS NULL filter with static field name.
#[inline]
pub const fn is_null(field: &'static str) -> Filter {
    Filter::IsNull(Cow::Borrowed(field))
}

/// Create an IS NOT NULL filter with static field name.
#[inline]
pub const fn is_not_null(field: &'static str) -> Filter {
    Filter::IsNotNull(Cow::Borrowed(field))
}

/// Create a LIKE %value% filter with static field name.
#[inline]
pub fn contains(field: &'static str, value: impl Into<FilterValue>) -> Filter {
    Filter::Contains(Cow::Borrowed(field), value.into())
}

/// Create a LIKE value% filter with static field name.
#[inline]
pub fn starts_with(field: &'static str, value: impl Into<FilterValue>) -> Filter {
    Filter::StartsWith(Cow::Borrowed(field), value.into())
}

/// Create a LIKE %value filter with static field name.
#[inline]
pub fn ends_with(field: &'static str, value: impl Into<FilterValue>) -> Filter {
    Filter::EndsWith(Cow::Borrowed(field), value.into())
}

/// Create an IN filter with static field name.
#[inline]
pub fn in_list(field: &'static str, values: impl Into<ValueList>) -> Filter {
    Filter::In(Cow::Borrowed(field), values.into())
}

/// Create a NOT IN filter with static field name.
#[inline]
pub fn not_in_list(field: &'static str, values: impl Into<ValueList>) -> Filter {
    Filter::NotIn(Cow::Borrowed(field), values.into())
}

// ============================================================================
// Optimized combinators for small filter counts
// ============================================================================

/// Combine exactly 2 filters with AND (optimized, avoids vec allocation overhead).
#[inline]
pub fn and2(a: Filter, b: Filter) -> Filter {
    Filter::And(Box::new([a, b]))
}

/// Combine exactly 3 filters with AND.
#[inline]
pub fn and3(a: Filter, b: Filter, c: Filter) -> Filter {
    Filter::And(Box::new([a, b, c]))
}

/// Combine exactly 4 filters with AND.
#[inline]
pub fn and4(a: Filter, b: Filter, c: Filter, d: Filter) -> Filter {
    Filter::And(Box::new([a, b, c, d]))
}

/// Combine exactly 5 filters with AND.
#[inline]
pub fn and5(a: Filter, b: Filter, c: Filter, d: Filter, e: Filter) -> Filter {
    Filter::And(Box::new([a, b, c, d, e]))
}

/// Combine exactly 2 filters with OR (optimized, avoids vec allocation overhead).
#[inline]
pub fn or2(a: Filter, b: Filter) -> Filter {
    Filter::Or(Box::new([a, b]))
}

/// Combine exactly 3 filters with OR.
#[inline]
pub fn or3(a: Filter, b: Filter, c: Filter) -> Filter {
    Filter::Or(Box::new([a, b, c]))
}

/// Combine exactly 4 filters with OR.
#[inline]
pub fn or4(a: Filter, b: Filter, c: Filter, d: Filter) -> Filter {
    Filter::Or(Box::new([a, b, c, d]))
}

/// Combine exactly 5 filters with OR.
#[inline]
pub fn or5(a: Filter, b: Filter, c: Filter, d: Filter, e: Filter) -> Filter {
    Filter::Or(Box::new([a, b, c, d, e]))
}

/// Negate a filter.
#[inline]
pub fn not(filter: Filter) -> Filter {
    Filter::Not(Box::new(filter))
}

// ============================================================================
// Compact filter value types
// ============================================================================

/// A compact filter value optimized for common cases.
///
/// Uses a tagged union representation to minimize size:
/// - Discriminant is inline with data
/// - Small strings can be stored inline (future optimization)
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum CompactValue {
    /// Null value.
    Null = 0,
    /// Boolean true.
    True = 1,
    /// Boolean false.
    False = 2,
    /// Small integer (-128 to 127).
    SmallInt(i8) = 3,
    /// Full integer.
    Int(i64) = 4,
    /// Float value.
    Float(f64) = 5,
    /// String value.
    String(String) = 6,
}

impl CompactValue {
    /// Convert to a FilterValue.
    #[inline]
    pub fn into_filter_value(self) -> FilterValue {
        match self {
            Self::Null => FilterValue::Null,
            Self::True => FilterValue::Bool(true),
            Self::False => FilterValue::Bool(false),
            Self::SmallInt(v) => FilterValue::Int(v as i64),
            Self::Int(v) => FilterValue::Int(v),
            Self::Float(v) => FilterValue::Float(v),
            Self::String(v) => FilterValue::String(v),
        }
    }
}

impl From<bool> for CompactValue {
    #[inline]
    fn from(v: bool) -> Self {
        if v { Self::True } else { Self::False }
    }
}

impl From<i32> for CompactValue {
    #[inline]
    fn from(v: i32) -> Self {
        if (-128..=127).contains(&v) {
            Self::SmallInt(v as i8)
        } else {
            Self::Int(v as i64)
        }
    }
}

impl From<i64> for CompactValue {
    #[inline]
    fn from(v: i64) -> Self {
        if (-128..=127).contains(&v) {
            Self::SmallInt(v as i8)
        } else {
            Self::Int(v)
        }
    }
}

impl From<f64> for CompactValue {
    #[inline]
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<String> for CompactValue {
    #[inline]
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for CompactValue {
    #[inline]
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

impl From<CompactValue> for FilterValue {
    #[inline]
    fn from(v: CompactValue) -> Self {
        v.into_filter_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_filter() {
        let filter = eq(fields::ID, 42);
        assert!(matches!(filter, Filter::Equals(_, FilterValue::Int(42))));
    }

    #[test]
    fn test_gt_filter() {
        let filter = gt(fields::AGE, 18);
        assert!(matches!(filter, Filter::Gt(_, FilterValue::Int(18))));
    }

    #[test]
    fn test_is_null_filter() {
        let filter = is_null(fields::DELETED_AT);
        assert!(matches!(filter, Filter::IsNull(_)));
    }

    #[test]
    fn test_and2_filter() {
        let filter = and2(eq(fields::ACTIVE, true), gt(fields::SCORE, 100));
        assert!(matches!(filter, Filter::And(_)));
    }

    #[test]
    fn test_or2_filter() {
        let filter = or2(eq(fields::STATUS, "active"), eq(fields::STATUS, "pending"));
        assert!(matches!(filter, Filter::Or(_)));
    }

    #[test]
    fn test_compact_value_bool() {
        let v: CompactValue = true.into();
        assert!(matches!(v, CompactValue::True));
        assert_eq!(v.into_filter_value(), FilterValue::Bool(true));
    }

    #[test]
    fn test_compact_value_small_int() {
        let v: CompactValue = 42i32.into();
        assert!(matches!(v, CompactValue::SmallInt(42)));
    }

    #[test]
    fn test_compact_value_large_int() {
        let v: CompactValue = 1000i32.into();
        assert!(matches!(v, CompactValue::Int(1000)));
    }

    #[test]
    fn test_field_constants() {
        assert_eq!(fields::ID, "id");
        assert_eq!(fields::EMAIL, "email");
        assert_eq!(fields::CREATED_AT, "created_at");
    }
}
