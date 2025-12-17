//! Common types used in query building.

use serde::{Deserialize, Serialize};
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
    pub column: String,
    /// The sort order.
    pub order: SortOrder,
    /// Null handling (optional).
    pub nulls: Option<NullsOrder>,
}

impl OrderByField {
    /// Create a new order by field.
    pub fn new(column: impl Into<String>, order: SortOrder) -> Self {
        Self {
            column: column.into(),
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
    pub fn asc(column: impl Into<String>) -> Self {
        Self::new(column, SortOrder::Asc)
    }

    /// Create a descending order.
    pub fn desc(column: impl Into<String>) -> Self {
        Self::new(column, SortOrder::Desc)
    }

    /// Generate the SQL for this order by field.
    pub fn to_sql(&self) -> String {
        let mut sql = format!("{} {}", self.column, self.order.as_sql());
        if let Some(nulls) = self.nulls {
            sql.push(' ');
            sql.push_str(nulls.as_sql());
        }
        sql
    }
}

/// Order by specification that can be a single field or multiple fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderBy {
    /// Order by a single field.
    Field(OrderByField),
    /// Order by multiple fields.
    Fields(Vec<OrderByField>),
}

impl OrderBy {
    /// Create an empty order by (no ordering).
    pub fn none() -> Self {
        Self::Fields(Vec::new())
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
            Self::Field(existing) => Self::Fields(vec![existing, field]),
            Self::Fields(mut fields) => {
                fields.push(field);
                Self::Fields(fields)
            }
        }
    }

    /// Generate the SQL ORDER BY clause (without the "ORDER BY" keyword).
    pub fn to_sql(&self) -> String {
        match self {
            Self::Field(field) => field.to_sql(),
            Self::Fields(fields) => fields
                .iter()
                .map(|f| f.to_sql())
                .collect::<Vec<_>>()
                .join(", "),
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
        Self::Fields(fields)
    }
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
            Self::Fields(fields) => fields.join(", "),
            Self::Field(field) => field.clone(),
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
    fn test_order_by_multiple() {
        let order = OrderBy::Field(OrderByField::desc("created_at"))
            .then(OrderByField::asc("name"));
        assert_eq!(order.to_sql(), "created_at DESC, name ASC");
    }

    #[test]
    fn test_select() {
        assert_eq!(Select::all().to_sql(), "*");
        assert_eq!(Select::field("id").to_sql(), "id");
        assert_eq!(Select::fields(["id", "name", "email"]).to_sql(), "id, name, email");
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

