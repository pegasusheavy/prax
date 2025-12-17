//! Create operation for inserting new records.

use std::marker::PhantomData;

use crate::error::QueryResult;
use crate::filter::FilterValue;
use crate::traits::{Model, QueryEngine};
use crate::types::Select;

/// A create operation for inserting a new record.
///
/// # Example
///
/// ```rust,ignore
/// let user = client
///     .user()
///     .create(user::Create {
///         email: "new@example.com".into(),
///         name: Some("New User".into()),
///     })
///     .exec()
///     .await?;
/// ```
pub struct CreateOperation<E: QueryEngine, M: Model> {
    engine: E,
    columns: Vec<String>,
    values: Vec<FilterValue>,
    select: Select,
    _model: PhantomData<M>,
}

impl<E: QueryEngine, M: Model> CreateOperation<E, M> {
    /// Create a new Create operation.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            columns: Vec::new(),
            values: Vec::new(),
            select: Select::All,
            _model: PhantomData,
        }
    }

    /// Set a column value.
    pub fn set(mut self, column: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.columns.push(column.into());
        self.values.push(value.into());
        self
    }

    /// Set multiple column values from an iterator.
    pub fn set_many(
        mut self,
        values: impl IntoIterator<Item = (impl Into<String>, impl Into<FilterValue>)>,
    ) -> Self {
        for (col, val) in values {
            self.columns.push(col.into());
            self.values.push(val.into());
        }
        self
    }

    /// Select specific fields to return.
    pub fn select(mut self, select: impl Into<Select>) -> Self {
        self.select = select.into();
        self
    }

    /// Build the SQL query.
    pub fn build_sql(&self) -> (String, Vec<FilterValue>) {
        let mut sql = String::new();

        // INSERT INTO clause
        sql.push_str("INSERT INTO ");
        sql.push_str(M::TABLE_NAME);

        // Columns
        sql.push_str(" (");
        sql.push_str(&self.columns.join(", "));
        sql.push(')');

        // VALUES
        sql.push_str(" VALUES (");
        let placeholders: Vec<_> = (1..=self.values.len())
            .map(|i| format!("${}", i))
            .collect();
        sql.push_str(&placeholders.join(", "));
        sql.push(')');

        // RETURNING clause
        sql.push_str(" RETURNING ");
        sql.push_str(&self.select.to_sql());

        (sql, self.values.clone())
    }

    /// Execute the create operation and return the created record.
    pub async fn exec(self) -> QueryResult<M>
    where
        M: Send + 'static,
    {
        let (sql, params) = self.build_sql();
        self.engine.execute_insert::<M>(&sql, params).await
    }
}

/// Create many records at once.
pub struct CreateManyOperation<E: QueryEngine, M: Model> {
    engine: E,
    columns: Vec<String>,
    rows: Vec<Vec<FilterValue>>,
    skip_duplicates: bool,
    _model: PhantomData<M>,
}

impl<E: QueryEngine, M: Model> CreateManyOperation<E, M> {
    /// Create a new CreateMany operation.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            columns: Vec::new(),
            rows: Vec::new(),
            skip_duplicates: false,
            _model: PhantomData,
        }
    }

    /// Set the columns for insertion.
    pub fn columns(mut self, columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.columns = columns.into_iter().map(Into::into).collect();
        self
    }

    /// Add a row of values.
    pub fn row(mut self, values: impl IntoIterator<Item = impl Into<FilterValue>>) -> Self {
        self.rows.push(values.into_iter().map(Into::into).collect());
        self
    }

    /// Add multiple rows.
    pub fn rows(
        mut self,
        rows: impl IntoIterator<Item = impl IntoIterator<Item = impl Into<FilterValue>>>,
    ) -> Self {
        for row in rows {
            self.rows.push(row.into_iter().map(Into::into).collect());
        }
        self
    }

    /// Skip records that violate unique constraints.
    pub fn skip_duplicates(mut self) -> Self {
        self.skip_duplicates = true;
        self
    }

    /// Build the SQL query.
    pub fn build_sql(&self) -> (String, Vec<FilterValue>) {
        let mut sql = String::new();
        let mut all_params = Vec::new();

        // INSERT INTO clause
        sql.push_str("INSERT INTO ");
        sql.push_str(M::TABLE_NAME);

        // Columns
        sql.push_str(" (");
        sql.push_str(&self.columns.join(", "));
        sql.push(')');

        // VALUES
        sql.push_str(" VALUES ");

        let mut value_groups = Vec::new();
        let mut param_idx = 1;

        for row in &self.rows {
            let placeholders: Vec<_> = row
                .iter()
                .map(|v| {
                    all_params.push(v.clone());
                    let placeholder = format!("${}", param_idx);
                    param_idx += 1;
                    placeholder
                })
                .collect();
            value_groups.push(format!("({})", placeholders.join(", ")));
        }

        sql.push_str(&value_groups.join(", "));

        // ON CONFLICT for skip_duplicates
        if self.skip_duplicates {
            sql.push_str(" ON CONFLICT DO NOTHING");
        }

        (sql, all_params)
    }

    /// Execute the create operation and return the number of created records.
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
    fn test_create_basic() {
        let op = CreateOperation::<MockEngine, TestModel>::new(MockEngine)
            .set("name", "Alice")
            .set("email", "alice@example.com");

        let (sql, params) = op.build_sql();

        assert!(sql.contains("INSERT INTO test_models"));
        assert!(sql.contains("(name, email)"));
        assert!(sql.contains("VALUES ($1, $2)"));
        assert!(sql.contains("RETURNING *"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_create_many() {
        let op = CreateManyOperation::<MockEngine, TestModel>::new(MockEngine)
            .columns(["name", "email"])
            .row(["Alice", "alice@example.com"])
            .row(["Bob", "bob@example.com"]);

        let (sql, params) = op.build_sql();

        assert!(sql.contains("INSERT INTO test_models"));
        assert!(sql.contains("VALUES ($1, $2), ($3, $4)"));
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn test_create_many_skip_duplicates() {
        let op = CreateManyOperation::<MockEngine, TestModel>::new(MockEngine)
            .columns(["name", "email"])
            .row(["Alice", "alice@example.com"])
            .skip_duplicates();

        let (sql, _) = op.build_sql();

        assert!(sql.contains("ON CONFLICT DO NOTHING"));
    }
}

