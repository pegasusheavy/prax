//! Delete operation for removing records.

use std::marker::PhantomData;

use crate::error::QueryResult;
use crate::filter::{Filter, FilterValue};
use crate::traits::{Model, QueryEngine};
use crate::types::Select;

/// A delete operation for removing records.
///
/// # Example
///
/// ```rust,ignore
/// let deleted = client
///     .user()
///     .delete()
///     .where_(user::id::equals(1))
///     .exec()
///     .await?;
/// ```
pub struct DeleteOperation<E: QueryEngine, M: Model> {
    engine: E,
    filter: Filter,
    select: Select,
    _model: PhantomData<M>,
}

impl<E: QueryEngine, M: Model> DeleteOperation<E, M> {
    /// Create a new Delete operation.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            filter: Filter::None,
            select: Select::All,
            _model: PhantomData,
        }
    }

    /// Add a filter condition.
    pub fn where_(mut self, filter: impl Into<Filter>) -> Self {
        let new_filter = filter.into();
        self.filter = self.filter.and_then(new_filter);
        self
    }

    /// Select specific fields to return from deleted records.
    pub fn select(mut self, select: impl Into<Select>) -> Self {
        self.select = select.into();
        self
    }

    /// Build the SQL query.
    pub fn build_sql(&self) -> (String, Vec<FilterValue>) {
        let (where_sql, params) = self.filter.to_sql(0);

        let mut sql = String::new();

        // DELETE FROM clause
        sql.push_str("DELETE FROM ");
        sql.push_str(M::TABLE_NAME);

        // WHERE clause
        if !self.filter.is_none() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_sql);
        }

        // RETURNING clause
        sql.push_str(" RETURNING ");
        sql.push_str(&self.select.to_sql());

        (sql, params)
    }

    /// Build SQL without RETURNING (for count).
    fn build_sql_count(&self) -> (String, Vec<FilterValue>) {
        let (where_sql, params) = self.filter.to_sql(0);

        let mut sql = String::new();

        sql.push_str("DELETE FROM ");
        sql.push_str(M::TABLE_NAME);

        if !self.filter.is_none() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_sql);
        }

        (sql, params)
    }

    /// Execute the delete and return deleted records.
    pub async fn exec(self) -> QueryResult<Vec<M>>
    where
        M: Send + 'static,
    {
        let (sql, params) = self.build_sql();
        self.engine.execute_update::<M>(&sql, params).await
    }

    /// Execute the delete and return the count of deleted records.
    pub async fn exec_count(self) -> QueryResult<u64> {
        let (sql, params) = self.build_sql_count();
        self.engine.execute_delete(&sql, params).await
    }
}

/// Delete many records at once.
pub struct DeleteManyOperation<E: QueryEngine, M: Model> {
    engine: E,
    filter: Filter,
    _model: PhantomData<M>,
}

impl<E: QueryEngine, M: Model> DeleteManyOperation<E, M> {
    /// Create a new DeleteMany operation.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            filter: Filter::None,
            _model: PhantomData,
        }
    }

    /// Add a filter condition.
    pub fn where_(mut self, filter: impl Into<Filter>) -> Self {
        let new_filter = filter.into();
        self.filter = self.filter.and_then(new_filter);
        self
    }

    /// Build the SQL query.
    pub fn build_sql(&self) -> (String, Vec<FilterValue>) {
        let (where_sql, params) = self.filter.to_sql(0);

        let mut sql = String::new();

        sql.push_str("DELETE FROM ");
        sql.push_str(M::TABLE_NAME);

        if !self.filter.is_none() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_sql);
        }

        (sql, params)
    }

    /// Execute the delete and return the count of deleted records.
    pub async fn exec(self) -> QueryResult<u64> {
        let (sql, params) = self.build_sql();
        self.engine.execute_delete(&sql, params).await
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
    fn test_delete_with_filter() {
        let op = DeleteOperation::<MockEngine, TestModel>::new(MockEngine)
            .where_(Filter::Equals("id".to_string(), FilterValue::Int(1)));

        let (sql, params) = op.build_sql();

        assert!(sql.contains("DELETE FROM test_models"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("RETURNING *"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_delete_many() {
        let op = DeleteManyOperation::<MockEngine, TestModel>::new(MockEngine)
            .where_(Filter::In(
                "id".to_string(),
                vec![FilterValue::Int(1), FilterValue::Int(2)],
            ));

        let (sql, params) = op.build_sql();

        assert!(sql.contains("DELETE FROM test_models"));
        assert!(sql.contains("IN"));
        assert!(!sql.contains("RETURNING"));
        assert_eq!(params.len(), 2);
    }
}

