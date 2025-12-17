//! Count operation for counting records.

use std::marker::PhantomData;

use crate::error::QueryResult;
use crate::filter::{Filter, FilterValue};
use crate::traits::{Model, QueryEngine};

/// A count operation for counting records.
///
/// # Example
///
/// ```rust,ignore
/// let count = client
///     .user()
///     .count()
///     .where_(user::active::equals(true))
///     .exec()
///     .await?;
/// ```
pub struct CountOperation<E: QueryEngine, M: Model> {
    engine: E,
    filter: Filter,
    distinct: Option<String>,
    _model: PhantomData<M>,
}

impl<E: QueryEngine, M: Model> CountOperation<E, M> {
    /// Create a new Count operation.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            filter: Filter::None,
            distinct: None,
            _model: PhantomData,
        }
    }

    /// Add a filter condition.
    pub fn where_(mut self, filter: impl Into<Filter>) -> Self {
        let new_filter = filter.into();
        self.filter = self.filter.and_then(new_filter);
        self
    }

    /// Count distinct values of a column.
    pub fn distinct(mut self, column: impl Into<String>) -> Self {
        self.distinct = Some(column.into());
        self
    }

    /// Build the SQL query.
    pub fn build_sql(&self) -> (String, Vec<FilterValue>) {
        let (where_sql, params) = self.filter.to_sql(0);

        let mut sql = String::new();

        // SELECT COUNT clause
        sql.push_str("SELECT COUNT(");
        match &self.distinct {
            Some(col) => {
                sql.push_str("DISTINCT ");
                sql.push_str(col);
            }
            None => sql.push('*'),
        }
        sql.push(')');

        // FROM clause
        sql.push_str(" FROM ");
        sql.push_str(M::TABLE_NAME);

        // WHERE clause
        if !self.filter.is_none() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_sql);
        }

        (sql, params)
    }

    /// Execute the count query.
    pub async fn exec(self) -> QueryResult<u64> {
        let (sql, params) = self.build_sql();
        self.engine.count(&sql, params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::QueryError;

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
    fn test_count_basic() {
        let op = CountOperation::<MockEngine, TestModel>::new(MockEngine);
        let (sql, params) = op.build_sql();

        assert_eq!(sql, "SELECT COUNT(*) FROM test_models");
        assert!(params.is_empty());
    }

    #[test]
    fn test_count_with_filter() {
        let op = CountOperation::<MockEngine, TestModel>::new(MockEngine)
            .where_(Filter::Equals("active".to_string(), FilterValue::Bool(true)));

        let (sql, params) = op.build_sql();

        assert!(sql.contains("WHERE"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_count_distinct() {
        let op = CountOperation::<MockEngine, TestModel>::new(MockEngine).distinct("email");

        let (sql, _) = op.build_sql();

        assert!(sql.contains("COUNT(DISTINCT email)"));
    }
}

