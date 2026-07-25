//! DuckDB query engine implementation.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::Value as JsonValue;
use tracing::{debug, instrument};

use prax_query::filter::FilterValue;
use prax_query::types::SortOrder;

use crate::error::{DuckDbError, DuckDbResult};
use crate::pool::{DuckDbPool, PooledConnection};

/// DuckDB query engine.
///
/// Two modes, controlled by the `tx_conn` field:
///
/// - **Pool mode** (`tx_conn == None`, the default): each query
///   acquires a fresh connection from [`DuckDbPool`] and returns it
///   after the call.
/// - **Transaction mode** (`tx_conn == Some(conn)`): every query
///   routes through the single pinned pooled connection the
///   transaction was opened on. The tx-bound engine is built by the
///   [`prax_query::traits::QueryEngine::transaction`] override, which
///   issues `BEGIN TRANSACTION` and then `COMMIT` / `ROLLBACK` on the
///   same connection based on the closure's `Ok` / `Err` result.
#[derive(Clone)]
pub struct DuckDbEngine {
    pool: DuckDbPool,
    /// Present when this engine is bound to an in-flight transaction:
    /// the pooled connection the transaction was opened on, pinned for
    /// its whole duration. `None` in the normal pool-checkout case.
    tx_conn: Option<Arc<PooledConnection>>,
    /// Present when this engine is bound to an in-flight transaction.
    /// Shared with the transaction finalizer, which sets the flag once
    /// COMMIT or ROLLBACK has been issued; engine clones the closure
    /// stashed check it before touching the pinned connection, so
    /// post-transaction queries fail loudly instead of silently running
    /// outside any transaction. `None` in pool mode.
    tx_finalized: Option<Arc<AtomicBool>>,
}

/// Connection routed for a single operation.
///
/// Either a borrow of the pinned transaction connection (transaction
/// mode) or an owned pool checkout (pool mode). Derefs to
/// [`PooledConnection`] so every call site uses the same method set
/// regardless of the source.
enum ConnectionSource<'a> {
    Tx(&'a PooledConnection),
    Pool(PooledConnection),
}

impl std::ops::Deref for ConnectionSource<'_> {
    type Target = PooledConnection;

    fn deref(&self) -> &PooledConnection {
        match self {
            Self::Tx(conn) => conn,
            Self::Pool(conn) => conn,
        }
    }
}

/// Error returned when a query is attempted through a transaction-bound
/// engine whose transaction has already been committed or rolled back
/// (e.g. an engine clone the transaction closure stashed for later use).
const TX_FINALIZED: &str = "transaction has already been committed or rolled back";

/// Result of a query operation.
#[derive(Debug, Clone)]
pub struct DuckDbQueryResult {
    /// The result data as JSON.
    pub data: JsonValue,
}

impl DuckDbQueryResult {
    /// Create a new query result.
    pub fn new(data: JsonValue) -> Self {
        Self { data }
    }

    /// Get the result as JSON.
    pub fn json(&self) -> &JsonValue {
        &self.data
    }

    /// Convert to the inner JSON value.
    pub fn into_json(self) -> JsonValue {
        self.data
    }
}

impl DuckDbEngine {
    /// Create a new DuckDB engine with the given pool.
    pub fn new(pool: DuckDbPool) -> Self {
        Self {
            pool,
            tx_conn: None,
            tx_finalized: None,
        }
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &DuckDbPool {
        &self.pool
    }

    /// Acquire the connection a single operation should run on.
    ///
    /// In transaction mode this borrows the pinned transaction
    /// connection, so every statement the tx-bound engine emits lands
    /// in the same `BEGIN`…`COMMIT` block; in pool mode it checks a
    /// fresh connection out of the pool. A tx-bound engine whose
    /// transaction has already been finalized (e.g. a clone the
    /// closure stashed past the end of `transaction`) is rejected.
    async fn connection(&self) -> DuckDbResult<ConnectionSource<'_>> {
        if let Some(tx) = &self.tx_conn {
            if self
                .tx_finalized
                .as_ref()
                .is_some_and(|f| f.load(Ordering::Acquire))
            {
                return Err(DuckDbError::internal(TX_FINALIZED));
            }
            Ok(ConnectionSource::Tx(tx.as_ref()))
        } else {
            self.pool.get().await.map(ConnectionSource::Pool)
        }
    }

