//! Compile-time filter construction macros.
//!
//! These macros provide zero-cost filter construction at compile time,
//! avoiding runtime allocations for static filter patterns.
//!
//! # Examples
//!
//! ```rust
//! use prax_query::{filter, and_filter, or_filter};
//! use prax_query::filter::Filter;
//!
//! // Simple equality filter
//! let f = filter!(id == 42);
//! assert!(matches!(f, Filter::Equals(_, _)));
//!
//! // Multiple conditions with AND
//! let f = and_filter!(
//!     filter!(active == true),
//!     filter!(score > 100)
//! );
//!
//! // Multiple conditions with OR
//! let f = or_filter!(
//!     filter!(status == "pending"),
//!     filter!(status == "processing")
//! );
//! ```

/// Create a filter expression with minimal allocations.
///
/// # Syntax
///
/// - `filter!(field == value)` - Equality
/// - `filter!(field != value)` - Not equals
/// - `filter!(field > value)` - Greater than
/// - `filter!(field >= value)` - Greater than or equal
/// - `filter!(field < value)` - Less than
/// - `filter!(field <= value)` - Less than or equal
/// - `filter!(field is null)` - IS NULL
/// - `filter!(field is not null)` - IS NOT NULL
/// - `filter!(field contains value)` - LIKE %value%
/// - `filter!(field starts_with value)` - LIKE value%
/// - `filter!(field ends_with value)` - LIKE %value
/// - `filter!(field in [v1, v2, ...])` - IN clause
/// - `filter!(field not in [v1, v2, ...])` - NOT IN clause
///
/// # Examples
///
/// ```rust
/// use prax_query::filter;
/// use prax_query::filter::Filter;
///
/// // Basic comparisons
/// let f = filter!(id == 42);
/// let f = filter!(age > 18);
/// let f = filter!(score >= 100);
///
/// // String comparisons
/// let f = filter!(name == "Alice");
/// let f = filter!(email contains "@example.com");
///
/// // Null checks
/// let f = filter!(deleted_at is null);
/// let f = filter!(verified_at is not null);
///
/// // IN clauses
/// let f = filter!(status in ["active", "pending"]);
/// ```
#[macro_export]
macro_rules! filter {
    // Equality: field == value
    ($field:ident == $value:expr) => {{
        $crate::filter::Filter::Equals(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
            $value.into(),
        )
    }};

    // Not equals: field != value
    ($field:ident != $value:expr) => {{
        $crate::filter::Filter::NotEquals(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
            $value.into(),
        )
    }};

    // Greater than: field > value
    ($field:ident > $value:expr) => {{
        $crate::filter::Filter::Gt(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
            $value.into(),
        )
    }};

    // Greater than or equal: field >= value
    ($field:ident >= $value:expr) => {{
        $crate::filter::Filter::Gte(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
            $value.into(),
        )
    }};

    // Less than: field < value
    ($field:ident < $value:expr) => {{
        $crate::filter::Filter::Lt(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
            $value.into(),
        )
    }};

    // Less than or equal: field <= value
    ($field:ident <= $value:expr) => {{
        $crate::filter::Filter::Lte(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
            $value.into(),
        )
    }};

    // IS NULL: field is null
    ($field:ident is null) => {{
        $crate::filter::Filter::IsNull(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
        )
    }};

    // IS NOT NULL: field is not null
    ($field:ident is not null) => {{
        $crate::filter::Filter::IsNotNull(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
        )
    }};

    // LIKE %value%: field contains value
    ($field:ident contains $value:expr) => {{
        $crate::filter::Filter::Contains(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
            $value.into(),
        )
    }};

    // LIKE value%: field starts_with value
    ($field:ident starts_with $value:expr) => {{
        $crate::filter::Filter::StartsWith(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
            $value.into(),
        )
    }};

    // LIKE %value: field ends_with value
    ($field:ident ends_with $value:expr) => {{
        $crate::filter::Filter::EndsWith(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
            $value.into(),
        )
    }};

    // IN clause: field in [values]
    ($field:ident in [$($value:expr),* $(,)?]) => {{
        $crate::filter::Filter::In(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
            ::std::vec![$($value.into()),*],
        )
    }};

    // NOT IN clause: field not in [values]
    ($field:ident not in [$($value:expr),* $(,)?]) => {{
        $crate::filter::Filter::NotIn(
            ::std::borrow::Cow::Borrowed(stringify!($field)),
            ::std::vec![$($value.into()),*],
        )
    }};
}

/// Combine filters with AND.
///
/// # Examples
///
/// ```rust
/// use prax_query::{filter, and_filter};
///
/// let f = and_filter!(
///     filter!(active == true),
///     filter!(score > 100)
/// );
///
/// // Multiple filters
/// let f = and_filter!(
///     filter!(status == "active"),
///     filter!(age >= 18),
///     filter!(verified == true)
/// );
/// ```
#[macro_export]
macro_rules! and_filter {
    // Two filters (optimized)
    ($a:expr, $b:expr $(,)?) => {{
        $crate::filter::Filter::And(Box::new([$a, $b]))
    }};

    // Three filters
    ($a:expr, $b:expr, $c:expr $(,)?) => {{
        $crate::filter::Filter::And(Box::new([$a, $b, $c]))
    }};

    // Four filters
    ($a:expr, $b:expr, $c:expr, $d:expr $(,)?) => {{
        $crate::filter::Filter::And(Box::new([$a, $b, $c, $d]))
    }};

    // Five filters
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr $(,)?) => {{
        $crate::filter::Filter::And(Box::new([$a, $b, $c, $d, $e]))
    }};

    // Variable number of filters
    ($($filter:expr),+ $(,)?) => {{
        $crate::filter::Filter::And(Box::new([$($filter),+]))
    }};
}

