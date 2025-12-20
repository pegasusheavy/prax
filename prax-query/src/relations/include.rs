//! Include specifications for eager loading relations.

use std::collections::HashMap;

use crate::filter::Filter;
use crate::pagination::Pagination;
use crate::types::OrderBy;

/// Specification for including a relation in a query.
#[derive(Debug, Clone)]
pub struct IncludeSpec {
    /// Name of the relation to include.
    pub relation_name: String,
    /// Filter to apply to the related records.
    pub filter: Option<Filter>,
    /// Ordering for the related records.
    pub order_by: Option<OrderBy>,
    /// Pagination for the related records.
    pub pagination: Option<Pagination>,
    /// Nested includes.
    pub nested: HashMap<String, IncludeSpec>,
    /// Whether to include the count of related records.
    pub include_count: bool,
}

impl IncludeSpec {
    /// Create a new include spec for a relation.
    pub fn new(relation_name: impl Into<String>) -> Self {
        Self {
            relation_name: relation_name.into(),
            filter: None,
            order_by: None,
            pagination: None,
            nested: HashMap::new(),
            include_count: false,
        }
    }

    /// Add a filter to the included relation.
    pub fn r#where(mut self, filter: impl Into<Filter>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    /// Set ordering for the included relation.
    pub fn order_by(mut self, order: impl Into<OrderBy>) -> Self {
        self.order_by = Some(order.into());
        self
    }

    /// Skip records in the included relation.
    pub fn skip(mut self, n: u64) -> Self {
        self.pagination = Some(
            self.pagination
                .unwrap_or_default()
                .skip(n),
        );
        self
    }

    /// Take a limited number of records from the included relation.
    pub fn take(mut self, n: u64) -> Self {
        self.pagination = Some(
            self.pagination
                .unwrap_or_default()
                .take(n),
        );
        self
    }

    /// Include a nested relation.
    pub fn include(mut self, nested: IncludeSpec) -> Self {
        self.nested.insert(nested.relation_name.clone(), nested);
        self
    }

    /// Include the count of related records.
    pub fn with_count(mut self) -> Self {
        self.include_count = true;
        self
    }

    /// Check if there are nested includes.
    pub fn has_nested(&self) -> bool {
        !self.nested.is_empty()
    }

    /// Get all nested include specs.
    pub fn nested_specs(&self) -> impl Iterator<Item = &IncludeSpec> {
        self.nested.values()
    }
}

/// Builder for include specifications.
///
/// This is typically used by the generated code to provide a fluent API.
#[derive(Debug, Clone, Default)]
pub struct Include {
    specs: HashMap<String, IncludeSpec>,
}

impl Include {
    /// Create a new empty include builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a relation to include.
    pub fn add(mut self, spec: IncludeSpec) -> Self {
        self.specs.insert(spec.relation_name.clone(), spec);
        self
    }

    /// Add multiple relations to include.
    pub fn add_many(mut self, specs: impl IntoIterator<Item = IncludeSpec>) -> Self {
        for spec in specs {
            self.specs.insert(spec.relation_name.clone(), spec);
        }
        self
    }

    /// Get an include spec by relation name.
    pub fn get(&self, relation: &str) -> Option<&IncludeSpec> {
        self.specs.get(relation)
    }

    /// Check if a relation is included.
    pub fn contains(&self, relation: &str) -> bool {
        self.specs.contains_key(relation)
    }

    /// Get all include specs.
    pub fn specs(&self) -> impl Iterator<Item = &IncludeSpec> {
        self.specs.values()
    }

    /// Check if there are any includes.
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    /// Get the number of includes.
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    /// Merge another include into this one.
    pub fn merge(mut self, other: Include) -> Self {
        self.specs.extend(other.specs);
        self
    }
}

impl From<IncludeSpec> for Include {
    fn from(spec: IncludeSpec) -> Self {
        Self::new().add(spec)
    }
}

impl FromIterator<IncludeSpec> for Include {
    fn from_iter<T: IntoIterator<Item = IncludeSpec>>(iter: T) -> Self {
        Self::new().add_many(iter)
    }
}

/// Helper function to create an include spec.
pub fn include(relation: impl Into<String>) -> IncludeSpec {
    IncludeSpec::new(relation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OrderByField;

    #[test]
    fn test_include_spec_basic() {
        let spec = IncludeSpec::new("posts");
        assert_eq!(spec.relation_name, "posts");
        assert!(spec.filter.is_none());
        assert!(spec.order_by.is_none());
    }

    #[test]
    fn test_include_spec_with_options() {
        let spec = IncludeSpec::new("posts")
            .order_by(OrderByField::desc("created_at"))
            .take(5)
            .with_count();

        assert!(spec.order_by.is_some());
        assert!(spec.pagination.is_some());
        assert!(spec.include_count);
    }

    #[test]
    fn test_include_spec_nested() {
        let spec = IncludeSpec::new("posts")
            .include(IncludeSpec::new("comments").take(10));

        assert!(spec.has_nested());
        assert!(spec.nested.contains_key("comments"));
    }

    #[test]
    fn test_include_builder() {
        let includes = Include::new()
            .add(IncludeSpec::new("posts"))
            .add(IncludeSpec::new("profile"));

        assert_eq!(includes.len(), 2);
        assert!(includes.contains("posts"));
        assert!(includes.contains("profile"));
    }

    #[test]
    fn test_include_from_iter() {
        let includes: Include = vec![
            IncludeSpec::new("posts"),
            IncludeSpec::new("comments"),
        ]
        .into_iter()
        .collect();

        assert_eq!(includes.len(), 2);
    }
}

