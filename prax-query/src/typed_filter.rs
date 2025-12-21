//! Type-level filter composition for zero-cost filter abstractions.
//!
//! This module provides a type-level approach to filter composition,
//! inspired by Diesel's zero-cost expression types. Filters are composed
//! at the type level, allowing the compiler to optimize away runtime overhead.
//!
//! # Performance
//!
//! Type-level filters avoid runtime overhead:
//! - No heap allocations for small filter trees
//! - Compiler can inline and optimize filter operations
//! - Stack-allocated filter storage for fixed-size combinations
//!
//! # Examples
//!
//! ```rust
//! use prax_query::typed_filter::{Eq, Gt, And, TypedFilter};
//!
//! // Type-level composition
//! let filter = And::new(
//!     Eq::new("active", true),
//!     Gt::new("score", 100),
//! );
//!
//! // Convert to runtime Filter when needed
//! let runtime_filter = filter.into_filter();
//! ```

use crate::filter::{Filter, FilterValue};
use std::borrow::Cow;

// ============================================================================
// Type-level filter trait
// ============================================================================

/// A filter expression that can be converted to a runtime Filter.
///
/// This trait is implemented by all type-level filter types.
/// The conversion is zero-cost for simple filters and minimal-allocation
/// for composite filters.
pub trait TypedFilter: Sized {
    /// Convert this typed filter to a runtime Filter.
    fn into_filter(self) -> Filter;

    /// Combine with another filter using AND.
    #[inline(always)]
    fn and<R: TypedFilter>(self, other: R) -> And<Self, R> {
        And {
            left: self,
            right: other,
        }
    }

    /// Combine with another filter using OR.
    #[inline(always)]
    fn or<R: TypedFilter>(self, other: R) -> Or<Self, R> {
        Or {
            left: self,
            right: other,
        }
    }

    /// Negate this filter.
    #[inline(always)]
    fn not(self) -> Not<Self> {
        Not { inner: self }
    }
}

/// Zero-allocation SQL generation trait.
///
/// This trait allows generating SQL directly without going through
/// the `Filter` enum, achieving maximum performance (~5ns target).
///
/// # Performance
///
/// Direct SQL generation avoids:
/// - Enum discriminant overhead
/// - Intermediate allocations
/// - Dynamic dispatch
///
/// # Example
///
/// ```rust
/// use prax_query::typed_filter::{Eq, DirectSql};
///
/// let filter = Eq::new("id", 42i64);
///
/// // Direct SQL generation (~5ns)
/// let mut sql = String::with_capacity(64);
/// let param_idx = filter.write_sql(&mut sql, 1);
/// // sql = "id = $1", param_idx = 2
/// ```
pub trait DirectSql {
    /// Write this filter's SQL directly to a buffer.
    ///
    /// Returns the next parameter index to use.
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize;

    /// Get the number of parameters this filter uses.
    fn param_count(&self) -> usize;
}

// ============================================================================
// Comparison filters
// ============================================================================

/// Equality filter: `field = value`
#[derive(Debug, Clone)]
pub struct Eq<V> {
    field: &'static str,
    value: V,
}

impl<V> Eq<V> {
    /// Create a new equality filter.
    #[inline(always)]
    pub fn new(field: &'static str, value: V) -> Self {
        Self { field, value }
    }
}

impl<V: Into<FilterValue>> TypedFilter for Eq<V> {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::Equals(Cow::Borrowed(self.field), self.value.into())
    }
}

impl<V: Clone> DirectSql for Eq<V> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        use crate::sql::POSTGRES_PLACEHOLDERS;
        buf.push_str(self.field);
        buf.push_str(" = ");
        // Use pre-computed placeholder if available
        if param_idx > 0 && param_idx <= POSTGRES_PLACEHOLDERS.len() {
            buf.push_str(POSTGRES_PLACEHOLDERS[param_idx - 1]);
        } else {
            use std::fmt::Write;
            let _ = write!(buf, "${}", param_idx);
        }
        param_idx + 1
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        1
    }
}

/// Not-equals filter: `field != value`
#[derive(Debug, Clone)]
pub struct Ne<V> {
    field: &'static str,
    value: V,
}

