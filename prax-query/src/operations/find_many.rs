//! FindMany operation for querying multiple records.

use std::marker::PhantomData;

use crate::error::QueryResult;
use crate::filter::Filter;
use crate::pagination::Pagination;
use crate::traits::{Model, QueryEngine};
use crate::types::{OrderBy, Select};

/// A query operation that finds multiple records.
///
/// # Example
///
/// ```rust,ignore
/// let users = client
///     .user()
///     .find_many()
///     .where_(user::email::contains("@example.com"))
///     .order_by(user::created_at::desc())
///     .skip(0)
///     .take(10)
///     .exec()
///     .await?;
/// ```
pub struct FindManyOperation<E: QueryEngine, M: Model> {
    engine: E,
    filter: Filter,
    order_by: OrderBy,
    pagination: Pagination,
    select: Select,
    distinct: Option<Vec<String>>,
    _model: PhantomData<M>,
}

impl<E: QueryEngine, M: Model> FindManyOperation<E, M> {
    /// Create a new FindMany operation.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            filter: Filter::None,
            order_by: OrderBy::none(),
            pagination: Pagination::new(),
            select: Select::All,
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

    /// Set the order by clause.
    pub fn order_by(mut self, order: impl Into<OrderBy>) -> Self {
        self.order_by = order.into();
        self
    }

    /// Skip a number of records.
    pub fn skip(mut self, n: u64) -> Self {
        self.pagination = self.pagination.skip(n);
        self
    }

    /// Take a limited number of records.
    pub fn take(mut self, n: u64) -> Self {
        self.pagination = self.pagination.take(n);
        self
    }

    /// Select specific fields.
    pub fn select(mut self, select: impl Into<Select>) -> Self {
        self.select = select.into();
        self
    }

    /// Make the query distinct.
    pub fn distinct(mut self, columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.distinct = Some(columns.into_iter().map(Into::into).collect());
        self
    }

    /// Set cursor for cursor-based pagination.
    pub fn cursor(mut self, cursor: crate::pagination::Cursor) -> Self {
        self.pagination = self.pagination.cursor(cursor);
        self
    }

    /// Build the SQL query.
    pub fn build_sql(&self) -> (String, Vec<crate::filter::FilterValue>) {
        let (where_sql, params) = self.filter.to_sql(0);

        let mut sql = String::new();

        // SELECT clause
        sql.push_str("SELECT ");
        if let Some(ref cols) = self.distinct {
            sql.push_str("DISTINCT ON (");
            sql.push_str(&cols.join(", "));
            sql.push_str(") ");
        }
        sql.push_str(&self.select.to_sql());

        // FROM clause
        sql.push_str(" FROM ");
        sql.push_str(M::TABLE_NAME);

        // WHERE clause
        if !self.filter.is_none() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_sql);
        }

        // ORDER BY clause
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            sql.push_str(&self.order_by.to_sql());
        }

        // LIMIT/OFFSET clause
        let pagination_sql = self.pagination.to_sql();
        if !pagination_sql.is_empty() {
            sql.push(' ');
            sql.push_str(&pagination_sql);
        }

        (sql, params)
    }

    /// Execute the query.
    pub async fn exec(self) -> QueryResult<Vec<M>>
    where
        M: Send + 'static,
    {
        let (sql, params) = self.build_sql();
        self.engine.query_many::<M>(&sql, params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::FilterValue;
    use crate::types::OrderByField;

    struct TestModel;

    impl Model for TestModel {
        const MODEL_NAME: &'static str = "TestModel";
        const TABLE_NAME: &'static str = "test_models";
        const PRIMARY_KEY: &'static [&'static str] = &["id"];
        const COLUMNS: &'static [&'static str] = &["id", "name", "email"];
    }

    // Mock query engine for testing SQL generation
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
            Box::pin(async { Err(crate::error::QueryError::not_found("test")) })
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
            Box::pin(async { Err(crate::error::QueryError::not_found("test")) })
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
    fn test_find_many_basic() {
        let op = FindManyOperation::<MockEngine, TestModel>::new(MockEngine);
        let (sql, params) = op.build_sql();

        assert_eq!(sql, "SELECT * FROM test_models");
        assert!(params.is_empty());
    }

    #[test]
    fn test_find_many_with_filter() {
        let op = FindManyOperation::<MockEngine, TestModel>::new(MockEngine)
            .where_(Filter::Equals("name".to_string(), "Alice".into()));

        let (sql, params) = op.build_sql();

        assert!(sql.contains("WHERE"));
        assert!(sql.contains("name = $1"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_find_many_with_order() {
        let op = FindManyOperation::<MockEngine, TestModel>::new(MockEngine)
            .order_by(OrderByField::desc("created_at"));

        let (sql, _) = op.build_sql();

        assert!(sql.contains("ORDER BY created_at DESC"));
    }

    #[test]
    fn test_find_many_with_pagination() {
        let op = FindManyOperation::<MockEngine, TestModel>::new(MockEngine)
            .skip(10)
            .take(20);

        let (sql, _) = op.build_sql();

        assert!(sql.contains("LIMIT 20"));
        assert!(sql.contains("OFFSET 10"));
    }

    #[test]
    fn test_find_many_with_select() {
        let op = FindManyOperation::<MockEngine, TestModel>::new(MockEngine)
            .select(Select::fields(["id", "name"]));

        let (sql, _) = op.build_sql();

        assert!(sql.contains("SELECT id, name FROM"));
    }
}