    /// Build a SELECT query.
    fn build_select(
        &self,
        table: &str,
        columns: &[String],
        filters: &HashMap<String, FilterValue>,
        sort: &[(String, SortOrder)],
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> (String, Vec<FilterValue>) {
        let mut sql = String::new();
        let mut params: Vec<FilterValue> = Vec::new();

        // SELECT clause
        let cols = if columns.is_empty() {
            "*".to_string()
        } else {
            columns
                .iter()
                .map(|c| format!("\"{}\"", c))
                .collect::<Vec<_>>()
                .join(", ")
        };
        sql.push_str(&format!("SELECT {} FROM \"{}\"", cols, table));

        // WHERE clause
        if !filters.is_empty() {
            let mut conditions = Vec::new();
            for (field, value) in filters {
                match value {
                    FilterValue::Null => {
                        conditions.push(format!("\"{}\" IS NULL", field));
                    }
                    _ => {
                        conditions.push(format!("\"{}\" = ?", field));
                        params.push(value.clone());
                    }
                }
            }
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        // ORDER BY clause
        if !sort.is_empty() {
            let order_parts: Vec<String> = sort
                .iter()
                .map(|(col, dir)| {
                    let direction = match dir {
                        SortOrder::Asc => "ASC",
                        SortOrder::Desc => "DESC",
                    };
                    format!("\"{}\" {}", col, direction)
                })
                .collect();
            sql.push_str(" ORDER BY ");
            sql.push_str(&order_parts.join(", "));
        }

        // LIMIT and OFFSET
        if let Some(lim) = limit {
            sql.push_str(&format!(" LIMIT {}", lim));
        }
        if let Some(off) = offset {
            sql.push_str(&format!(" OFFSET {}", off));
        }

        (sql, params)
    }

    /// Build an INSERT query.
    fn build_insert(
        &self,
        table: &str,
        data: &HashMap<String, FilterValue>,
    ) -> (String, Vec<FilterValue>) {
        let mut columns = Vec::new();
        let mut placeholders = Vec::new();
        let mut params: Vec<FilterValue> = Vec::new();

        for (col, val) in data {
            columns.push(format!("\"{}\"", col));
            placeholders.push("?".to_string());
            params.push(val.clone());
        }

        let sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            table,
            columns.join(", "),
            placeholders.join(", ")
        );

        (sql, params)
    }

    /// Build an UPDATE query.
    fn build_update(
        &self,
        table: &str,
        data: &HashMap<String, FilterValue>,
        filters: &HashMap<String, FilterValue>,
    ) -> (String, Vec<FilterValue>) {
        let mut params: Vec<FilterValue> = Vec::new();

        // SET clause
        let set_parts: Vec<String> = data
            .iter()
            .map(|(col, val)| {
                params.push(val.clone());
                format!("\"{}\" = ?", col)
            })
            .collect();

        let mut sql = format!("UPDATE \"{}\" SET {}", table, set_parts.join(", "));

        // WHERE clause
        if !filters.is_empty() {
            let mut conditions = Vec::new();
            for (field, value) in filters {
                match value {
                    FilterValue::Null => {
                        conditions.push(format!("\"{}\" IS NULL", field));
                    }
                    _ => {
                        conditions.push(format!("\"{}\" = ?", field));
                        params.push(value.clone());
                    }
                }
            }
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        (sql, params)
    }

    /// Build a DELETE query.
    fn build_delete(
        &self,
        table: &str,
        filters: &HashMap<String, FilterValue>,
    ) -> (String, Vec<FilterValue>) {
        let mut sql = format!("DELETE FROM \"{}\"", table);
        let mut params: Vec<FilterValue> = Vec::new();

        if !filters.is_empty() {
            let mut conditions = Vec::new();
            for (field, value) in filters {
                match value {
                    FilterValue::Null => {
                        conditions.push(format!("\"{}\" IS NULL", field));
                    }
                    _ => {
                        conditions.push(format!("\"{}\" = ?", field));
                        params.push(value.clone());
                    }
                }
            }
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        (sql, params)
    }

    /// Execute a query and return multiple results.
    #[instrument(skip(self, columns, filters, sort), fields(table = %table))]
    pub async fn query_many(
        &self,
        table: &str,
        columns: &[String],
        filters: &HashMap<String, FilterValue>,
        sort: &[(String, SortOrder)],
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> DuckDbResult<Vec<DuckDbQueryResult>> {
        let (sql, params) = self.build_select(table, columns, filters, sort, limit, offset);
        debug!(sql = %sql, "Executing query_many");

        let conn = self.connection().await?;
        let results = conn.query(&sql, &params).await?;

        Ok(results.into_iter().map(DuckDbQueryResult::new).collect())
    }

    /// Execute a query and return a single result.
    #[instrument(skip(self, columns, filters), fields(table = %table))]
    pub async fn query_one(
        &self,
        table: &str,
        columns: &[String],
        filters: &HashMap<String, FilterValue>,
    ) -> DuckDbResult<DuckDbQueryResult> {
        let (sql, params) = self.build_select(table, columns, filters, &[], Some(1), None);
        debug!(sql = %sql, "Executing query_one");

        let conn = self.connection().await?;
        let result = conn.query_one(&sql, &params).await?;

        Ok(DuckDbQueryResult::new(result))
    }

    /// Execute a query and return an optional result.
    #[instrument(skip(self, columns, filters), fields(table = %table))]
    pub async fn query_optional(
        &self,
        table: &str,
        columns: &[String],
        filters: &HashMap<String, FilterValue>,
    ) -> DuckDbResult<Option<DuckDbQueryResult>> {
        let (sql, params) = self.build_select(table, columns, filters, &[], Some(1), None);
        debug!(sql = %sql, "Executing query_optional");

        let conn = self.connection().await?;
        let result = conn.query_optional(&sql, &params).await?;

        Ok(result.map(DuckDbQueryResult::new))
    }

    /// Execute an INSERT and return the persisted row.
    ///
    /// Uses DuckDB's `RETURNING *` support, so the result is the real
    /// database row — including server-generated values such as
    /// defaults and sequence-produced keys — not just the submitted
    /// input echoed back.
    #[instrument(skip(self, data), fields(table = %table))]
    pub async fn execute_insert(
        &self,
        table: &str,
        data: &HashMap<String, FilterValue>,
    ) -> DuckDbResult<DuckDbQueryResult> {
        let (sql, params) = self.build_insert(table, data);
        let sql = format!("{} RETURNING *", sql);
        debug!(sql = %sql, "Executing insert");

        let conn = self.connection().await?;
        let rows = conn.query(&sql, &params).await?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| DuckDbError::query("INSERT ... RETURNING produced no row"))?;

        Ok(DuckDbQueryResult::new(row))
    }

    /// Execute an UPDATE and return the number of affected rows.
    #[instrument(skip(self, data, filters), fields(table = %table))]
    pub async fn execute_update(
        &self,
        table: &str,
        data: &HashMap<String, FilterValue>,
        filters: &HashMap<String, FilterValue>,
    ) -> DuckDbResult<u64> {
        let (sql, params) = self.build_update(table, data, filters);
        debug!(sql = %sql, "Executing update");

        let conn = self.connection().await?;
        let affected = conn.execute(&sql, &params).await?;

        Ok(affected as u64)
    }

    /// Execute a DELETE and return the number of affected rows.
    #[instrument(skip(self, filters), fields(table = %table))]
    pub async fn execute_delete(
        &self,
        table: &str,
        filters: &HashMap<String, FilterValue>,
    ) -> DuckDbResult<u64> {
        let (sql, params) = self.build_delete(table, filters);
        debug!(sql = %sql, "Executing delete");

        let conn = self.connection().await?;
        let affected = conn.execute(&sql, &params).await?;

        Ok(affected as u64)
    }

    /// Execute raw SQL and return results.
    #[instrument(skip(self, params), fields(sql = %sql))]
    pub async fn execute_raw(
        &self,
        sql: &str,
        params: &[FilterValue],
    ) -> DuckDbResult<Vec<DuckDbQueryResult>> {
        debug!("Executing raw SQL");

        let conn = self.connection().await?;
        let results = conn.query(sql, params).await?;

        Ok(results.into_iter().map(DuckDbQueryResult::new).collect())
    }

    /// Execute a raw SQL statement and return the number of affected rows.
    #[instrument(skip(self, params), fields(sql = %sql))]
    pub async fn raw_sql_execute(&self, sql: &str, params: &[FilterValue]) -> DuckDbResult<u64> {
        debug!("Executing raw SQL statement");

        let conn = self.connection().await?;
        let affected = conn.execute(sql, params).await?;

        Ok(affected as u64)
    }

    /// Execute a raw SQL query using the Sql builder.
    #[instrument(skip(self, sql))]
    pub async fn raw_sql(&self, sql: prax_query::raw::Sql) -> DuckDbResult<Vec<DuckDbQueryResult>> {
        let (query_string, params) = sql.build();
        debug!(sql = %query_string, "Executing raw SQL from builder");
        self.execute_raw(&query_string, &params).await
    }

    /// Execute a raw SQL query and return the first result.
    #[instrument(skip(self, params), fields(sql = %sql))]
    pub async fn raw_sql_first(
        &self,
        sql: &str,
        params: &[FilterValue],
    ) -> DuckDbResult<DuckDbQueryResult> {
        let conn = self.connection().await?;
        let result = conn.query_one(sql, params).await?;
        Ok(DuckDbQueryResult::new(result))
    }

    /// Execute a raw SQL query and return the first result or None.
    #[instrument(skip(self, params), fields(sql = %sql))]
    pub async fn raw_sql_optional(
        &self,
        sql: &str,
        params: &[FilterValue],
    ) -> DuckDbResult<Option<DuckDbQueryResult>> {
        let conn = self.connection().await?;
        let result = conn.query_optional(sql, params).await?;
        Ok(result.map(DuckDbQueryResult::new))
    }

    /// Execute a raw SQL query and return a single scalar value.
    #[instrument(skip(self, params), fields(sql = %sql))]
    pub async fn raw_sql_scalar<T>(&self, sql: &str, params: &[FilterValue]) -> DuckDbResult<T>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        let conn = self.connection().await?;
        let result = conn.query_one(sql, params).await?;

        let value = result
            .as_object()
            .and_then(|obj| obj.values().next())
            .ok_or_else(|| DuckDbError::query("raw_sql_scalar returned empty row"))?;

        serde_json::from_value(value.clone()).map_err(|e| {
            DuckDbError::deserialization(format!("failed to deserialize scalar: {}", e))
        })
    }

    /// Execute multiple raw SQL statements in a batch.
    #[instrument(skip(self), fields(sql_len = %sql.len()))]
    pub async fn raw_sql_batch(&self, sql: &str) -> DuckDbResult<()> {
        debug!("Executing raw SQL batch");

        let conn = self.connection().await?;
        conn.execute_batch(sql).await
    }

    /// Count rows matching the filter.
    #[instrument(skip(self, filters), fields(table = %table))]
    pub async fn count(
        &self,
        table: &str,
        filters: &HashMap<String, FilterValue>,
    ) -> DuckDbResult<u64> {
        let mut sql = format!("SELECT COUNT(*) as count FROM \"{}\"", table);
        let mut params: Vec<FilterValue> = Vec::new();

        if !filters.is_empty() {
            let mut conditions = Vec::new();
            for (field, value) in filters {
                match value {
                    FilterValue::Null => {
                        conditions.push(format!("\"{}\" IS NULL", field));
                    }
                    _ => {
                        conditions.push(format!("\"{}\" = ?", field));
                        params.push(value.clone());
                    }
                }
            }
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        debug!(sql = %sql, "Executing count");

        let conn = self.connection().await?;
        let results = conn.query(&sql, &params).await?;

        // A genuine zero count is `0`; a missing row, missing `count`
        // key, or non-integer cell means the result shape drifted from
        // what we emitted — fail loudly instead of collapsing to 0.
        let count = results
            .first()
            .and_then(|row| row.get("count"))
            .and_then(|v| v.as_i64())
            .ok_or_else(|| {
                DuckDbError::deserialization(
                    "count query returned no integer 'count' cell".to_string(),
                )
            })?;

        Ok(count as u64)
    }

    // =========================================================================
    // DuckDB-specific analytical operations
    // =========================================================================

    /// Copy query results to a Parquet file.
    #[instrument(skip(self), fields(query_len = %query.len()))]
    pub async fn copy_to_parquet(&self, query: &str, path: &str) -> DuckDbResult<()> {
        let conn = self.connection().await?;
        conn.copy_to_parquet(query, path).await
    }

    /// Copy query results to a CSV file.
    #[instrument(skip(self), fields(query_len = %query.len()))]
    pub async fn copy_to_csv(&self, query: &str, path: &str, header: bool) -> DuckDbResult<()> {
        let conn = self.connection().await?;
        conn.copy_to_csv(query, path, header).await
    }

    /// Query a Parquet file.
    pub async fn query_parquet(&self, path: &str) -> DuckDbResult<Vec<DuckDbQueryResult>> {
        let conn = self.connection().await?;
        let results = conn.query_parquet(path).await?;
        Ok(results.into_iter().map(DuckDbQueryResult::new).collect())
    }

    /// Query a CSV file.
    pub async fn query_csv(
        &self,
        path: &str,
        header: bool,
    ) -> DuckDbResult<Vec<DuckDbQueryResult>> {
        let conn = self.connection().await?;
        let results = conn.query_csv(path, header).await?;
        Ok(results.into_iter().map(DuckDbQueryResult::new).collect())
    }

    /// Query a JSON file.
    pub async fn query_json(&self, path: &str) -> DuckDbResult<Vec<DuckDbQueryResult>> {
        let conn = self.connection().await?;
        let results = conn.query_json(path).await?;
        Ok(results.into_iter().map(DuckDbQueryResult::new).collect())
    }

    /// Get DuckDB version.
    pub async fn version(&self) -> DuckDbResult<String> {
        let result = self.raw_sql_first("SELECT version()", &[]).await?;
        result
            .data
            .as_object()
            .and_then(|obj| obj.values().next())
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| DuckDbError::query("Failed to get version"))
    }

    /// Explain a query plan.
    pub async fn explain(&self, query: &str) -> DuckDbResult<String> {
        let sql = format!("EXPLAIN {}", query);
        let results = self.execute_raw(&sql, &[]).await?;

        let mut plan = String::new();
        for result in results {
            if let Some(obj) = result.data.as_object() {
                for value in obj.values() {
                    if let Some(s) = value.as_str() {
                        plan.push_str(s);
                        plan.push('\n');
                    }
                }
            }
        }

        Ok(plan)
    }
}

// -----------------------------------------------------------------------------
// QueryEngine impl
// -----------------------------------------------------------------------------
//
// DuckDB's SQL surface is Postgres-compatible — same placeholder syntax,
// `RETURNING`, `ON CONFLICT (...) DO UPDATE`, and identifier quoting — so
// the Postgres dialect builder emits valid DuckDB statements. Aggregate
// queries run through the engine's JSON row path with per-cell probing,
// and transactions pin a single pooled connection for the closure's
// duration (see `tx_conn` on [`DuckDbEngine`]).

/// Decode one aggregate-result cell from its JSON representation into a
/// [`FilterValue`], probing integer, then float, then bool/text.
///
/// Aggregate result sets have no fixed schema: `COUNT` comes back as a
/// JSON integer, `AVG` as a JSON float, and DuckDB's `SUM`/decimal
/// aggregates surface through the JSON path as strings (mirroring the
/// Postgres engine's NUMERIC-as-text handling, which the aggregate
/// result folder parses back into numbers). `MIN`/`MAX` keep whatever
/// type the source column had.
fn json_cell_to_filter_value(value: &JsonValue) -> FilterValue {
    match value {
        JsonValue::Null => FilterValue::Null,
        JsonValue::Bool(b) => FilterValue::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                FilterValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                FilterValue::Float(f)
            } else {
                FilterValue::Null
            }
        }
        JsonValue::String(s) => FilterValue::String(s.clone()),
        other => FilterValue::Json(other.clone()),
    }
}

