//! Filter types for building WHERE clauses.

use serde::{Deserialize, Serialize};

/// A filter value that can be used in comparisons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    /// Null value.
    Null,
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Int(i64),
    /// Float value.
    Float(f64),
    /// String value.
    String(String),
    /// JSON value.
    Json(serde_json::Value),
    /// List of values.
    List(Vec<FilterValue>),
}

impl FilterValue {
    /// Check if this is a null value.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Convert to SQL parameter placeholder.
    pub fn to_sql_placeholder(&self, param_index: usize) -> String {
        format!("${}", param_index)
    }
}

impl From<bool> for FilterValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<i32> for FilterValue {
    fn from(v: i32) -> Self {
        Self::Int(v as i64)
    }
}

impl From<i64> for FilterValue {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<f64> for FilterValue {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<String> for FilterValue {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for FilterValue {
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

impl<T: Into<FilterValue>> From<Vec<T>> for FilterValue {
    fn from(v: Vec<T>) -> Self {
        Self::List(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<FilterValue>> From<Option<T>> for FilterValue {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Self::Null,
        }
    }
}

/// Scalar filter operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarFilter<T> {
    /// Equals the value.
    Equals(T),
    /// Not equals the value.
    Not(Box<T>),
    /// In a list of values.
    In(Vec<T>),
    /// Not in a list of values.
    NotIn(Vec<T>),
    /// Less than.
    Lt(T),
    /// Less than or equal.
    Lte(T),
    /// Greater than.
    Gt(T),
    /// Greater than or equal.
    Gte(T),
    /// Contains (for strings).
    Contains(T),
    /// Starts with (for strings).
    StartsWith(T),
    /// Ends with (for strings).
    EndsWith(T),
    /// Is null.
    IsNull,
    /// Is not null.
    IsNotNull,
}

impl<T: Into<FilterValue>> ScalarFilter<T> {
    /// Convert to a Filter with the given column name.
    pub fn into_filter(self, column: impl Into<String>) -> Filter {
        let column = column.into();
        match self {
            Self::Equals(v) => Filter::Equals(column, v.into()),
            Self::Not(v) => Filter::NotEquals(column, (*v).into()),
            Self::In(values) => {
                Filter::In(column, values.into_iter().map(Into::into).collect())
            }
            Self::NotIn(values) => {
                Filter::NotIn(column, values.into_iter().map(Into::into).collect())
            }
            Self::Lt(v) => Filter::Lt(column, v.into()),
            Self::Lte(v) => Filter::Lte(column, v.into()),
            Self::Gt(v) => Filter::Gt(column, v.into()),
            Self::Gte(v) => Filter::Gte(column, v.into()),
            Self::Contains(v) => Filter::Contains(column, v.into()),
            Self::StartsWith(v) => Filter::StartsWith(column, v.into()),
            Self::EndsWith(v) => Filter::EndsWith(column, v.into()),
            Self::IsNull => Filter::IsNull(column),
            Self::IsNotNull => Filter::IsNotNull(column),
        }
    }
}

/// A complete filter that can be converted to SQL.
#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    /// No filter (always true).
    None,

    /// Equals comparison.
    Equals(String, FilterValue),
    /// Not equals comparison.
    NotEquals(String, FilterValue),

    /// Less than comparison.
    Lt(String, FilterValue),
    /// Less than or equal comparison.
    Lte(String, FilterValue),
    /// Greater than comparison.
    Gt(String, FilterValue),
    /// Greater than or equal comparison.
    Gte(String, FilterValue),

    /// In a list of values.
    In(String, Vec<FilterValue>),
    /// Not in a list of values.
    NotIn(String, Vec<FilterValue>),

    /// Contains (LIKE %value%).
    Contains(String, FilterValue),
    /// Starts with (LIKE value%).
    StartsWith(String, FilterValue),
    /// Ends with (LIKE %value).
    EndsWith(String, FilterValue),

    /// Is null check.
    IsNull(String),
    /// Is not null check.
    IsNotNull(String),