impl<V> Ne<V> {
    /// Create a new not-equals filter.
    #[inline(always)]
    pub fn new(field: &'static str, value: V) -> Self {
        Self { field, value }
    }
}

impl<V: Into<FilterValue>> TypedFilter for Ne<V> {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::NotEquals(Cow::Borrowed(self.field), self.value.into())
    }
}

impl<V: Clone> DirectSql for Ne<V> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        use crate::sql::POSTGRES_PLACEHOLDERS;
        buf.push_str(self.field);
        buf.push_str(" != ");
        if param_idx > 0 && param_idx <= POSTGRES_PLACEHOLDERS.len() {
            buf.push_str(POSTGRES_PLACEHOLDERS[param_idx - 1]);
        } else {
            use std::fmt::Write;
            let _ = write!(buf, "${}", param_idx);
        }
        param_idx + 1
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        1
    }
}

/// Less-than filter: `field < value`
#[derive(Debug, Clone)]
pub struct Lt<V> {
    field: &'static str,
    value: V,
}

impl<V> Lt<V> {
    /// Create a new less-than filter.
    #[inline(always)]
    pub fn new(field: &'static str, value: V) -> Self {
        Self { field, value }
    }
}

impl<V: Into<FilterValue>> TypedFilter for Lt<V> {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::Lt(Cow::Borrowed(self.field), self.value.into())
    }
}

impl<V: Clone> DirectSql for Lt<V> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        use crate::sql::POSTGRES_PLACEHOLDERS;
        buf.push_str(self.field);
        buf.push_str(" < ");
        if param_idx > 0 && param_idx <= POSTGRES_PLACEHOLDERS.len() {
            buf.push_str(POSTGRES_PLACEHOLDERS[param_idx - 1]);
        } else {
            use std::fmt::Write;
            let _ = write!(buf, "${}", param_idx);
        }
        param_idx + 1
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        1
    }
}

/// Less-than-or-equal filter: `field <= value`
#[derive(Debug, Clone)]
pub struct Lte<V> {
    field: &'static str,
    value: V,
}

impl<V> Lte<V> {
    /// Create a new less-than-or-equal filter.
    #[inline(always)]
    pub fn new(field: &'static str, value: V) -> Self {
        Self { field, value }
    }
}

impl<V: Into<FilterValue>> TypedFilter for Lte<V> {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::Lte(Cow::Borrowed(self.field), self.value.into())
    }
}

impl<V: Clone> DirectSql for Lte<V> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        use crate::sql::POSTGRES_PLACEHOLDERS;
        buf.push_str(self.field);
        buf.push_str(" <= ");
        if param_idx > 0 && param_idx <= POSTGRES_PLACEHOLDERS.len() {
            buf.push_str(POSTGRES_PLACEHOLDERS[param_idx - 1]);
        } else {
            use std::fmt::Write;
            let _ = write!(buf, "${}", param_idx);
        }
        param_idx + 1
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        1
    }
}

/// Greater-than filter: `field > value`
#[derive(Debug, Clone)]
pub struct Gt<V> {
    field: &'static str,
    value: V,
}

impl<V> Gt<V> {
    /// Create a new greater-than filter.
    #[inline(always)]
    pub fn new(field: &'static str, value: V) -> Self {
        Self { field, value }
    }
}

impl<V: Into<FilterValue>> TypedFilter for Gt<V> {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::Gt(Cow::Borrowed(self.field), self.value.into())
    }
}

impl<V: Clone> DirectSql for Gt<V> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        use crate::sql::POSTGRES_PLACEHOLDERS;
        buf.push_str(self.field);
        buf.push_str(" > ");
        if param_idx > 0 && param_idx <= POSTGRES_PLACEHOLDERS.len() {
            buf.push_str(POSTGRES_PLACEHOLDERS[param_idx - 1]);
        } else {
            use std::fmt::Write;
            let _ = write!(buf, "${}", param_idx);
        }
        param_idx + 1
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        1
    }
}

/// Greater-than-or-equal filter: `field >= value`
#[derive(Debug, Clone)]
pub struct Gte<V> {
    field: &'static str,
    value: V,
}

impl<V> Gte<V> {
    /// Create a new greater-than-or-equal filter.
    #[inline(always)]
    pub fn new(field: &'static str, value: V) -> Self {
        Self { field, value }
    }
}