impl DuckDbEngine {
    /// Shared row-fetch path used by every QueryEngine method that
    /// returns typed models. Factored out of the trait impl so the
    /// trait-method bodies don't collide with DuckDbEngine's
    /// inherent `query_many` (which takes a different signature and
    /// returns untyped JSON).
    async fn fetch_typed<T: prax_query::traits::Model + prax_query::row::FromRow>(
        &self,
        sql: &str,
        params: &[FilterValue],
    ) -> prax_query::QueryResult<Vec<T>> {
        let conn = self
            .connection()
            .await
            .map_err(|e| prax_query::QueryError::connection(e.to_string()).with_source(e))?;
        let snapshots = conn
            .query_rows(sql, params)
            .await
            .map_err(|e| prax_query::QueryError::database(e.to_string()).with_source(e))?;
        snapshots
            .into_iter()
            .map(|r| {
                T::from_row(&r).map_err(|e| {
                    let msg = e.to_string();
                    prax_query::QueryError::deserialization(msg).with_source(e)
                })
            })
            .collect()
    }

    async fn fetch_affected(
        &self,
        sql: &str,
        params: &[FilterValue],
    ) -> prax_query::QueryResult<u64> {
        let conn = self
            .connection()
            .await
            .map_err(|e| prax_query::QueryError::connection(e.to_string()).with_source(e))?;
        let affected = conn
            .execute(sql, params)
            .await
            .map_err(|e| prax_query::QueryError::database(e.to_string()).with_source(e))?;
        Ok(affected as u64)
    }
}