    /// Logical AND of multiple filters.
    And(Vec<Filter>),
    /// Logical OR of multiple filters.
    Or(Vec<Filter>),
    /// Logical NOT of a filter.
    Not(Box<Filter>),
}

impl Filter {
    /// Create an empty filter (matches everything).
    pub fn none() -> Self {
        Self::None
    }

    /// Check if this filter is empty.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Create an AND filter.
    pub fn and(filters: impl IntoIterator<Item = Filter>) -> Self {
        let filters: Vec<_> = filters.into_iter().filter(|f| !f.is_none()).collect();
        match filters.len() {
            0 => Self::None,
            1 => filters.into_iter().next().unwrap(),
            _ => Self::And(filters),
        }
    }

    /// Create an OR filter.
    pub fn or(filters: impl IntoIterator<Item = Filter>) -> Self {
        let filters: Vec<_> = filters.into_iter().filter(|f| !f.is_none()).collect();
        match filters.len() {
            0 => Self::None,
            1 => filters.into_iter().next().unwrap(),
            _ => Self::Or(filters),
        }
    }

    /// Create a NOT filter.
    pub fn not(filter: Filter) -> Self {
        if filter.is_none() {
            return Self::None;
        }
        Self::Not(Box::new(filter))
    }

    /// Combine with another filter using AND.
    pub fn and_then(self, other: Filter) -> Self {
        if self.is_none() {
            return other;
        }
        if other.is_none() {
            return self;
        }
        match self {
            Self::And(mut filters) => {
                filters.push(other);
                Self::And(filters)
            }
            _ => Self::And(vec![self, other]),
        }
    }

    /// Combine with another filter using OR.
    pub fn or_else(self, other: Filter) -> Self {
        if self.is_none() {
            return other;
        }
        if other.is_none() {
            return self;
        }
        match self {
            Self::Or(mut filters) => {
                filters.push(other);
                Self::Or(filters)
            }
            _ => Self::Or(vec![self, other]),
        }
    }

    /// Generate SQL for this filter with parameter placeholders.
    /// Returns (sql, params) where params are the values to bind.
    pub fn to_sql(&self, param_offset: usize) -> (String, Vec<FilterValue>) {
        let mut params = Vec::new();
        let sql = self.to_sql_with_params(param_offset, &mut params);
        (sql, params)
    }