impl<V: Into<FilterValue>> TypedFilter for Gte<V> {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::Gte(Cow::Borrowed(self.field), self.value.into())
    }
}

impl<V: Clone> DirectSql for Gte<V> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        use crate::sql::POSTGRES_PLACEHOLDERS;
        buf.push_str(self.field);
        buf.push_str(" >= ");
        if param_idx > 0 && param_idx <= POSTGRES_PLACEHOLDERS.len() {
            buf.push_str(POSTGRES_PLACEHOLDERS[param_idx - 1]);
        } else {
            use std::fmt::Write;
            let _ = write!(buf, "${}", param_idx);
        }
        param_idx + 1
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        1
    }
}

// ============================================================================
// Null checks
// ============================================================================

/// IS NULL filter: `field IS NULL`
#[derive(Debug, Clone, Copy)]
pub struct IsNull {
    field: &'static str,
}

impl IsNull {
    /// Create a new IS NULL filter.
    #[inline(always)]
    pub const fn new(field: &'static str) -> Self {
        Self { field }
    }
}

impl TypedFilter for IsNull {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::IsNull(Cow::Borrowed(self.field))
    }
}

impl DirectSql for IsNull {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        buf.push_str(self.field);
        buf.push_str(" IS NULL");
        param_idx // No parameters consumed
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        0
    }
}

/// IS NOT NULL filter: `field IS NOT NULL`
#[derive(Debug, Clone, Copy)]
pub struct IsNotNull {
    field: &'static str,
}

impl IsNotNull {
    /// Create a new IS NOT NULL filter.
    #[inline(always)]
    pub const fn new(field: &'static str) -> Self {
        Self { field }
    }
}

impl TypedFilter for IsNotNull {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::IsNotNull(Cow::Borrowed(self.field))
    }
}

impl DirectSql for IsNotNull {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        buf.push_str(self.field);
        buf.push_str(" IS NOT NULL");
        param_idx // No parameters consumed
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        0
    }
}

// ============================================================================
// String operations
// ============================================================================

/// LIKE %value% filter: `field LIKE '%value%'`
#[derive(Debug, Clone)]
pub struct Contains<V> {
    field: &'static str,
    value: V,
}

impl<V: Into<FilterValue>> Contains<V> {
    /// Create a new contains filter.
    #[inline]
    pub fn new(field: &'static str, value: V) -> Self {
        Self { field, value }
    }
}

impl<V: Into<FilterValue>> TypedFilter for Contains<V> {
    #[inline]
    fn into_filter(self) -> Filter {
        Filter::Contains(Cow::Borrowed(self.field), self.value.into())
    }
}

/// LIKE value% filter: `field LIKE 'value%'`
#[derive(Debug, Clone)]
pub struct StartsWith<V> {
    field: &'static str,
    value: V,
}

impl<V: Into<FilterValue>> StartsWith<V> {
    /// Create a new starts-with filter.
    #[inline]
    pub fn new(field: &'static str, value: V) -> Self {
        Self { field, value }
    }
}

impl<V: Into<FilterValue>> TypedFilter for StartsWith<V> {
    #[inline]
    fn into_filter(self) -> Filter {
        Filter::StartsWith(Cow::Borrowed(self.field), self.value.into())
    }
}

/// LIKE %value filter: `field LIKE '%value'`
#[derive(Debug, Clone)]
pub struct EndsWith<V> {
    field: &'static str,
    value: V,
}

impl<V: Into<FilterValue>> EndsWith<V> {
    /// Create a new ends-with filter.
    #[inline]
    pub fn new(field: &'static str, value: V) -> Self {
        Self { field, value }
    }
}

impl<V: Into<FilterValue>> TypedFilter for EndsWith<V> {
    #[inline]
    fn into_filter(self) -> Filter {
        Filter::EndsWith(Cow::Borrowed(self.field), self.value.into())
    }
}

// ============================================================================
// Composite filters (type-level composition)
// ============================================================================

/// AND composition of two filters: `L AND R`
#[derive(Debug, Clone)]
pub struct And<L, R> {
    left: L,
    right: R,
}

impl<L, R> And<L, R> {
    /// Create a new AND filter.
    #[inline(always)]
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L: TypedFilter, R: TypedFilter> TypedFilter for And<L, R> {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::And(Box::new([
            self.left.into_filter(),
            self.right.into_filter(),
        ]))
    }
}