/// Drop-based guard for the open `BEGIN TRANSACTION` in
/// [`prax_query::traits::QueryEngine::transaction`]. If the transaction
/// closure *panics*, unwinding skips the finalisation `match` entirely —
/// without this guard the pooled connection would return to the idle
/// pool still inside the transaction, and the next checkout would
/// inherit it.
///
/// `Drop` cannot `.await`, so the best-effort ROLLBACK is spawned onto
/// the runtime. That is still race-free: the spawned task holds its own
/// `Arc<PooledConnection>`, so the connection cannot be recycled into
/// the pool until the ROLLBACK has completed — no later checkout can
/// interleave statements into the still-open transaction.
struct RollbackOnPanic {
    conn: Arc<PooledConnection>,
    armed: bool,
}

impl RollbackOnPanic {
    fn new(conn: Arc<PooledConnection>) -> Self {
        Self { conn, armed: true }
    }

    /// Stand down after COMMIT/ROLLBACK has been issued explicitly.
    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for RollbackOnPanic {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            let conn = self.conn.clone();
            rt.spawn(async move {
                if let Err(e) = conn.execute_batch("ROLLBACK").await {
                    // The transaction state is unknowable; keep the
                    // connection out of the idle pool.
                    tracing::warn!(
                        error = %e,
                        "panic-guard ROLLBACK failed; retiring connection"
                    );
                    conn.poison();
                }
            });
        } else {
            // No runtime to spawn on; roll back synchronously as a last
            // resort (DuckDB is in-process, so this is quick) rather
            // than recycle the connection with BEGIN still open.
            if self.conn.connection().execute_batch("ROLLBACK").is_err() {
                self.conn.poison();
            }
        }
    }
}

