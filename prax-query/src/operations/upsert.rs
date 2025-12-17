//! Upsert operation for creating or updating records.

use std::marker::PhantomData;

use crate::error::QueryResult;
use crate::filter::{Filter, FilterValue};
use crate::traits::{Model, QueryEngine};
use crate::types::Select;

/// An upsert (insert or update) operation.
///
/// # Example
///
/// ```rust,ignore
/// let user = client
///     .user()
///     .upsert()
///     .where_(user::email::equals("test@example.com"))
///     .create(user::Create { email: "test@example.com".into(), name: Some("Test".into()) })
///     .update(user::Update { name: Some("Updated".into()), ..Default::default() })
///     .exec()
///     .await?;
/// ```
pub struct UpsertOperation<E: QueryEngine, M: Model> {
    engine: E,
    filter: Filter,
    create_columns: Vec<String>,
    create_values: Vec<FilterValue>,
    update_columns: Vec<String>,
    update_values: Vec<FilterValue>,
    conflict_columns: Vec<String>,
    select: Select,
    _model: PhantomData<M>,
}

impl<E: QueryEngine, M: Model> UpsertOperation<E, M> {
    /// Create a new Upsert operation.
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            filter: Filter::None,
            create_columns: Vec::new(),
            create_values: Vec::new(),
            update_columns: Vec::new(),
            update_values: Vec::new(),
            conflict_columns: Vec::new(),
            select: Select::All,
            _model: PhantomData,
        }
    }

    /// Add a filter condition (identifies the record to upsert).
    pub fn where_(mut self, filter: impl Into<Filter>) -> Self {
        self.filter = filter.into();
        self
    }

    /// Set the columns to check for conflict.
    pub fn on_conflict(mut self, columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.conflict_columns = columns.into_iter().map(Into::into).collect();
        self
    }

    /// Set the create data.
    pub fn create(
        mut self,
        values: impl IntoIterator<Item = (impl Into<String>, impl Into<FilterValue>)>,
    ) -> Self {
        for (col, val) in values {
            self.create_columns.push(col.into());
            self.create_values.push(val.into());
        }
        self
    }

    /// Set a single create column.
    pub fn create_set(
        mut self,
        column: impl Into<String>,
        value: impl Into<FilterValue>,
    ) -> Self {
        self.create_columns.push(column.into());
        self.create_values.push(value.into());
        self
    }

    /// Set the update data.
    pub fn update(
        mut self,
        values: impl IntoIterator<Item = (impl Into<String>, impl Into<FilterValue>)>,
    ) -> Self {
        for (col, val) in values {
            self.update_columns.push(col.into());
            self.update_values.push(val.into());
        }
        self
    }

    /// Set a single update column.
    pub fn update_set(
        mut self,
        column: impl Into<String>,
        value: impl Into<FilterValue>,
    ) -> Self {
        self.update_columns.push(column.into());
        self.update_values.push(value.into());
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
        let mut params = Vec::new();
        let mut param_idx = 1;

        // INSERT INTO clause
        sql.push_str("INSERT INTO ");
        sql.push_str(M::TABLE_NAME);

        // Columns
        sql.push_str(" (");
        sql.push_str(&self.create_columns.join(", "));
        sql.push(')');

        // VALUES
        sql.push_str(" VALUES (");
        let placeholders: Vec<_> = self
            .create_values
            .iter()
            .map(|v| {
                params.push(v.clone());
                let p = format!("${}", param_idx);
                param_idx += 1;
                p
            })
            .collect();
        sql.push_str(&placeholders.join(", "));
        sql.push(')');

        // ON CONFLICT
        sql.push_str(" ON CONFLICT ");
        if !self.conflict_columns.is_empty() {
            sql.push('(');
            sql.push_str(&self.conflict_columns.join(", "));
            sql.push_str(") ");
        }

        // DO UPDATE SET
        if self.update_columns.is_empty() {
            sql.push_str("DO NOTHING");
        } else {
            sql.push_str("DO UPDATE SET ");
            let update_parts: Vec<_> = self
                .update_columns
                .iter()
                .zip(self.update_values.iter())
                .map(|(col, val)| {
                    params.push(val.clone());
                    let part = format!("{} = ${}", col, param_idx);
                    param_idx += 1;
                    part
                })
                .collect();
            sql.push_str(&update_parts.join(", "));
        }

        // RETURNING clause
        sql.push_str(" RETURNING ");
        sql.push_str(&self.select.to_sql());

        (sql, params)
    }

    /// Execute the upsert and return the record.
    pub async fn exec(self) -> QueryResult<M>
    where
        M: Send + 'static,
    {
        let (sql, params) = self.build_sql();
        self.engine.execute_insert::<M>(&sql, params).await
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
    fn test_upsert_basic() {
        let op = UpsertOperation::<MockEngine, TestModel>::new(MockEngine)
            .on_conflict(["email"])
            .create_set("email", "test@example.com")
            .create_set("name", "Test")
            .update_set("name", "Updated");

        let (sql, params) = op.build_sql();

        assert!(sql.contains("INSERT INTO test_models"));
        assert!(sql.contains("ON CONFLICT (email)"));
        assert!(sql.contains("DO UPDATE SET"));
        assert!(sql.contains("RETURNING *"));
        assert_eq!(params.len(), 3); // 2 create + 1 update
    }

    #[test]
    fn test_upsert_do_nothing() {
        let op = UpsertOperation::<MockEngine, TestModel>::new(MockEngine)
            .on_conflict(["email"])
            .create_set("email", "test@example.com");

        let (sql, _) = op.build_sql();

        assert!(sql.contains("DO NOTHING"));
    }
}