impl<L: DirectSql, R: DirectSql> DirectSql for And<L, R> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        buf.push('(');
        let idx = self.left.write_sql(buf, param_idx);
        buf.push_str(" AND ");
        let idx = self.right.write_sql(buf, idx);
        buf.push(')');
        idx
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        self.left.param_count() + self.right.param_count()
    }
}

/// OR composition of two filters: `L OR R`
#[derive(Debug, Clone)]
pub struct Or<L, R> {
    left: L,
    right: R,
}

impl<L, R> Or<L, R> {
    /// Create a new OR filter.
    #[inline(always)]
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L: TypedFilter, R: TypedFilter> TypedFilter for Or<L, R> {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::Or(Box::new([
            self.left.into_filter(),
            self.right.into_filter(),
        ]))
    }
}

impl<L: DirectSql, R: DirectSql> DirectSql for Or<L, R> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        buf.push('(');
        let idx = self.left.write_sql(buf, param_idx);
        buf.push_str(" OR ");
        let idx = self.right.write_sql(buf, idx);
        buf.push(')');
        idx
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        self.left.param_count() + self.right.param_count()
    }
}

/// NOT filter: `NOT inner`
#[derive(Debug, Clone)]
pub struct Not<T> {
    inner: T,
}

impl<T> Not<T> {
    /// Create a new NOT filter.
    #[inline]
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: TypedFilter> TypedFilter for Not<T> {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::Not(Box::new(self.inner.into_filter()))
    }
}

impl<T: DirectSql> DirectSql for Not<T> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        buf.push_str("NOT (");
        let idx = self.inner.write_sql(buf, param_idx);
        buf.push(')');
        idx
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        self.inner.param_count()
    }
}

// ============================================================================
// Fixed-size filter arrays with const generics
// ============================================================================

/// A fixed-size array of AND filters.
///
/// Uses const generics to store filters inline without heap allocation.
#[derive(Debug, Clone)]
pub struct AndN<const N: usize> {
    filters: [Filter; N],
}

impl<const N: usize> AndN<N> {
    /// Create from an array of filters.
    #[inline]
    pub fn new(filters: [Filter; N]) -> Self {
        Self { filters }
    }
}

impl<const N: usize> TypedFilter for AndN<N> {
    #[inline]
    fn into_filter(self) -> Filter {
        Filter::And(Box::new(self.filters))
    }
}

/// A fixed-size array of OR filters.
///
/// Uses const generics to store filters inline without heap allocation.
#[derive(Debug, Clone)]
pub struct OrN<const N: usize> {
    filters: [Filter; N],
}

impl<const N: usize> OrN<N> {
    /// Create from an array of filters.
    #[inline]
    pub fn new(filters: [Filter; N]) -> Self {
        Self { filters }
    }
}

impl<const N: usize> TypedFilter for OrN<N> {
    #[inline]
    fn into_filter(self) -> Filter {
        Filter::Or(Box::new(self.filters))
    }
}

// ============================================================================
// IN filter with DirectSql support
// ============================================================================

/// IN filter for i64 values with optimized SQL generation.
#[derive(Debug, Clone)]
pub struct InI64<const N: usize> {
    field: &'static str,
    values: [i64; N],
}

impl<const N: usize> InI64<N> {
    /// Create a new IN filter with i64 values.
    #[inline(always)]
    pub fn new(field: &'static str, values: [i64; N]) -> Self {
        Self { field, values }
    }
}

impl<const N: usize> TypedFilter for InI64<N> {
    #[inline]
    fn into_filter(self) -> Filter {
        let values: Vec<FilterValue> = self.values.iter().map(|&v| FilterValue::Int(v)).collect();
        Filter::In(Cow::Borrowed(self.field), values)
    }
}

impl<const N: usize> DirectSql for InI64<N> {
    #[inline]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        use crate::sql::write_postgres_in_pattern;

        buf.push_str(self.field);
        buf.push_str(" IN (");
        write_postgres_in_pattern(buf, param_idx, N);
        buf.push(')');

        param_idx + N
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        N
    }
}