impl prax_query::traits::QueryEngine for DuckDbEngine {
    fn dialect(&self) -> &dyn prax_query::dialect::SqlDialect {
        &prax_query::dialect::Postgres
    }

    fn query_many<T: prax_query::traits::Model + prax_query::row::FromRow + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> prax_query::traits::BoxFuture<'_, prax_query::QueryResult<Vec<T>>> {
        let sql = sql.to_string();
        Box::pin(async move { self.fetch_typed::<T>(&sql, &params).await })
    }

    fn query_one<T: prax_query::traits::Model + prax_query::row::FromRow + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> prax_query::traits::BoxFuture<'_, prax_query::QueryResult<T>> {
        let sql = sql.to_string();
        Box::pin(async move {
            let mut rows: Vec<T> = self.fetch_typed::<T>(&sql, &params).await?;
            if rows.is_empty() {
                Err(prax_query::QueryError::not_found(T::MODEL_NAME))
            } else {
                Ok(rows.swap_remove(0))
            }
        })
    }

    fn query_optional<T: prax_query::traits::Model + prax_query::row::FromRow + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> prax_query::traits::BoxFuture<'_, prax_query::QueryResult<Option<T>>> {
        let sql = sql.to_string();
        Box::pin(async move {
            let mut rows: Vec<T> = self.fetch_typed::<T>(&sql, &params).await?;
            Ok(rows.drain(..).next())
        })
    }

    fn execute_insert<T: prax_query::traits::Model + prax_query::row::FromRow + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> prax_query::traits::BoxFuture<'_, prax_query::QueryResult<T>> {
        let sql = sql.to_string();
        Box::pin(async move {
            let mut rows: Vec<T> = self.fetch_typed::<T>(&sql, &params).await?;
            if rows.is_empty() {
                Err(prax_query::QueryError::deserialization(
                    "INSERT ... RETURNING produced no row".to_string(),
                ))
            } else {
                Ok(rows.swap_remove(0))
            }
        })
    }

    fn execute_update<T: prax_query::traits::Model + prax_query::row::FromRow + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> prax_query::traits::BoxFuture<'_, prax_query::QueryResult<Vec<T>>> {
        let sql = sql.to_string();
        Box::pin(async move { self.fetch_typed::<T>(&sql, &params).await })
    }

    fn execute_delete(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> prax_query::traits::BoxFuture<'_, prax_query::QueryResult<u64>> {
        let sql = sql.to_string();
        Box::pin(async move { self.fetch_affected(&sql, &params).await })
    }

    fn execute_raw(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> prax_query::traits::BoxFuture<'_, prax_query::QueryResult<u64>> {
        let sql = sql.to_string();
        Box::pin(async move { self.fetch_affected(&sql, &params).await })
    }

    fn count(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> prax_query::traits::BoxFuture<'_, prax_query::QueryResult<u64>> {
        let sql = sql.to_string();
        Box::pin(async move {
            let conn = self
                .connection()
                .await
                .map_err(|e| prax_query::QueryError::connection(e.to_string()).with_source(e))?;
            let snapshots = conn
                .query_rows(&sql, &params)
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()).with_source(e))?;
            let first = snapshots.into_iter().next().ok_or_else(|| {
                prax_query::QueryError::deserialization("count returned no row".to_string())
            })?;
            // COUNT(*) in DuckDB returns BigInt, whose column name is
            // usually `count_star()` unless aliased. Probe the RowRef by
            // ordinal (the generic builder emits an unaliased COUNT) by
            // reading the one-and-only column as i64.
            use prax_query::row::RowRef;
            if let Ok(n) = first.get_i64("count") {
                return Ok(n as u64);
            }
            if let Ok(n) = first.get_i64("count_star()") {
                return Ok(n as u64);
            }
            Err(prax_query::QueryError::deserialization(
                "count column missing from DuckDB result".to_string(),
            ))
        })
    }

    fn aggregate_query(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> prax_query::traits::BoxFuture<'_, prax_query::QueryResult<Vec<HashMap<String, FilterValue>>>>
    {
        let sql = sql.to_string();
        Box::pin(async move {
            let conn = self
                .connection()
                .await
                .map_err(|e| prax_query::QueryError::connection(e.to_string()).with_source(e))?;
            // Aggregate result sets don't fit a `Model` schema, so go
            // through the JSON row path and probe each cell rather than
            // the typed `query_rows`/`FromRow` machinery.
            let rows = conn
                .query(&sql, &params)
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()).with_source(e))?;

            Ok(rows
                .iter()
                .map(|row| {
                    let mut map = HashMap::new();
                    if let JsonValue::Object(obj) = row {
                        for (name, value) in obj {
                            map.insert(name.clone(), json_cell_to_filter_value(value));
                        }
                    }
                    map
                })
                .collect())
        })
    }

    fn in_transaction(&self) -> bool {
        self.tx_conn.is_some()
    }

    fn transaction<'a, R, Fut, F>(
        &'a self,
        f: F,
    ) -> prax_query::traits::BoxFuture<'a, prax_query::QueryResult<R>>
    where
        F: FnOnce(Self) -> Fut + Send + 'a,
        Fut: std::future::Future<Output = prax_query::QueryResult<R>> + Send + 'a,
        R: Send + 'a,
        Self: Clone,
    {
        Box::pin(async move {
            // Refuse nested transactions until SAVEPOINT support is
            // wired through the engine; users can still issue SAVEPOINT
            // manually via execute_raw if they need it.
            if self.tx_conn.is_some() {
                return Err(prax_query::QueryError::internal(
                    "nested transactions not yet implemented \
                     (call .transaction() on the outer engine only, or \
                     issue SAVEPOINT via execute_raw)",
                ));
            }

            // Pin one pooled connection for the whole transaction.
            // Every query the closure emits through the tx-bound engine
            // clone is routed back to this connection, so the
            // BEGIN/COMMIT block and the statements inside it share a
            // single DuckDB session.
            let conn =
                self.pool.get().await.map_err(|e| {
                    prax_query::QueryError::connection(e.to_string()).with_source(e)
                })?;

            conn.execute_batch("BEGIN TRANSACTION")
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()).with_source(e))?;

            let tx_conn = Arc::new(conn);
            let finalized = Arc::new(AtomicBool::new(false));
            let tx_engine = DuckDbEngine {
                pool: self.pool.clone(),
                tx_conn: Some(tx_conn.clone()),
                tx_finalized: Some(finalized.clone()),
            };

            // Guard the open transaction: if the closure panics,
            // unwinding skips the finalisation match below and would
            // otherwise leak the connection back to the idle pool still
            // inside BEGIN TRANSACTION.
            let rollback_guard = RollbackOnPanic::new(tx_conn.clone());

            // Run the caller's closure on the tx-bound engine clone.
            let result = f(tx_engine).await;

            // Finalise. The transaction's query phase is over, so flip
            // the finalized flag first: engine clones the closure
            // stashed then fail loudly instead of silently querying the
            // pinned connection outside any transaction. COMMIT on
            // success, ROLLBACK on failure, preserving the caller's
            // error; a failed COMMIT gets a rollback attempt too. A
            // failed rollback leaves the connection's state unknowable,
            // so the connection is poisoned (dropped instead of
            // recycled into the idle pool) and a warning emitted. The
            // guard is disarmed on every path — it only fires when the
            // closure panics and unwinding skips this match.
            finalized.store(true, Ordering::Release);
            match result {
                Ok(v) => match tx_conn.execute_batch("COMMIT").await {
                    Ok(()) => {
                        rollback_guard.disarm();
                        Ok(v)
                    }
                    Err(e) => {
                        if let Err(rb) = tx_conn.execute_batch("ROLLBACK").await {
                            tracing::warn!(
                                error = %rb,
                                "ROLLBACK after failed COMMIT failed; retiring connection"
                            );
                            tx_conn.poison();
                        }
                        rollback_guard.disarm();
                        Err(prax_query::QueryError::database(e.to_string()).with_source(e))
                    }
                },
                Err(e) => {
                    if let Err(rb) = tx_conn.execute_batch("ROLLBACK").await {
                        tracing::warn!(
                            error = %rb,
                            "ROLLBACK failed; retiring connection instead of returning it to the idle pool"
                        );
                        tx_conn.poison();
                    }
                    rollback_guard.disarm();
                    Err(e)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DuckDbConfig;

    #[tokio::test]
    async fn test_engine_creation() {
        let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await.unwrap();
        let engine = DuckDbEngine::new(pool);

        let version = engine.version().await.unwrap();
        assert!(!version.is_empty());
    }

    #[tokio::test]
    async fn test_query_many() {
        let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await.unwrap();
        let engine = DuckDbEngine::new(pool);

        engine
            .raw_sql_batch(
                "CREATE TABLE test (id INTEGER, name VARCHAR);
                 INSERT INTO test VALUES (1, 'Alice'), (2, 'Bob');",
            )
            .await
            .unwrap();

        let results = engine
            .query_many("test", &[], &HashMap::new(), &[], None, None)
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_count() {
        let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await.unwrap();
        let engine = DuckDbEngine::new(pool);

        engine
            .raw_sql_batch(
                "CREATE TABLE test (id INTEGER);
                 INSERT INTO test VALUES (1), (2), (3);",
            )
            .await
            .unwrap();

        let count = engine.count("test", &HashMap::new()).await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        use prax_query::traits::QueryEngine;

        let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await.unwrap();
        let engine = DuckDbEngine::new(pool);

        engine
            .raw_sql_batch("CREATE TABLE tx_test (id INTEGER);")
            .await
            .unwrap();

        let result: prax_query::QueryResult<()> = engine
            .transaction(|tx| async move {
                assert!(tx.in_transaction());
                QueryEngine::execute_raw(&tx, "INSERT INTO tx_test VALUES (1)", vec![]).await?;
                Err(prax_query::QueryError::internal("forced failure"))
            })
            .await;
        assert!(result.is_err());
        assert!(!engine.in_transaction());

        // The insert must have been rolled back: table unchanged.
        let count = engine.count("tx_test", &HashMap::new()).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        use prax_query::traits::QueryEngine;

        let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await.unwrap();
        let engine = DuckDbEngine::new(pool);

        engine
            .raw_sql_batch("CREATE TABLE tx_commit (id INTEGER);")
            .await
            .unwrap();

        let result: prax_query::QueryResult<()> = engine
            .transaction(|tx| async move {
                QueryEngine::execute_raw(&tx, "INSERT INTO tx_commit VALUES (1)", vec![]).await?;
                Ok(())
            })
            .await;
        assert!(result.is_ok());

        let count = engine.count("tx_commit", &HashMap::new()).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_aggregate_query() {
        use prax_query::traits::QueryEngine;

        let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await.unwrap();
        let engine = DuckDbEngine::new(pool);

        engine
            .raw_sql_batch(
                "CREATE TABLE agg (id INTEGER, grp VARCHAR);
                 INSERT INTO agg VALUES (1, 'a'), (2, 'a'), (3, 'b');",
            )
            .await
            .unwrap();

        let rows = QueryEngine::aggregate_query(
            &engine,
            "SELECT grp, COUNT(*) AS cnt, AVG(id) AS avg_id FROM agg GROUP BY grp ORDER BY grp",
            vec![],
        )
        .await
        .unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].get("grp"),
            Some(&FilterValue::String("a".to_string()))
        );
        assert_eq!(rows[0].get("cnt"), Some(&FilterValue::Int(2)));
        assert_eq!(rows[0].get("avg_id"), Some(&FilterValue::Float(1.5)));
        assert_eq!(
            rows[1].get("grp"),
            Some(&FilterValue::String("b".to_string()))
        );
        assert_eq!(rows[1].get("cnt"), Some(&FilterValue::Int(1)));
        assert_eq!(rows[1].get("avg_id"), Some(&FilterValue::Float(3.0)));
    }

    #[tokio::test]
    async fn test_stashed_tx_engine_fails_after_finalize() {
        use prax_query::traits::QueryEngine;

        let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await.unwrap();
        let engine = DuckDbEngine::new(pool);

        engine
            .raw_sql_batch("CREATE TABLE stash (id INTEGER);")
            .await
            .unwrap();

        // Return the tx-bound engine itself from the closure — the
        // post-transaction equivalent of a clone stashed for later use.
        let tx: DuckDbEngine = engine
            .transaction(|tx| async move { Ok::<_, prax_query::QueryError>(tx) })
            .await
            .unwrap();

        // Queries through the stashed engine must fail loudly rather
        // than silently run on the pinned connection outside any
        // transaction.
        let err = QueryEngine::execute_raw(&tx, "INSERT INTO stash VALUES (1)", vec![])
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains(TX_FINALIZED),
            "expected a finalized-transaction error, got: {err}"
        );

        // The rejected statement must not have executed. Drop the
        // stashed engine first: it still pins the pool's only
        // connection (in-memory pools are clamped to one).
        drop(tx);
        let count = engine.count("stash", &HashMap::new()).await.unwrap();
        assert_eq!(count, 0);
    }
}