    fn to_sql_with_params(&self, mut param_idx: usize, params: &mut Vec<FilterValue>) -> String {
        match self {
            Self::None => "TRUE".to_string(),

            Self::Equals(col, val) => {
                if val.is_null() {
                    format!("{} IS NULL", col)
                } else {
                    params.push(val.clone());
                    param_idx += params.len();
                    format!("{} = ${}", col, param_idx)
                }
            }
            Self::NotEquals(col, val) => {
                if val.is_null() {
                    format!("{} IS NOT NULL", col)
                } else {
                    params.push(val.clone());
                    param_idx += params.len();
                    format!("{} != ${}", col, param_idx)
                }
            }

            Self::Lt(col, val) => {
                params.push(val.clone());
                param_idx += params.len();
                format!("{} < ${}", col, param_idx)
            }
            Self::Lte(col, val) => {
                params.push(val.clone());
                param_idx += params.len();
                format!("{} <= ${}", col, param_idx)
            }
            Self::Gt(col, val) => {
                params.push(val.clone());
                param_idx += params.len();
                format!("{} > ${}", col, param_idx)
            }
            Self::Gte(col, val) => {
                params.push(val.clone());
                param_idx += params.len();
                format!("{} >= ${}", col, param_idx)
            }

            Self::In(col, values) => {
                if values.is_empty() {
                    return "FALSE".to_string();
                }
                let placeholders: Vec<_> = values
                    .iter()
                    .map(|v| {
                        params.push(v.clone());
                        param_idx += params.len();
                        format!("${}", param_idx)
                    })
                    .collect();
                format!("{} IN ({})", col, placeholders.join(", "))
            }
            Self::NotIn(col, values) => {
                if values.is_empty() {
                    return "TRUE".to_string();
                }
                let placeholders: Vec<_> = values
                    .iter()
                    .map(|v| {
                        params.push(v.clone());
                        param_idx += params.len();
                        format!("${}", param_idx)
                    })
                    .collect();
                format!("{} NOT IN ({})", col, placeholders.join(", "))
            }

            Self::Contains(col, val) => {
                if let FilterValue::String(s) = val {
                    params.push(FilterValue::String(format!("%{}%", s)));
                } else {
                    params.push(val.clone());
                }
                param_idx += params.len();
                format!("{} LIKE ${}", col, param_idx)
            }
            Self::StartsWith(col, val) => {
                if let FilterValue::String(s) = val {
                    params.push(FilterValue::String(format!("{}%", s)));
                } else {
                    params.push(val.clone());
                }
                param_idx += params.len();
                format!("{} LIKE ${}", col, param_idx)
            }
            Self::EndsWith(col, val) => {
                if let FilterValue::String(s) = val {
                    params.push(FilterValue::String(format!("%{}", s)));
                } else {
                    params.push(val.clone());
                }
                param_idx += params.len();
                format!("{} LIKE ${}", col, param_idx)
            }

            Self::IsNull(col) => format!("{} IS NULL", col),
            Self::IsNotNull(col) => format!("{} IS NOT NULL", col),

            Self::And(filters) => {
                if filters.is_empty() {
                    return "TRUE".to_string();
                }
                let parts: Vec<_> = filters
                    .iter()
                    .map(|f| f.to_sql_with_params(param_idx + params.len(), params))
                    .collect();
                format!("({})", parts.join(" AND "))
            }
            Self::Or(filters) => {
                if filters.is_empty() {
                    return "FALSE".to_string();
                }
                let parts: Vec<_> = filters
                    .iter()
                    .map(|f| f.to_sql_with_params(param_idx + params.len(), params))
                    .collect();
                format!("({})", parts.join(" OR "))
            }
            Self::Not(filter) => {
                let inner = filter.to_sql_with_params(param_idx, params);
                format!("NOT ({})", inner)
            }
        }
    }
}

impl Default for Filter {
    fn default() -> Self {
        Self::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_value_from() {
        assert_eq!(FilterValue::from(42i32), FilterValue::Int(42));
        assert_eq!(FilterValue::from("hello"), FilterValue::String("hello".to_string()));
        assert_eq!(FilterValue::from(true), FilterValue::Bool(true));
    }

    #[test]
    fn test_scalar_filter_equals() {
        let filter = ScalarFilter::Equals("test@example.com".to_string())
            .into_filter("email");

        let (sql, params) = filter.to_sql(0);
        assert_eq!(sql, "email = $1");
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_filter_and() {
        let f1 = Filter::Equals("name".to_string(), "Alice".into());
        let f2 = Filter::Gt("age".to_string(), FilterValue::Int(18));
        let combined = Filter::and([f1, f2]);

        let (sql, params) = combined.to_sql(0);
        assert!(sql.contains("AND"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_filter_or() {
        let f1 = Filter::Equals("status".to_string(), "active".into());
        let f2 = Filter::Equals("status".to_string(), "pending".into());
        let combined = Filter::or([f1, f2]);

        let (sql, _) = combined.to_sql(0);
        assert!(sql.contains("OR"));
    }

    #[test]
    fn test_filter_not() {
        let filter = Filter::not(Filter::Equals("deleted".to_string(), FilterValue::Bool(true)));

        let (sql, _) = filter.to_sql(0);
        assert!(sql.contains("NOT"));
    }

    #[test]
    fn test_filter_is_null() {
        let filter = Filter::IsNull("deleted_at".to_string());
        let (sql, params) = filter.to_sql(0);
        assert_eq!(sql, "deleted_at IS NULL");
        assert!(params.is_empty());
    }

    #[test]
    fn test_filter_in() {
        let filter = Filter::In(
            "status".to_string(),
            vec!["active".into(), "pending".into()],
        );
        let (sql, params) = filter.to_sql(0);
        assert!(sql.contains("IN"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_filter_contains() {
        let filter = Filter::Contains("email".to_string(), "example".into());
        let (sql, params) = filter.to_sql(0);
        assert!(sql.contains("LIKE"));
        assert_eq!(params.len(), 1);
        if let FilterValue::String(s) = &params[0] {
            assert!(s.contains("%example%"));
        }
    }
}

