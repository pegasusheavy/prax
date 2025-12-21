//! Common types used in query building.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

/// Sort order for query results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortOrder {
    /// Ascending order (A-Z, 0-9, oldest first).
    Asc,
    /// Descending order (Z-A, 9-0, newest first).
    Desc,
}

impl SortOrder {
    /// Get the SQL keyword for this sort order.
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_sql())
    }
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Asc
    }
}

/// Null handling in sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NullsOrder {
    /// Nulls appear first in the results.
    First,
    /// Nulls appear last in the results.
    Last,
}

impl NullsOrder {
    /// Get the SQL clause for this null order.
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::First => "NULLS FIRST",
            Self::Last => "NULLS LAST",
        }
    }
}

/// Order by specification for a single field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderByField {
    /// The column name to order by.
    pub column: Cow<'static, str>,
    /// The sort order.
    pub order: SortOrder,
    /// Null handling (optional).
    pub nulls: Option<NullsOrder>,
}

impl OrderByField {
    /// Create a new order by field.
    pub fn new(column: impl Into<Cow<'static, str>>, order: SortOrder) -> Self {
        Self {
            column: column.into(),
            order,
            nulls: None,
        }
    }

    /// Create a new order by field with a static column name (zero allocation).
    #[inline]
    pub const fn new_static(column: &'static str, order: SortOrder) -> Self {
        Self {
            column: Cow::Borrowed(column),
            order,
            nulls: None,
        }
    }

    /// Set null handling.
    pub fn nulls(mut self, nulls: NullsOrder) -> Self {
        self.nulls = Some(nulls);
        self
    }

    /// Create an ascending order.
    pub fn asc(column: impl Into<Cow<'static, str>>) -> Self {
        Self::new(column, SortOrder::Asc)
    }

    /// Create a descending order.
    pub fn desc(column: impl Into<Cow<'static, str>>) -> Self {
        Self::new(column, SortOrder::Desc)
    }

    /// Create an ascending order with a static column name (zero allocation).
    #[inline]
    pub const fn asc_static(column: &'static str) -> Self {
        Self::new_static(column, SortOrder::Asc)
    }

    /// Create a descending order with a static column name (zero allocation).
    #[inline]
    pub const fn desc_static(column: &'static str) -> Self {
        Self::new_static(column, SortOrder::Desc)
    }

    /// Generate the SQL for this order by field.
    ///
    /// Optimized to write directly to a pre-sized buffer.
    pub fn to_sql(&self) -> String {
        // column + " " + ASC/DESC (4) + optional " NULLS FIRST/LAST" (12)
        let cap = self.column.len() + 5 + if self.nulls.is_some() { 12 } else { 0 };
        let mut sql = String::with_capacity(cap);
        self.write_sql(&mut sql);
        sql
    }

    /// Write the SQL directly to a buffer (zero allocation).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use prax_query::types::OrderByField;
    ///
    /// let field = OrderByField::desc("created_at");
    /// let mut buffer = String::with_capacity(64);
    /// buffer.push_str("ORDER BY ");
    /// field.write_sql(&mut buffer);
    /// assert_eq!(buffer, "ORDER BY created_at DESC");
    /// ```
    #[inline]
    pub fn write_sql(&self, buffer: &mut String) {
        buffer.push_str(&self.column);
        buffer.push(' ');
        buffer.push_str(self.order.as_sql());
        if let Some(nulls) = self.nulls {
            buffer.push(' ');
            buffer.push_str(nulls.as_sql());
        }
    }

    /// Get the estimated SQL length for capacity planning.
    #[inline]
    pub fn estimated_len(&self) -> usize {
        self.column.len() + 5 + if self.nulls.is_some() { 12 } else { 0 }
    }
}

/// Order by specification that can be a single field or multiple fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderBy {
    /// Order by a single field.
    Field(OrderByField),
    /// Order by multiple fields (frozen slice for memory efficiency).
    Fields(Box<[OrderByField]>),
}

impl OrderBy {
    /// Create an empty order by (no ordering).
    pub fn none() -> Self {
        Self::Fields(Box::new([]))
    }

