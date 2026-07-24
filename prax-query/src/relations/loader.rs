//! Relation loading strategies and loaders.

use std::collections::HashMap;

use crate::filter::FilterValue;
use crate::traits::QueryEngine;

use super::include::IncludeSpec;
use super::spec::RelationSpec;

/// Strategy for loading relations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RelationLoadStrategy {
    /// Load relations in separate queries (default, N+1 safe with batching).
    #[default]
    Separate,
    /// Load relations using JOINs (single query, good for one-to-one/many-to-one).
    Join,
    /// Load relations lazily on access.
    Lazy,
}

impl RelationLoadStrategy {
    /// Check if this is a separate query strategy.
    pub fn is_separate(&self) -> bool {
        matches!(self, Self::Separate)
    }

    /// Check if this is a join strategy.
    pub fn is_join(&self) -> bool {
        matches!(self, Self::Join)
    }

    /// Check if this is lazy loading.
    pub fn is_lazy(&self) -> bool {
        matches!(self, Self::Lazy)
    }
}

/// Relation loader for executing relation queries.
pub struct RelationLoader<E: QueryEngine> {
    engine: E,
    strategy: RelationLoadStrategy,
    batch_size: usize,
}

impl<E: QueryEngine> RelationLoader<E> {
    /// Create a new relation loader.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            strategy: RelationLoadStrategy::Separate,
            batch_size: 100,
        }
    }

    /// Set the loading strategy.
    pub fn with_strategy(mut self, strategy: RelationLoadStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the batch size for separate queries.
    ///
    /// Used by [`Self::build_one_to_many_queries`] to chunk parent IDs into
    /// bounded `IN (...)` lists, one statement per chunk.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Get the engine.
    pub fn engine(&self) -> &E {
        &self.engine
    }

    /// Build a query for loading a one-to-many relation.
    ///
    /// Placeholders and the nested `Filter::to_sql` call are emitted through
    /// the engine's dialect (`QueryEngine::dialect()`), so the statement
    /// matches the connected backend.
    pub fn build_one_to_many_query(
        &self,
        spec: &RelationSpec,
        include: &IncludeSpec,
        parent_ids: &[FilterValue],
    ) -> (String, Vec<FilterValue>) {
        let dialect = self.engine.dialect();

        let mut sql = format!(
            "SELECT * FROM {} WHERE {} IN (",
            spec.related_table,
            spec.references.first().unwrap_or(&"id".to_string())
        );

        let placeholders: Vec<_> = (1..=parent_ids.len())
            .map(|i| dialect.placeholder(i))
            .collect();
        sql.push_str(&placeholders.join(", "));
        sql.push(')');

        let mut params = parent_ids.to_vec();

        // Apply filter if present; ordering and pagination still apply below.
        if let Some(ref filter) = include.filter {
            let (filter_sql, filter_params) = filter.to_sql(parent_ids.len(), dialect);
            sql.push_str(" AND ");
            sql.push_str(&filter_sql);
            params.extend(filter_params);
        }

        // Apply ordering
        if let Some(ref order) = include.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(&order.to_sql());
        }

        // Apply pagination (per-parent limits need subquery or window functions)
        if let Some(ref pagination) = include.pagination {
            let pagination_sql = pagination.to_sql();
            if !pagination_sql.is_empty() {
                sql.push(' ');
                sql.push_str(&pagination_sql);
            }
        }

        (sql, params)
    }

    /// Build one-to-many queries batched by [`Self::with_batch_size`].
    ///
    /// Chunks `parent_ids` into groups of at most `batch_size` and emits one
    /// statement per chunk — ceil(N / batch_size) statements — keeping each
    /// `IN (...)` list bounded. Callers using the separate-query strategy
    /// should execute every statement and merge the results.
    pub fn build_one_to_many_queries(
        &self,
        spec: &RelationSpec,
        include: &IncludeSpec,
        parent_ids: &[FilterValue],
    ) -> Vec<(String, Vec<FilterValue>)> {
        // Guard against a zero batch size, which would panic `slice::chunks`.
        let batch_size = self.batch_size.max(1);
        parent_ids
            .chunks(batch_size)
            .map(|chunk| self.build_one_to_many_query(spec, include, chunk))
            .collect()
    }

    /// Build a query for loading a many-to-one relation.
    pub fn build_many_to_one_query(
        &self,
        spec: &RelationSpec,
        child_foreign_keys: &[FilterValue],
    ) -> (String, Vec<FilterValue>) {
        let default_pk = "id".to_string();
        let pk = spec.references.first().unwrap_or(&default_pk);

        let mut sql = format!("SELECT * FROM {} WHERE {} IN (", spec.related_table, pk);

        let placeholders: Vec<_> = (1..=child_foreign_keys.len())
            .map(|i| format!("${}", i))
            .collect();
        sql.push_str(&placeholders.join(", "));
        sql.push(')');

        (sql, child_foreign_keys.to_vec())
    }

    /// Build a query for loading a many-to-many relation.
    pub fn build_many_to_many_query(
        &self,
        spec: &RelationSpec,
        include: &IncludeSpec,
        parent_ids: &[FilterValue],
    ) -> (String, Vec<FilterValue>) {
        let jt = spec.join_table.as_ref().unwrap_or_else(|| {
            panic!(
                "build_many_to_many_query: relation `{}` (table `{}`) has no join table; \
                 a many-to-many RelationSpec must be constructed via \
                 RelationSpec::many_to_many(...), which requires a JoinTableSpec",
                spec.name, spec.related_table
            )
        });

        let mut sql = format!(
            "SELECT t.*, jt.{} as _parent_id FROM {} t \
             INNER JOIN {} jt ON t.{} = jt.{} \
             WHERE jt.{} IN (",
            jt.source_column,
            spec.related_table,
            jt.table_name,
            spec.references.first().unwrap_or(&"id".to_string()),
            jt.target_column,
            jt.source_column
        );

        let placeholders: Vec<_> = (1..=parent_ids.len()).map(|i| format!("${}", i)).collect();
        sql.push_str(&placeholders.join(", "));
        sql.push(')');

        // Apply ordering
        if let Some(ref order) = include.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(&order.to_sql());
        }

        (sql, parent_ids.to_vec())
    }
}