/// IN filter for string values with optimized SQL generation.
#[derive(Debug, Clone)]
pub struct InStr<const N: usize> {
    field: &'static str,
    values: [&'static str; N],
}

impl<const N: usize> InStr<N> {
    /// Create a new IN filter with static string values.
    #[inline(always)]
    pub fn new(field: &'static str, values: [&'static str; N]) -> Self {
        Self { field, values }
    }
}

impl<const N: usize> TypedFilter for InStr<N> {
    #[inline]
    fn into_filter(self) -> Filter {
        let values: Vec<FilterValue> = self.values.iter().map(|&v| FilterValue::String(v.to_string())).collect();
        Filter::In(Cow::Borrowed(self.field), values)
    }
}

impl<const N: usize> DirectSql for InStr<N> {
    #[inline]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        use crate::sql::write_postgres_in_pattern;

        buf.push_str(self.field);
        buf.push_str(" IN (");
        write_postgres_in_pattern(buf, param_idx, N);
        buf.push(')');

        param_idx + N
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        N
    }
}

/// Create an IN filter for i64 values with const generic size.
#[inline]
pub fn in_i64<const N: usize>(field: &'static str, values: [i64; N]) -> InI64<N> {
    InI64::new(field, values)
}

/// Create an IN filter for string values with const generic size.
#[inline]
pub fn in_str<const N: usize>(field: &'static str, values: [&'static str; N]) -> InStr<N> {
    InStr::new(field, values)
}

// ============================================================================
// Slice-based IN filters (zero-allocation for DirectSql)
// ============================================================================

/// IN filter that borrows an i64 slice - zero allocation for DirectSql.
///
/// This is the most performant way to create IN filters when you have
/// a slice of i64 values available and only need SQL generation.
///
/// # Performance
///
/// - `DirectSql::write_sql`: ~10ns for any size (only SQL generation, no value allocation)
/// - `TypedFilter::into_filter`: O(n) allocation (creates Vec<FilterValue>)
///
/// # Example
///
/// ```rust
/// use prax_query::typed_filter::{InI64Slice, DirectSql};
///
/// let ids = [1i64, 2, 3, 4, 5];
/// let filter = InI64Slice::new("id", &ids);
///
/// let mut sql = String::with_capacity(64);
/// let next_idx = filter.write_sql(&mut sql, 1);
/// assert_eq!(sql, "id IN ($1, $2, $3, $4, $5)");
/// assert_eq!(next_idx, 6);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct InI64Slice<'a> {
    field: &'static str,
    values: &'a [i64],
}

impl<'a> InI64Slice<'a> {
    /// Create a new IN filter borrowing an i64 slice.
    #[inline(always)]
    pub const fn new(field: &'static str, values: &'a [i64]) -> Self {
        Self { field, values }
    }
}

impl<'a> TypedFilter for InI64Slice<'a> {
    #[inline]
    fn into_filter(self) -> Filter {
        let values: Vec<FilterValue> = self.values.iter().map(|&v| FilterValue::Int(v)).collect();
        Filter::In(Cow::Borrowed(self.field), values)
    }
}

impl<'a> DirectSql for InI64Slice<'a> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        use crate::sql::write_postgres_in_pattern;

        buf.push_str(self.field);
        buf.push_str(" IN (");
        write_postgres_in_pattern(buf, param_idx, self.values.len());
        buf.push(')');

        param_idx + self.values.len()
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        self.values.len()
    }
}

/// IN filter that borrows a string slice - zero allocation for DirectSql.
#[derive(Debug, Clone, Copy)]
pub struct InStrSlice<'a> {
    field: &'static str,
    values: &'a [&'a str],
}

impl<'a> InStrSlice<'a> {
    /// Create a new IN filter borrowing a string slice.
    #[inline(always)]
    pub const fn new(field: &'static str, values: &'a [&'a str]) -> Self {
        Self { field, values }
    }
}

impl<'a> TypedFilter for InStrSlice<'a> {
    #[inline]
    fn into_filter(self) -> Filter {
        let values: Vec<FilterValue> = self.values.iter().map(|v| FilterValue::String(v.to_string())).collect();
        Filter::In(Cow::Borrowed(self.field), values)
    }
}

impl<'a> DirectSql for InStrSlice<'a> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        use crate::sql::write_postgres_in_pattern;