    /// Check if the order by is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Field(_) => false,
            Self::Fields(fields) => fields.is_empty(),
        }
    }

    /// Add a field to the order by.
    pub fn then(self, field: OrderByField) -> Self {
        match self {
            Self::Field(existing) => Self::Fields(vec![existing, field].into_boxed_slice()),
            Self::Fields(existing) => {
                let mut fields: Vec<_> = existing.into_vec();
                fields.push(field);
                Self::Fields(fields.into_boxed_slice())
            }
        }
    }

    /// Create an OrderBy from multiple fields (optimized).
    pub fn from_fields(fields: impl IntoIterator<Item = OrderByField>) -> Self {
        let fields: Vec<_> = fields.into_iter().collect();
        match fields.len() {
            0 => Self::none(),
            1 => Self::Field(fields.into_iter().next().unwrap()),
            _ => Self::Fields(fields.into_boxed_slice()),
        }
    }

    /// Generate the SQL ORDER BY clause (without the "ORDER BY" keyword).
    ///
    /// Optimized to write directly to a pre-sized buffer.
    pub fn to_sql(&self) -> String {
        match self {
            Self::Field(field) => field.to_sql(),
            Self::Fields(fields) if fields.is_empty() => String::new(),
            Self::Fields(fields) => {
                // Estimate capacity
                let cap: usize = fields.iter().map(|f| f.estimated_len() + 2).sum();
                let mut sql = String::with_capacity(cap);
                self.write_sql(&mut sql);
                sql
            }
        }
    }

    /// Write the SQL ORDER BY clause directly to a buffer (zero allocation).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use prax_query::types::{OrderBy, OrderByField};
    ///
    /// let order = OrderBy::from_fields([
    ///     OrderByField::desc("created_at"),
    ///     OrderByField::asc("id"),
    /// ]);
    /// let mut buffer = String::with_capacity(64);
    /// buffer.push_str("ORDER BY ");
    /// order.write_sql(&mut buffer);
    /// assert_eq!(buffer, "ORDER BY created_at DESC, id ASC");
    /// ```
    #[inline]
    pub fn write_sql(&self, buffer: &mut String) {
        match self {
            Self::Field(field) => field.write_sql(buffer),
            Self::Fields(fields) => {
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        buffer.push_str(", ");
                    }
                    field.write_sql(buffer);
                }
            }
        }
    }

    /// Get the number of fields in this OrderBy.
    #[inline]
    pub fn field_count(&self) -> usize {
        match self {
            Self::Field(_) => 1,
            Self::Fields(fields) => fields.len(),
        }
    }
}

impl From<OrderByField> for OrderBy {
    fn from(field: OrderByField) -> Self {
        Self::Field(field)
    }
}

impl From<Vec<OrderByField>> for OrderBy {
    fn from(fields: Vec<OrderByField>) -> Self {
        match fields.len() {
            0 => Self::none(),
            1 => Self::Field(fields.into_iter().next().unwrap()),
            _ => Self::Fields(fields.into_boxed_slice()),
        }
    }
}

/// Builder for constructing OrderBy with pre-allocated capacity.
#[derive(Debug)]
pub struct OrderByBuilder {
    fields: Vec<OrderByField>,
}

impl OrderByBuilder {
    /// Create a new builder with the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            fields: Vec::with_capacity(capacity),
        }
    }

    /// Add a field to the order by.
    #[inline]
    pub fn push(mut self, field: OrderByField) -> Self {
        self.fields.push(field);
        self
    }

    /// Add an ascending field.
    #[inline]
    pub fn asc(self, column: impl Into<Cow<'static, str>>) -> Self {
        self.push(OrderByField::asc(column))
    }

    /// Add a descending field.
    #[inline]
    pub fn desc(self, column: impl Into<Cow<'static, str>>) -> Self {
        self.push(OrderByField::desc(column))
    }

    /// Build the OrderBy.
    #[inline]
    pub fn build(self) -> OrderBy {
        OrderBy::from(self.fields)
    }
}