impl<E: QueryEngine + Clone> Clone for RelationLoader<E> {
    fn clone(&self) -> Self {
        Self {
            engine: self.engine.clone(),
            strategy: self.strategy,
            batch_size: self.batch_size,
        }
    }
}

/// Result of loading relations, keyed by parent ID.
pub type RelationLoadResult<T> = HashMap<String, Vec<T>>;

/// Batch relation loading context.
#[derive(Debug)]
pub struct BatchLoadContext {
    /// Parent IDs to load relations for.
    pub parent_ids: Vec<FilterValue>,
    /// Field to group results by.
    pub group_by_field: String,
}

impl BatchLoadContext {
    /// Create a new batch load context.
    pub fn new(parent_ids: Vec<FilterValue>, group_by_field: impl Into<String>) -> Self {
        Self {
            parent_ids,
            group_by_field: group_by_field.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{QueryError, QueryResult};
    use crate::filter::Filter;
    use crate::traits::{BoxFuture, Model};
    use crate::types::OrderByField;

    struct TestModel;

    impl Model for TestModel {
        const MODEL_NAME: &'static str = "TestModel";
        const TABLE_NAME: &'static str = "test_models";
        const PRIMARY_KEY: &'static [&'static str] = &["id"];
        const COLUMNS: &'static [&'static str] = &["id", "name"];
    }

    #[derive(Clone)]
    struct MockEngine;

    impl QueryEngine for MockEngine {
        fn dialect(&self) -> &dyn crate::dialect::SqlDialect {
            &crate::dialect::Postgres
        }

        fn query_many<T: Model + Send + 'static>(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> BoxFuture<'_, QueryResult<Vec<T>>> {
            Box::pin(async { Ok(Vec::new()) })
        }

        fn query_one<T: Model + Send + 'static>(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> BoxFuture<'_, QueryResult<T>> {
            Box::pin(async { Err(QueryError::not_found("test")) })
        }

        fn query_optional<T: Model + Send + 'static>(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> BoxFuture<'_, QueryResult<Option<T>>> {
            Box::pin(async { Ok(None) })
        }