        buf.push_str(self.field);
        buf.push_str(" IN (");
        write_postgres_in_pattern(buf, param_idx, self.values.len());
        buf.push(')');

        param_idx + self.values.len()
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        self.values.len()
    }
}

/// NOT IN filter that borrows an i64 slice - zero allocation for DirectSql.
#[derive(Debug, Clone, Copy)]
pub struct NotInI64Slice<'a> {
    field: &'static str,
    values: &'a [i64],
}

impl<'a> NotInI64Slice<'a> {
    /// Create a new NOT IN filter borrowing an i64 slice.
    #[inline(always)]
    pub const fn new(field: &'static str, values: &'a [i64]) -> Self {
        Self { field, values }
    }
}

impl<'a> TypedFilter for NotInI64Slice<'a> {
    #[inline]
    fn into_filter(self) -> Filter {
        let values: Vec<FilterValue> = self.values.iter().map(|&v| FilterValue::Int(v)).collect();
        Filter::NotIn(Cow::Borrowed(self.field), values)
    }
}

impl<'a> DirectSql for NotInI64Slice<'a> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        use crate::sql::write_postgres_in_pattern;

        buf.push_str(self.field);
        buf.push_str(" NOT IN (");
        write_postgres_in_pattern(buf, param_idx, self.values.len());
        buf.push(')');

        param_idx + self.values.len()
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        self.values.len()
    }
}

/// Create an IN filter for i64 values from a slice (zero DirectSql allocation).
#[inline(always)]
pub fn in_i64_slice<'a>(field: &'static str, values: &'a [i64]) -> InI64Slice<'a> {
    InI64Slice::new(field, values)
}

/// Create an IN filter for string values from a slice (zero DirectSql allocation).
#[inline(always)]
pub fn in_str_slice<'a>(field: &'static str, values: &'a [&'a str]) -> InStrSlice<'a> {
    InStrSlice::new(field, values)
}

/// Create a NOT IN filter for i64 values from a slice (zero DirectSql allocation).
#[inline(always)]
pub fn not_in_i64_slice<'a>(field: &'static str, values: &'a [i64]) -> NotInI64Slice<'a> {
    NotInI64Slice::new(field, values)
}

// ============================================================================
// Stack-allocated AND/OR with DirectSql (truly zero-allocation)
// ============================================================================

/// Stack-allocated AND filter with 3 conditions (zero heap allocation).
#[derive(Debug, Clone)]
pub struct And3<A, B, C> {
    a: A,
    b: B,
    c: C,
}

impl<A, B, C> And3<A, B, C> {
    #[inline(always)]
    pub fn new(a: A, b: B, c: C) -> Self {
        Self { a, b, c }
    }
}

impl<A: TypedFilter, B: TypedFilter, C: TypedFilter> TypedFilter for And3<A, B, C> {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::And(Box::new([
            self.a.into_filter(),
            self.b.into_filter(),
            self.c.into_filter(),
        ]))
    }
}

impl<A: DirectSql, B: DirectSql, C: DirectSql> DirectSql for And3<A, B, C> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        buf.push('(');
        let idx = self.a.write_sql(buf, param_idx);
        buf.push_str(" AND ");
        let idx = self.b.write_sql(buf, idx);
        buf.push_str(" AND ");
        let idx = self.c.write_sql(buf, idx);
        buf.push(')');
        idx
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        self.a.param_count() + self.b.param_count() + self.c.param_count()
    }
}

/// Stack-allocated AND filter with 5 conditions (zero heap allocation).
#[derive(Debug, Clone)]
pub struct And5<A, B, C, D, E> {
    a: A,
    b: B,
    c: C,
    d: D,
    e: E,
}

impl<A, B, C, D, E> And5<A, B, C, D, E> {
    #[inline(always)]
    pub fn new(a: A, b: B, c: C, d: D, e: E) -> Self {
        Self { a, b, c, d, e }
    }
}

impl<A: TypedFilter, B: TypedFilter, C: TypedFilter, D: TypedFilter, E: TypedFilter> TypedFilter for And5<A, B, C, D, E> {
    #[inline(always)]
    fn into_filter(self) -> Filter {
        Filter::And(Box::new([
            self.a.into_filter(),
            self.b.into_filter(),
            self.c.into_filter(),
            self.d.into_filter(),
            self.e.into_filter(),
        ]))
    }
}