/// Pre-defined common ordering patterns (zero allocation).
pub mod order_patterns {
    use super::*;

    /// Order by `created_at DESC` (most recent first).
    pub const CREATED_AT_DESC: OrderByField = OrderByField::desc_static("created_at");

    /// Order by `created_at ASC` (oldest first).
    pub const CREATED_AT_ASC: OrderByField = OrderByField::asc_static("created_at");

    /// Order by `updated_at DESC`.
    pub const UPDATED_AT_DESC: OrderByField = OrderByField::desc_static("updated_at");

    /// Order by `updated_at ASC`.
    pub const UPDATED_AT_ASC: OrderByField = OrderByField::asc_static("updated_at");

    /// Order by `id ASC`.
    pub const ID_ASC: OrderByField = OrderByField::asc_static("id");

    /// Order by `id DESC`.
    pub const ID_DESC: OrderByField = OrderByField::desc_static("id");

    /// Order by `name ASC`.
    pub const NAME_ASC: OrderByField = OrderByField::asc_static("name");

    /// Order by `name DESC`.
    pub const NAME_DESC: OrderByField = OrderByField::desc_static("name");

    /// Order by `price ASC`.
    pub const PRICE_ASC: OrderByField = OrderByField::asc_static("price");

    /// Order by `price DESC`.
    pub const PRICE_DESC: OrderByField = OrderByField::desc_static("price");
}

/// Field selection for queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Select {
    /// Select all fields.
    All,
    /// Select specific fields.
    Fields(Vec<String>),
    /// Select a single field.
    Field(String),
}

impl Select {
    /// Create a selection for all fields.
    pub fn all() -> Self {
        Self::All
    }

    /// Create a selection for specific fields.
    pub fn fields(fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Fields(fields.into_iter().map(Into::into).collect())
    }

    /// Create a selection for a single field.
    pub fn field(field: impl Into<String>) -> Self {
        Self::Field(field.into())
    }

    /// Check if this selects all fields.
    pub fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Get the list of field names.
    pub fn field_names(&self) -> Vec<&str> {
        match self {
            Self::All => vec!["*"],
            Self::Fields(fields) => fields.iter().map(String::as_str).collect(),
            Self::Field(field) => vec![field.as_str()],
        }
    }

    /// Generate the SQL column list.
    pub fn to_sql(&self) -> String {
        match self {
            Self::All => "*".to_string(),
            Self::Fields(fields) => {
                // Estimate capacity
                let cap: usize = fields.iter().map(|f| f.len() + 2).sum();
                let mut sql = String::with_capacity(cap);
                self.write_sql(&mut sql);
                sql
            }
            Self::Field(field) => field.clone(),
        }
    }

    /// Write the SQL column list directly to a buffer (zero allocation).
    #[inline]
    pub fn write_sql(&self, buffer: &mut String) {
        match self {
            Self::All => buffer.push('*'),
            Self::Fields(fields) => {
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        buffer.push_str(", ");
                    }
                    buffer.push_str(field);
                }
            }
            Self::Field(field) => buffer.push_str(field),
        }
    }
}

impl Default for Select {
    fn default() -> Self {
        Self::All
    }
}

/// Set parameter for updates - either set a value or leave unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetParam<T> {
    /// Set the field to this value.
    Set(T),
    /// Leave the field unchanged.
    Unset,
}

impl<T> SetParam<T> {
    /// Check if a value is set.
    pub fn is_set(&self) -> bool {
        matches!(self, Self::Set(_))
    }

    /// Get the inner value if set.
    pub fn get(&self) -> Option<&T> {
        match self {
            Self::Set(v) => Some(v),
            Self::Unset => None,
        }
    }

    /// Take the inner value if set.
    pub fn take(self) -> Option<T> {
        match self {
            Self::Set(v) => Some(v),
            Self::Unset => None,
        }
    }

    /// Map the inner value.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> SetParam<U> {
        match self {
            Self::Set(v) => SetParam::Set(f(v)),
            Self::Unset => SetParam::Unset,
        }
    }
}

impl<T> Default for SetParam<T> {
    fn default() -> Self {
        Self::Unset
    }
}

