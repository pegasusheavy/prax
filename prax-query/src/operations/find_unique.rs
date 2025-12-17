//! FindUnique operation for querying a single record by unique constraint.

use std::marker::PhantomData;

use crate::error::QueryResult;
use crate::filter::Filter;
use crate::traits::{Model, QueryEngine};
use crate::types::Select;

/// A query operation that finds a single record by unique constraint.
///
/// # Example
///
/// ```rust,ignore
/// let user = client
///     .user()
///     .find_unique()
///     .where_(user::id::equals(1))
///     .exec()
///     .await?;
/// ```
pub struct FindUniqueOperation<E: QueryEngine, M: Model> {
    engine: E,
    filter: Filter,
    select: Select,
    _model: PhantomData<M>,
}

impl<E: QueryEngine, M: Model> FindUniqueOperation<E, M> {
    /// Create a new FindUnique operation.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            filter: Filter::None,
            select: Select::All,
            _model: PhantomData,
        }
    }

    /// Add a filter condition (should be on unique fields).
    pub fn where_(mut self, filter: impl Into<Filter>) -> Self {
        self.filter = filter.into();
        self
    }

    /// Select specific fields.
    pub fn select(mut self, select: impl Into<Select>) -> Self {
        self.select = select.into();
        self
    }

    /// Build the SQL query.
    pub fn build_sql(&self) -> (String, Vec<crate::filter::FilterValue>) {
        let (where_sql, params) = self.filter.to_sql(0);

        let mut sql = String::new();

        // SELECT clause
        sql.push_str("SELECT ");
        sql.push_str(&self.select.to_sql());

        // FROM clause
        sql.push_str(" FROM ");
        sql.push_str(M::TABLE_NAME);

        // WHERE clause
        if !self.filter.is_none() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_sql);
        }

        // LIMIT 1 for unique query
        sql.push_str(" LIMIT 1");

        (sql, params)
    }

    /// Execute the query and return the result (errors if not found).
    pub async fn exec(self) -> QueryResult<M>
    where
        M: Send + 'static,
    {
        let (sql, params) = self.build_sql();
        self.engine.query_one::<M>(&sql, params).await
    }

    /// Execute the query and return an optional result.
    pub async fn exec_optional(self) -> QueryResult<Option<M>>
    where
        M: Send + 'static,
    {
        let (sql, params) = self.build_sql();
        self.engine.query_optional::<M>(&sql, params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::QueryError;
    use crate::filter::FilterValue;

    struct TestModel;

    impl Model for TestModel {
        const MODEL_NAME: &'static str = "TestModel";
        const TABLE_NAME: &'static str = "test_models";
        const PRIMARY_KEY: &'static [&'static str] = &["id"];
        const COLUMNS: &'static [&'static str] = &["id", "name", "email"];
    }

    #[derive(Clone)]
    struct MockEngine;

    impl QueryEngine for MockEngine {
        fn query_many<T: Model + Send + 'static>(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> crate::traits::BoxFuture<'_, QueryResult<Vec<T>>> {
            Box::pin(async { Ok(Vec::new()) })
        }

        fn query_one<T: Model + Send + 'static>(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> crate::traits::BoxFuture<'_, QueryResult<T>> {
            Box::pin(async { Err(QueryError::not_found("test")) })
        }

        fn query_optional<T: Model + Send + 'static>(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> crate::traits::BoxFuture<'_, QueryResult<Option<T>>> {
            Box::pin(async { Ok(None) })
        }

        fn execute_insert<T: Model + Send + 'static>(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> crate::traits::BoxFuture<'_, QueryResult<T>> {
            Box::pin(async { Err(QueryError::not_found("test")) })
        }

        fn execute_update<T: Model + Send + 'static>(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> crate::traits::BoxFuture<'_, QueryResult<Vec<T>>> {
            Box::pin(async { Ok(Vec::new()) })
        }

        fn execute_delete(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> crate::traits::BoxFuture<'_, QueryResult<u64>> {
            Box::pin(async { Ok(0) })
        }

        fn execute_raw(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> crate::traits::BoxFuture<'_, QueryResult<u64>> {
            Box::pin(async { Ok(0) })
        }

        fn count(
            &self,
            _sql: &str,
            _params: Vec<FilterValue>,
        ) -> crate::traits::BoxFuture<'_, QueryResult<u64>> {
            Box::pin(async { Ok(0) })
        }
    }

    #[test]
    fn test_find_unique_basic() {
        let op = FindUniqueOperation::<MockEngine, TestModel>::new(MockEngine)
            .where_(Filter::Equals("id".to_string(), FilterValue::Int(1)));

        let (sql, params) = op.build_sql();

        assert!(sql.contains("SELECT * FROM test_models"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("LIMIT 1"));
        assert_eq!(params.len(), 1);
    }
}