impl<A: DirectSql, B: DirectSql, C: DirectSql, D: DirectSql, E: DirectSql> DirectSql for And5<A, B, C, D, E> {
    #[inline(always)]
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize {
        buf.push('(');
        let idx = self.a.write_sql(buf, param_idx);
        buf.push_str(" AND ");
        let idx = self.b.write_sql(buf, idx);
        buf.push_str(" AND ");
        let idx = self.c.write_sql(buf, idx);
        buf.push_str(" AND ");
        let idx = self.d.write_sql(buf, idx);
        buf.push_str(" AND ");
        let idx = self.e.write_sql(buf, idx);
        buf.push(')');
        idx
    }

    #[inline(always)]
    fn param_count(&self) -> usize {
        self.a.param_count() + self.b.param_count() + self.c.param_count()
            + self.d.param_count() + self.e.param_count()
    }
}

/// Create a stack-allocated AND filter with 3 conditions.
#[inline(always)]
pub fn and3<A: TypedFilter, B: TypedFilter, C: TypedFilter>(a: A, b: B, c: C) -> And3<A, B, C> {
    And3::new(a, b, c)
}

/// Create a stack-allocated AND filter with 5 conditions.
#[inline(always)]
pub fn and5<A: TypedFilter, B: TypedFilter, C: TypedFilter, D: TypedFilter, E: TypedFilter>(
    a: A, b: B, c: C, d: D, e: E
) -> And5<A, B, C, D, E> {
    And5::new(a, b, c, d, e)
}

// ============================================================================
// Lazy filter builder (defers allocation)
// ============================================================================

/// A lazy filter that defers allocation until converted to Filter.
///
/// This is useful when building filters conditionally - the allocation
/// only happens when the final filter is materialized.
pub struct LazyFilter<F> {
    builder: F,
}

impl<F: FnOnce() -> Filter> LazyFilter<F> {
    /// Create a new lazy filter.
    #[inline]
    pub fn new(builder: F) -> Self {
        Self { builder }
    }
}

impl<F: FnOnce() -> Filter> TypedFilter for LazyFilter<F> {
    #[inline]
    fn into_filter(self) -> Filter {
        (self.builder)()
    }
}

/// Create a lazy filter that defers construction.
#[inline]
pub fn lazy<F: FnOnce() -> Filter>(f: F) -> LazyFilter<F> {
    LazyFilter::new(f)
}

// ============================================================================
// Conditional filter builder
// ============================================================================

/// A conditional filter that may or may not be included.
pub struct Maybe<T> {
    inner: Option<T>,
}

impl<T: TypedFilter> Maybe<T> {
    /// Create a conditional filter.
    #[inline]
    pub fn new(inner: Option<T>) -> Self {
        Self { inner }
    }

    /// Create from a condition.
    #[inline]
    pub fn when(condition: bool, filter: T) -> Self {
        Self {
            inner: if condition { Some(filter) } else { None },
        }
    }
}