        fn execute_insert<T: Model + Send + 'static>(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> BoxFuture<'_, QueryResult<T>> {
            Box::pin(async { Err(QueryError::not_found("test")) })
        }

        fn execute_update<T: Model + Send + 'static>(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> BoxFuture<'_, QueryResult<Vec<T>>> {
            Box::pin(async { Ok(Vec::new()) })
        }

        fn execute_delete(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> BoxFuture<'_, QueryResult<u64>> {
            Box::pin(async { Ok(0) })
        }

        fn execute_raw(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> BoxFuture<'_, QueryResult<u64>> {
            Box::pin(async { Ok(0) })
        }

        fn count(&self, _sql: &str, _params: Vec<FilterValue>) -> BoxFuture<'_, QueryResult<u64>> {
            Box::pin(async { Ok(0) })
        }
    }

    #[test]
    fn test_relation_load_strategy() {
        assert!(RelationLoadStrategy::Separate.is_separate());
        assert!(RelationLoadStrategy::Join.is_join());
        assert!(RelationLoadStrategy::Lazy.is_lazy());
    }

    #[test]
    fn test_one_to_many_query() {
        let loader = RelationLoader::new(MockEngine);
        let spec = RelationSpec::one_to_many("posts", "Post", "posts").references(["author_id"]);
        let include = IncludeSpec::new("posts");
        let parent_ids = vec![FilterValue::Int(1), FilterValue::Int(2)];

        let (sql, params) = loader.build_one_to_many_query(&spec, &include, &parent_ids);

        assert!(sql.contains("SELECT * FROM posts"));
        assert!(sql.contains("WHERE author_id IN"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_one_to_many_query_with_filter_order_and_pagination() {
        let loader = RelationLoader::new(MockEngine);
        let spec = RelationSpec::one_to_many("posts", "Post", "posts").references(["author_id"]);
        let include = IncludeSpec::new("posts")
            .r#where(Filter::Equals("published".into(), FilterValue::Bool(true)))
            .order_by(OrderByField::desc("created_at"))
            .take(5);
        let parent_ids = vec![FilterValue::Int(1), FilterValue::Int(2)];

        let (sql, params) = loader.build_one_to_many_query(&spec, &include, &parent_ids);

        assert!(sql.contains("WHERE author_id IN"));
        assert!(sql.contains("AND"));
        assert!(sql.contains("ORDER BY created_at DESC"));
        assert!(sql.contains("LIMIT 5"));
        // Ordering/pagination must come after the filter clause.
        assert!(sql.find("AND").unwrap() < sql.find("ORDER BY").unwrap());
        assert!(sql.find("ORDER BY").unwrap() < sql.find("LIMIT").unwrap());
        // 2 parent IDs + 1 filter param.
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_one_to_many_batched_queries() {
        let loader = RelationLoader::new(MockEngine).with_batch_size(2);
        let spec = RelationSpec::one_to_many("posts", "Post", "posts").references(["author_id"]);
        let include = IncludeSpec::new("posts");
        let parent_ids: Vec<_> = (1..=5).map(FilterValue::Int).collect();

        let queries = loader.build_one_to_many_queries(&spec, &include, &parent_ids);

        // ceil(5 / 2) = 3 statements.
        assert_eq!(queries.len(), 3);
        assert_eq!(queries[0].1.len(), 2);
        assert_eq!(queries[1].1.len(), 2);
        assert_eq!(queries[2].1.len(), 1);
        // Each statement's IN list is bounded by the batch size.
        assert!(queries[0].0.contains("$1, $2"));
        assert!(queries[2].0.contains("$1)"));
        assert!(!queries[2].0.contains("$2"));
    }

    #[test]
    fn test_many_to_one_query() {
        let loader = RelationLoader::new(MockEngine);
        let spec = RelationSpec::many_to_one("author", "User", "users").references(["id"]);
        let foreign_keys = vec![FilterValue::Int(1), FilterValue::Int(2)];

        let (sql, params) = loader.build_many_to_one_query(&spec, &foreign_keys);

        assert!(sql.contains("SELECT * FROM users"));
        assert!(sql.contains("WHERE id IN"));
        assert_eq!(params.len(), 2);
    }
}
