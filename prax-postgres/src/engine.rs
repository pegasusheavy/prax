//! PostgreSQL query engine implementation.

use std::marker::PhantomData;

use prax_query::filter::FilterValue;
use prax_query::traits::{BoxFuture, Model, QueryEngine};
use prax_query::QueryResult;
use tracing::debug;

use crate::pool::PgPool;
use crate::types::filter_value_to_sql;

/// PostgreSQL query engine that implements the Prax QueryEngine trait.
#[derive(Clone)]
pub struct PgEngine {
    pool: PgPool,
}

impl PgEngine {
    /// Create a new PostgreSQL engine with the given connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Convert filter values to PostgreSQL parameters.
    fn to_params(
        values: &[FilterValue],
    ) -> Result<Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>>, prax_query::QueryError>
    {
        values
            .iter()
            .map(|v| filter_value_to_sql(v).map_err(|e| prax_query::QueryError::database(e.to_string())))
            .collect()
    }
}

impl QueryEngine for PgEngine {
    fn query_many<T: Model + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<Vec<T>>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(sql = %sql, "Executing query_many");

            let conn = self.pool.get().await.map_err(|e| {
                prax_query::QueryError::connection(e.to_string())
            })?;

            let pg_params = Self::to_params(&params)?;
            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                pg_params.iter().map(|p| p.as_ref() as _).collect();

            let rows = conn.query(&sql, &param_refs).await.map_err(|e| {
                prax_query::QueryError::database(e.to_string())
            })?;

            // For now, we'll return an empty vec since we need FromPgRow implementation
            // In practice, this would deserialize rows into T
            // This is a placeholder - real implementation would use FromPgRow
            let _ = rows;
            Ok(Vec::new())
        })
    }

    fn query_one<T: Model + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<T>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(sql = %sql, "Executing query_one");

            let conn = self.pool.get().await.map_err(|e| {
                prax_query::QueryError::connection(e.to_string())
            })?;

            let pg_params = Self::to_params(&params)?;
            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                pg_params.iter().map(|p| p.as_ref() as _).collect();

            let _row = conn.query_one(&sql, &param_refs).await.map_err(|e| {
                if e.to_string().contains("no rows") {
                    prax_query::QueryError::not_found(T::MODEL_NAME)
                } else {
                    prax_query::QueryError::database(e.to_string())
                }
            })?;

            // Placeholder - would deserialize row into T
            Err(prax_query::QueryError::internal(
                "deserialization not yet implemented".to_string(),
            ))
        })
    }

    fn query_optional<T: Model + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<Option<T>>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(sql = %sql, "Executing query_optional");

            let conn = self.pool.get().await.map_err(|e| {
                prax_query::QueryError::connection(e.to_string())
            })?;

            let pg_params = Self::to_params(&params)?;
            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                pg_params.iter().map(|p| p.as_ref() as _).collect();

            let row = conn.query_opt(&sql, &param_refs).await.map_err(|e| {
                prax_query::QueryError::database(e.to_string())
            })?;

            match row {
                Some(_row) => {
                    // Placeholder - would deserialize row into T
                    Err(prax_query::QueryError::internal(
                        "deserialization not yet implemented".to_string(),
                    ))
                }
                None => Ok(None),
            }
        })
    }

    fn execute_insert<T: Model + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<T>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(sql = %sql, "Executing insert");

            let conn = self.pool.get().await.map_err(|e| {
                prax_query::QueryError::connection(e.to_string())
            })?;

            let pg_params = Self::to_params(&params)?;
            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                pg_params.iter().map(|p| p.as_ref() as _).collect();

            let _row = conn.query_one(&sql, &param_refs).await.map_err(|e| {
                prax_query::QueryError::database(e.to_string())
            })?;

            // Placeholder - would deserialize row into T
            Err(prax_query::QueryError::internal(
                "deserialization not yet implemented".to_string(),
            ))
        })
    }

    fn execute_update<T: Model + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<Vec<T>>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(sql = %sql, "Executing update");

            let conn = self.pool.get().await.map_err(|e| {
                prax_query::QueryError::connection(e.to_string())
            })?;

            let pg_params = Self::to_params(&params)?;
            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                pg_params.iter().map(|p| p.as_ref() as _).collect();

            let rows = conn.query(&sql, &param_refs).await.map_err(|e| {
                prax_query::QueryError::database(e.to_string())
            })?;

            // Placeholder - would deserialize rows into Vec<T>
            let _ = rows;
            Ok(Vec::new())
        })
    }

    fn execute_delete(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<u64>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(sql = %sql, "Executing delete");

            let conn = self.pool.get().await.map_err(|e| {
                prax_query::QueryError::connection(e.to_string())
            })?;

            let pg_params = Self::to_params(&params)?;
            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                pg_params.iter().map(|p| p.as_ref() as _).collect();

            let count = conn.execute(&sql, &param_refs).await.map_err(|e| {
                prax_query::QueryError::database(e.to_string())
            })?;

            Ok(count)
        })
    }

    fn execute_raw(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<u64>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(sql = %sql, "Executing raw SQL");

            let conn = self.pool.get().await.map_err(|e| {
                prax_query::QueryError::connection(e.to_string())
            })?;

            let pg_params = Self::to_params(&params)?;
            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                pg_params.iter().map(|p| p.as_ref() as _).collect();

            let count = conn.execute(&sql, &param_refs).await.map_err(|e| {
                prax_query::QueryError::database(e.to_string())
            })?;

            Ok(count)
        })
    }

    fn count(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<u64>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(sql = %sql, "Executing count");

            let conn = self.pool.get().await.map_err(|e| {
                prax_query::QueryError::connection(e.to_string())
            })?;

            let pg_params = Self::to_params(&params)?;
            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                pg_params.iter().map(|p| p.as_ref() as _).collect();

            let row = conn.query_one(&sql, &param_refs).await.map_err(|e| {
                prax_query::QueryError::database(e.to_string())
            })?;

            let count: i64 = row.get(0);
            Ok(count as u64)
        })
    }
}

/// A typed query builder that uses the PostgreSQL engine.
pub struct PgQueryBuilder<T: Model> {
    engine: PgEngine,
    _marker: PhantomData<T>,
}

impl<T: Model> PgQueryBuilder<T> {
    /// Create a new query builder.
    pub fn new(engine: PgEngine) -> Self {
        Self {
            engine,
            _marker: PhantomData,
        }
    }

    /// Get the underlying engine.
    pub fn engine(&self) -> &PgEngine {
        &self.engine
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require a real PostgreSQL database
}