/// Combine filters with OR.
///
/// # Examples
///
/// ```rust
/// use prax_query::{filter, or_filter};
///
/// let f = or_filter!(
///     filter!(status == "pending"),
///     filter!(status == "processing")
/// );
/// ```
#[macro_export]
macro_rules! or_filter {
    // Two filters (optimized)
    ($a:expr, $b:expr $(,)?) => {{
        $crate::filter::Filter::Or(Box::new([$a, $b]))
    }};

    // Three filters
    ($a:expr, $b:expr, $c:expr $(,)?) => {{
        $crate::filter::Filter::Or(Box::new([$a, $b, $c]))
    }};

    // Variable number of filters
    ($($filter:expr),+ $(,)?) => {{
        $crate::filter::Filter::Or(Box::new([$($filter),+]))
    }};
}

/// Negate a filter.
///
/// # Examples
///
/// ```rust
/// use prax_query::{filter, not_filter};
///
/// let f = not_filter!(filter!(deleted == true));
/// ```
#[macro_export]
macro_rules! not_filter {
    ($filter:expr) => {{ $crate::filter::Filter::Not(Box::new($filter)) }};
}

#[cfg(test)]
mod tests {
    use crate::filter::{Filter, FilterValue};

    #[test]
    fn test_filter_eq_macro() {
        let f = filter!(id == 42);
        match f {
            Filter::Equals(field, FilterValue::Int(42)) => {
                assert_eq!(field.as_ref(), "id");
            }
            _ => panic!("Expected Equals filter"),
        }
    }

    #[test]
    fn test_filter_ne_macro() {
        let f = filter!(status != "deleted");
        assert!(matches!(f, Filter::NotEquals(_, _)));
    }

    #[test]
    fn test_filter_gt_macro() {
        let f = filter!(age > 18);
        assert!(matches!(f, Filter::Gt(_, FilterValue::Int(18))));
    }

    #[test]
    fn test_filter_gte_macro() {
        let f = filter!(score >= 100);
        assert!(matches!(f, Filter::Gte(_, FilterValue::Int(100))));
    }

    #[test]
    fn test_filter_lt_macro() {
        let f = filter!(price < 50);
        assert!(matches!(f, Filter::Lt(_, FilterValue::Int(50))));
    }

    #[test]
    fn test_filter_lte_macro() {
        let f = filter!(quantity <= 10);
        assert!(matches!(f, Filter::Lte(_, FilterValue::Int(10))));
    }

    #[test]
    fn test_filter_is_null_macro() {
        let f = filter!(deleted_at is null);
        match f {
            Filter::IsNull(field) => {
                assert_eq!(field.as_ref(), "deleted_at");
            }
            _ => panic!("Expected IsNull filter"),
        }
    }

    #[test]
    fn test_filter_is_not_null_macro() {
        let f = filter!(verified_at is not null);
        match f {
            Filter::IsNotNull(field) => {
                assert_eq!(field.as_ref(), "verified_at");
            }
            _ => panic!("Expected IsNotNull filter"),
        }
    }

    #[test]
    fn test_filter_contains_macro() {
        let f = filter!(email contains "@example.com");
        assert!(matches!(f, Filter::Contains(_, _)));
    }

    #[test]
    fn test_filter_starts_with_macro() {
        let f = filter!(name starts_with "John");
        assert!(matches!(f, Filter::StartsWith(_, _)));
    }

    #[test]
    fn test_filter_ends_with_macro() {
        let f = filter!(email ends_with ".com");
        assert!(matches!(f, Filter::EndsWith(_, _)));
    }

    #[test]
    fn test_filter_in_macro() {
        let f = filter!(status in ["active", "pending", "processing"]);
        match f {
            Filter::In(field, values) => {
                assert_eq!(field.as_ref(), "status");
                assert_eq!(values.len(), 3);
            }
            _ => panic!("Expected In filter"),
        }
    }

    #[test]
    fn test_filter_not_in_macro() {
        let f = filter!(role not in ["admin", "superuser"]);
        match f {
            Filter::NotIn(field, values) => {
                assert_eq!(field.as_ref(), "role");
                assert_eq!(values.len(), 2);
            }
            _ => panic!("Expected NotIn filter"),
        }
    }

    #[test]
    fn test_and_filter_macro() {
        let f = and_filter!(filter!(active == true), filter!(score > 100));
        match f {
            Filter::And(filters) => {
                assert_eq!(filters.len(), 2);
            }
            _ => panic!("Expected And filter"),
        }
    }

    #[test]
    fn test_or_filter_macro() {
        let f = or_filter!(
            filter!(status == "pending"),
            filter!(status == "processing")
        );
        match f {
            Filter::Or(filters) => {
                assert_eq!(filters.len(), 2);
            }
            _ => panic!("Expected Or filter"),
        }
    }

    #[test]
    fn test_not_filter_macro() {
        let f = not_filter!(filter!(deleted == true));
        assert!(matches!(f, Filter::Not(_)));
    }

    #[test]
    fn test_complex_filter_macro() {
        // Complex nested filter
        let f = and_filter!(
            filter!(active == true),
            or_filter!(filter!(role == "admin"), filter!(role == "moderator")),
            filter!(age >= 18)
        );
        assert!(matches!(f, Filter::And(_)));
    }
}