impl<T> From<T> for SetParam<T> {
    fn from(value: T) -> Self {
        Self::Set(value)
    }
}

impl<T> From<Option<T>> for SetParam<T> {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => Self::Set(v),
            None => Self::Unset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_order() {
        assert_eq!(SortOrder::Asc.as_sql(), "ASC");
        assert_eq!(SortOrder::Desc.as_sql(), "DESC");
    }

    #[test]
    fn test_order_by_field() {
        let field = OrderByField::desc("created_at");
        assert_eq!(field.to_sql(), "created_at DESC");

        let field_with_nulls = OrderByField::asc("name").nulls(NullsOrder::Last);
        assert_eq!(field_with_nulls.to_sql(), "name ASC NULLS LAST");
    }

    #[test]
    fn test_order_by_field_static() {
        let field = OrderByField::desc_static("created_at");
        assert_eq!(field.to_sql(), "created_at DESC");

        let field = OrderByField::asc_static("id");
        assert_eq!(field.to_sql(), "id ASC");
    }

    #[test]
    fn test_order_by_field_write_sql() {
        let field = OrderByField::desc("created_at");
        let mut buffer = String::with_capacity(32);
        field.write_sql(&mut buffer);
        assert_eq!(buffer, "created_at DESC");

        let field = OrderByField::asc("name").nulls(NullsOrder::First);
        let mut buffer = String::with_capacity(32);
        field.write_sql(&mut buffer);
        assert_eq!(buffer, "name ASC NULLS FIRST");
    }

    #[test]
    fn test_order_by_multiple() {
        let order =
            OrderBy::Field(OrderByField::desc("created_at")).then(OrderByField::asc("name"));
        assert_eq!(order.to_sql(), "created_at DESC, name ASC");
    }

    #[test]
    fn test_order_by_from_fields() {
        let order =
            OrderBy::from_fields([OrderByField::desc("created_at"), OrderByField::asc("id")]);
        assert_eq!(order.to_sql(), "created_at DESC, id ASC");
        assert_eq!(order.field_count(), 2);
    }

    #[test]
    fn test_order_by_write_sql() {
        let order =
            OrderBy::from_fields([OrderByField::desc("created_at"), OrderByField::asc("id")]);
        let mut buffer = String::with_capacity(64);
        buffer.push_str("ORDER BY ");
        order.write_sql(&mut buffer);
        assert_eq!(buffer, "ORDER BY created_at DESC, id ASC");
    }

    #[test]
    fn test_order_by_builder() {
        let order = OrderByBuilder::with_capacity(3)
            .desc("created_at")
            .asc("name")
            .asc("id")
            .build();
        assert_eq!(order.to_sql(), "created_at DESC, name ASC, id ASC");
        assert_eq!(order.field_count(), 3);
    }

    #[test]
    fn test_order_patterns() {
        assert_eq!(order_patterns::CREATED_AT_DESC.to_sql(), "created_at DESC");
        assert_eq!(order_patterns::ID_ASC.to_sql(), "id ASC");
        assert_eq!(order_patterns::NAME_ASC.to_sql(), "name ASC");
    }

    #[test]
    fn test_select() {
        assert_eq!(Select::all().to_sql(), "*");
        assert_eq!(Select::field("id").to_sql(), "id");
        assert_eq!(
            Select::fields(["id", "name", "email"]).to_sql(),
            "id, name, email"
        );
    }

    #[test]
    fn test_select_write_sql() {
        let select = Select::fields(["id", "name", "email"]);
        let mut buffer = String::with_capacity(32);
        buffer.push_str("SELECT ");
        select.write_sql(&mut buffer);
        assert_eq!(buffer, "SELECT id, name, email");
    }

    #[test]
    fn test_set_param() {
        let set: SetParam<i32> = SetParam::Set(42);
        assert!(set.is_set());
        assert_eq!(set.get(), Some(&42));

        let unset: SetParam<i32> = SetParam::Unset;
        assert!(!unset.is_set());
        assert_eq!(unset.get(), None);
    }
}
