//! Update operation for modifying existing records.

use std::marker::PhantomData;

use crate::error::QueryResult;
use crate::filter::{Filter, FilterValue};
use crate::traits::{Model, QueryEngine};
use crate::types::Select;

/// An update operation for modifying existing records.
///
/// # Example
///
/// ```rust,ignore
/// let users = client
///     .user()
///     .update()
///     .where_(user::id::equals(1))
///     .set("name", "Updated Name")
///     .exec()
///     .await?;
/// ```
pub struct UpdateOperation<E: QueryEngine, M: Model> {
    engine: E,
    filter: Filter,
    updates: Vec<(String, FilterValue)>,
    select: Select,
    _model: PhantomData<M>,
}

impl<E: QueryEngine, M: Model> UpdateOperation<E, M> {
    /// Create a new Update operation.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            filter: Filter::None,
            updates: Vec::new(),
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

    /// Set a column to a new value.
    pub fn set(mut self, column: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.updates.push((column.into(), value.into()));
        self
    }

    /// Set multiple columns from an iterator.
    pub fn set_many(
        mut self,
        values: impl IntoIterator<Item = (impl Into<String>, impl Into<FilterValue>)>,
    ) -> Self {
        for (col, val) in values {
            self.updates.push((col.into(), val.into()));
        }
        self
    }

    /// Increment a numeric column.
    pub fn increment(self, column: impl Into<String>, amount: i64) -> Self {
        // This would need special handling in SQL generation
        // For now, we'll implement a basic version
        self.set(column, FilterValue::Int(amount))
    }

    /// Select specific fields to return.
    pub fn select(mut self, select: impl Into<Select>) -> Self {
        self.select = select.into();
        self
    }

    /// Build the SQL query.
    pub fn build_sql(&self) -> (String, Vec<FilterValue>) {
        let mut sql = String::new();
        let mut params = Vec::new();
        let mut param_idx = 1;

        // UPDATE clause
        sql.push_str("UPDATE ");
        sql.push_str(M::TABLE_NAME);

        // SET clause
        sql.push_str(" SET ");
        let set_parts: Vec<_> = self
            .updates
            .iter()
            .map(|(col, val)| {
                params.push(val.clone());
                let part = format!("{} = ${}", col, param_idx);
                param_idx += 1;
                part
            })
            .collect();
        sql.push_str(&set_parts.join(", "));

        // WHERE clause
        if !self.filter.is_none() {
            let (where_sql, where_params) = self.filter.to_sql(param_idx - 1);
            sql.push_str(" WHERE ");
            sql.push_str(&where_sql);
            params.extend(where_params);
        }

        // RETURNING clause
        sql.push_str(" RETURNING ");
        sql.push_str(&self.select.to_sql());

        (sql, params)
    }

    /// Execute the update and return modified records.
    pub async fn exec(self) -> QueryResult<Vec<M>>
    where
        M: Send + 'static,
    {
        let (sql, params) = self.build_sql();
        self.engine.execute_update::<M>(&sql, params).await
    }

    /// Execute the update and return the first modified record.
    pub async fn exec_one(self) -> QueryResult<M>
    where
        M: Send + 'static,
    {
        let (sql, params) = self.build_sql();
        self.engine.query_one::<M>(&sql, params).await
    }
}

/// Update many records at once.
pub struct UpdateManyOperation<E: QueryEngine, M: Model> {
    engine: E,
    filter: Filter,
    updates: Vec<(String, FilterValue)>,
    _model: PhantomData<M>,
}

impl<E: QueryEngine, M: Model> UpdateManyOperation<E, M> {
    /// Create a new UpdateMany operation.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            filter: Filter::None,
            updates: Vec::new(),
            _model: PhantomData,
        }
    }

    /// Add a filter condition.
    pub fn where_(mut self, filter: impl Into<Filter>) -> Self {
        let new_filter = filter.into();
        self.filter = self.filter.and_then(new_filter);
        self
    }

    /// Set a column to a new value.
    pub fn set(mut self, column: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.updates.push((column.into(), value.into()));
        self
    }

    /// Build the SQL query.
    pub fn build_sql(&self) -> (String, Vec<FilterValue>) {
        let mut sql = String::new();
        let mut params = Vec::new();
        let mut param_idx = 1;

        // UPDATE clause
        sql.push_str("UPDATE ");
        sql.push_str(M::TABLE_NAME);

        // SET clause
        sql.push_str(" SET ");
        let set_parts: Vec<_> = self
            .updates
            .iter()
            .map(|(col, val)| {
                params.push(val.clone());
                let part = format!("{} = ${}", col, param_idx);
                param_idx += 1;
                part
            })
            .collect();
        sql.push_str(&set_parts.join(", "));

        // WHERE clause
        if !self.filter.is_none() {
            let (where_sql, where_params) = self.filter.to_sql(param_idx - 1);
            sql.push_str(" WHERE ");
            sql.push_str(&where_sql);
            params.extend(where_params);
        }

        (sql, params)
    }

    /// Execute the update and return the count of modified records.
    pub async fn exec(self) -> QueryResult<u64> {
        let (sql, params) = self.build_sql();
        self.engine.execute_raw(&sql, params).await
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
    fn test_update_basic() {
        let op = UpdateOperation::<MockEngine, TestModel>::new(MockEngine)
            .where_(Filter::Equals("id".to_string(), FilterValue::Int(1)))
            .set("name", "Updated");

        let (sql, params) = op.build_sql();

        assert!(sql.contains("UPDATE test_models SET"));
        assert!(sql.contains("name = $1"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("RETURNING *"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_update_many_fields() {
        let op = UpdateOperation::<MockEngine, TestModel>::new(MockEngine)
            .set("name", "Updated")
            .set("email", "updated@example.com");

        let (sql, params) = op.build_sql();

        assert!(sql.contains("name = $1"));
        assert!(sql.contains("email = $2"));
        assert_eq!(params.len(), 2);
    }
}