impl<T: TypedFilter> TypedFilter for Maybe<T> {
    #[inline]
    fn into_filter(self) -> Filter {
        match self.inner {
            Some(f) => f.into_filter(),
            None => Filter::None,
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Create an equality filter.
#[inline]
pub fn eq<V: Into<FilterValue>>(field: &'static str, value: V) -> Eq<V> {
    Eq::new(field, value)
}

/// Create a not-equals filter.
#[inline]
pub fn ne<V: Into<FilterValue>>(field: &'static str, value: V) -> Ne<V> {
    Ne::new(field, value)
}

/// Create a less-than filter.
#[inline]
pub fn lt<V: Into<FilterValue>>(field: &'static str, value: V) -> Lt<V> {
    Lt::new(field, value)
}

/// Create a less-than-or-equal filter.
#[inline]
pub fn lte<V: Into<FilterValue>>(field: &'static str, value: V) -> Lte<V> {
    Lte::new(field, value)
}

/// Create a greater-than filter.
#[inline]
pub fn gt<V: Into<FilterValue>>(field: &'static str, value: V) -> Gt<V> {
    Gt::new(field, value)
}

/// Create a greater-than-or-equal filter.
#[inline]
pub fn gte<V: Into<FilterValue>>(field: &'static str, value: V) -> Gte<V> {
    Gte::new(field, value)
}

/// Create an IS NULL filter.
#[inline]
pub const fn is_null(field: &'static str) -> IsNull {
    IsNull::new(field)
}

/// Create an IS NOT NULL filter.
#[inline]
pub const fn is_not_null(field: &'static str) -> IsNotNull {
    IsNotNull::new(field)
}

/// Create a contains (LIKE %value%) filter.
#[inline]
pub fn contains<V: Into<FilterValue>>(field: &'static str, value: V) -> Contains<V> {
    Contains::new(field, value)
}

/// Create a starts-with (LIKE value%) filter.
#[inline]
pub fn starts_with<V: Into<FilterValue>>(field: &'static str, value: V) -> StartsWith<V> {
    StartsWith::new(field, value)
}

/// Create an ends-with (LIKE %value) filter.
#[inline]
pub fn ends_with<V: Into<FilterValue>>(field: &'static str, value: V) -> EndsWith<V> {
    EndsWith::new(field, value)
}

/// Create a fixed-size AND filter array.
#[inline]
pub fn and_n<const N: usize>(filters: [Filter; N]) -> AndN<N> {
    AndN::new(filters)
}

/// Create a fixed-size OR filter array.
#[inline]
pub fn or_n<const N: usize>(filters: [Filter; N]) -> OrN<N> {
    OrN::new(filters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_eq() {
        let filter = eq("id", 42).into_filter();
        assert!(matches!(filter, Filter::Equals(_, FilterValue::Int(42))));
    }

    #[test]
    fn test_typed_gt() {
        let filter = gt("age", 18).into_filter();
        assert!(matches!(filter, Filter::Gt(_, FilterValue::Int(18))));
    }

    #[test]
    fn test_typed_and() {
        let filter = eq("active", true).and(gt("score", 100)).into_filter();
        assert!(matches!(filter, Filter::And(_)));
    }

    #[test]
    fn test_typed_or() {
        let filter = eq("status", "a").or(eq("status", "b")).into_filter();
        assert!(matches!(filter, Filter::Or(_)));
    }

    #[test]
    fn test_typed_not() {
        let filter = eq("deleted", true).not().into_filter();
        assert!(matches!(filter, Filter::Not(_)));
    }

    #[test]
    fn test_typed_is_null() {
        let filter = is_null("deleted_at").into_filter();
        assert!(matches!(filter, Filter::IsNull(_)));
    }

    #[test]
    fn test_typed_is_not_null() {
        let filter = is_not_null("email").into_filter();
        assert!(matches!(filter, Filter::IsNotNull(_)));
    }

    #[test]
    fn test_and_n_const_generic() {
        let filter = and_n([
            eq("a", 1).into_filter(),
            eq("b", 2).into_filter(),
            eq("c", 3).into_filter(),
        ])
        .into_filter();
        match filter {
            Filter::And(filters) => assert_eq!(filters.len(), 3),
            _ => panic!("Expected And filter"),
        }
    }

    #[test]
    fn test_or_n_const_generic() {
        let filter = or_n([
            eq("status", "a").into_filter(),
            eq("status", "b").into_filter(),
        ])
        .into_filter();
        match filter {
            Filter::Or(filters) => assert_eq!(filters.len(), 2),
            _ => panic!("Expected Or filter"),
        }
    }

    #[test]
    fn test_lazy_filter() {
        let filter = lazy(|| Filter::Equals("id".into(), FilterValue::Int(42))).into_filter();
        assert!(matches!(filter, Filter::Equals(_, FilterValue::Int(42))));
    }

    #[test]
    fn test_maybe_filter_some() {
        let filter = Maybe::when(true, eq("active", true)).into_filter();
        assert!(matches!(filter, Filter::Equals(_, _)));
    }

    #[test]
    fn test_maybe_filter_none() {
        let filter = Maybe::when(false, eq("active", true)).into_filter();
        assert!(matches!(filter, Filter::None));
    }

    #[test]
    fn test_chained_composition() {
        // Complex type-level composition
        let filter = eq("active", true)
            .and(gt("score", 100))
            .and(is_not_null("email"))
            .into_filter();

        // Should create nested And structure
        assert!(matches!(filter, Filter::And(_)));
    }
}
