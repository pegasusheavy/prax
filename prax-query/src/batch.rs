//! Batch query execution for combining multiple operations.
//!
//! This module provides utilities for executing multiple queries in a single
//! database round-trip, improving performance for bulk operations.
//!
//! # Example
//!
//! ```rust,ignore
//! use prax_query::batch::BatchBuilder;
//!
//! let batch = BatchBuilder::new()
//!     .insert("users", &user1_data)
//!     .insert("users", &user2_data)
//!     .insert("users", &user3_data)
//!     .build();
//!
//! let results = engine.execute_batch(batch).await?;
//! ```

use crate::filter::FilterValue;
use crate::sql::{DatabaseType, FastSqlBuilder, QueryCapacity};
use std::collections::HashMap;

/// A batch of operations to execute together.
#[derive(Debug, Clone)]
pub struct Batch {
    /// The operations in the batch.
    operations: Vec<BatchOperation>,
}

impl Batch {
    /// Create a new empty batch.
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    /// Create a batch with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            operations: Vec::with_capacity(capacity),
        }
    }

    /// Add an operation to the batch.
    pub fn add(&mut self, op: BatchOperation) {
        self.operations.push(op);
    }

    /// Get the operations in the batch.
    pub fn operations(&self) -> &[BatchOperation] {
        &self.operations
    }

    /// Get the number of operations.
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Check if the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Convert the batch to a single SQL statement for databases that support it.
    ///
    /// This combines multiple INSERT statements into a single multi-row INSERT.
    pub fn to_combined_sql(&self, db_type: DatabaseType) -> Option<(String, Vec<FilterValue>)> {
        if self.operations.is_empty() {
            return None;
        }

        // Group operations by type and table
        let mut inserts: HashMap<&str, Vec<&BatchOperation>> = HashMap::new();
        let mut other_ops = Vec::new();

        for op in &self.operations {
            match op {
                BatchOperation::Insert { table, .. } => {
                    inserts.entry(table.as_str()).or_default().push(op);
                }
                _ => other_ops.push(op),
            }
        }

        // If we have non-insert operations or multiple tables, can't combine
        if !other_ops.is_empty() || inserts.len() > 1 {
            return None;
        }

        // Combine inserts for a single table
        if let Some((table, ops)) = inserts.into_iter().next() {
            return self.combine_inserts(table, &ops, db_type);
        }

        None
    }

    /// Combine multiple INSERT operations into a single multi-row INSERT.
    fn combine_inserts(
        &self,
        table: &str,
        ops: &[&BatchOperation],
        db_type: DatabaseType,
    ) -> Option<(String, Vec<FilterValue>)> {
        if ops.is_empty() {
            return None;
        }

        // Get columns from first insert
        let first_columns: Vec<&str> = match &ops[0] {
            BatchOperation::Insert { data, .. } => data.keys().map(String::as_str).collect(),
            _ => return None,
        };

        // Verify all inserts have the same columns
        for op in ops.iter().skip(1) {
            if let BatchOperation::Insert { data, .. } = op {
                let cols: Vec<&str> = data.keys().map(String::as_str).collect();
                if cols.len() != first_columns.len() {
                    return None;
                }
            }
        }

        // Build combined INSERT
        let cols_per_row = first_columns.len();
        let total_params = cols_per_row * ops.len();

        let mut builder =
            FastSqlBuilder::with_capacity(db_type, QueryCapacity::Custom(64 + total_params * 8));

        builder.push_str("INSERT INTO ");
        builder.push_str(table);
        builder.push_str(" (");

        for (i, col) in first_columns.iter().enumerate() {
            if i > 0 {
                builder.push_str(", ");
            }
            builder.push_str(col);
        }

        builder.push_str(") VALUES ");

        let mut all_params = Vec::with_capacity(total_params);

        for (row_idx, op) in ops.iter().enumerate() {
            if row_idx > 0 {
                builder.push_str(", ");
            }
            builder.push_char('(');

            if let BatchOperation::Insert { data, .. } = op {
                for (col_idx, col) in first_columns.iter().enumerate() {
                    if col_idx > 0 {
                        builder.push_str(", ");
                    }
                    builder.bind(data.get(*col).cloned().unwrap_or(FilterValue::Null));
                    if let Some(val) = data.get(*col) {
                        all_params.push(val.clone());
                    } else {
                        all_params.push(FilterValue::Null);
                    }
                }
            }

            builder.push_char(')');
        }

        Some(builder.build())
    }
}

impl Default for Batch {
    fn default() -> Self {
        Self::new()
    }
}

/// A single operation in a batch.
#[derive(Debug, Clone)]
pub enum BatchOperation {
    /// An INSERT operation.
    Insert {
        /// The table name.
        table: String,
        /// The data to insert.
        data: HashMap<String, FilterValue>,
    },
    /// An UPDATE operation.
    Update {
        /// The table name.
        table: String,
        /// The filter for which rows to update.
        filter: HashMap<String, FilterValue>,
        /// The data to update.
        data: HashMap<String, FilterValue>,
    },
    /// A DELETE operation.
    Delete {
        /// The table name.
        table: String,
        /// The filter for which rows to delete.
        filter: HashMap<String, FilterValue>,
    },
    /// A raw SQL operation.
    Raw {
        /// The SQL query.
        sql: String,
        /// The parameters.
        params: Vec<FilterValue>,
    },
}

impl BatchOperation {
    /// Create an INSERT operation.
    pub fn insert(table: impl Into<String>, data: HashMap<String, FilterValue>) -> Self {
        Self::Insert {
            table: table.into(),
            data,
        }
    }

    /// Create an UPDATE operation.
    pub fn update(
        table: impl Into<String>,
        filter: HashMap<String, FilterValue>,
        data: HashMap<String, FilterValue>,
    ) -> Self {
        Self::Update {
            table: table.into(),
            filter,
            data,
        }
    }

    /// Create a DELETE operation.
    pub fn delete(table: impl Into<String>, filter: HashMap<String, FilterValue>) -> Self {
        Self::Delete {
            table: table.into(),
            filter,
        }
    }

    /// Create a raw SQL operation.
    pub fn raw(sql: impl Into<String>, params: Vec<FilterValue>) -> Self {
        Self::Raw {
            sql: sql.into(),
            params,
        }
    }
}

/// Builder for creating batches fluently.
#[derive(Debug, Default)]
pub struct BatchBuilder {
    batch: Batch,
}

impl BatchBuilder {
    /// Create a new batch builder.
    pub fn new() -> Self {
        Self {
            batch: Batch::new(),
        }
    }

    /// Create a builder with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            batch: Batch::with_capacity(capacity),
        }
    }

    /// Add an INSERT operation.
    pub fn insert(mut self, table: impl Into<String>, data: HashMap<String, FilterValue>) -> Self {
        self.batch.add(BatchOperation::insert(table, data));
        self
    }

    /// Add an UPDATE operation.
    pub fn update(
        mut self,
        table: impl Into<String>,
        filter: HashMap<String, FilterValue>,
        data: HashMap<String, FilterValue>,
    ) -> Self {
        self.batch.add(BatchOperation::update(table, filter, data));
        self
    }

    /// Add a DELETE operation.
    pub fn delete(
        mut self,
        table: impl Into<String>,
        filter: HashMap<String, FilterValue>,
    ) -> Self {
        self.batch.add(BatchOperation::delete(table, filter));
        self
    }

    /// Add a raw SQL operation.
    pub fn raw(mut self, sql: impl Into<String>, params: Vec<FilterValue>) -> Self {
        self.batch.add(BatchOperation::raw(sql, params));
        self
    }

    /// Build the batch.
    pub fn build(self) -> Batch {
        self.batch
    }
}

/// Result of a batch execution.
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Results for each operation.
    pub results: Vec<OperationResult>,
    /// Total rows affected across all operations.
    pub total_affected: u64,
}

impl BatchResult {
    /// Create a new batch result.
    pub fn new(results: Vec<OperationResult>) -> Self {
        let total_affected = results.iter().map(|r| r.rows_affected).sum();
        Self {
            results,
            total_affected,
        }
    }

    /// Get the number of operations.
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Check if all operations succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.results.iter().all(|r| r.success)
    }
}

/// Result of a single operation in a batch.
#[derive(Debug, Clone)]
pub struct OperationResult {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Number of rows affected.
    pub rows_affected: u64,
    /// Error message if failed.
    pub error: Option<String>,
}

impl OperationResult {
    /// Create a successful result.
    pub fn success(rows_affected: u64) -> Self {
        Self {
            success: true,
            rows_affected,
            error: None,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            rows_affected: 0,
            error: Some(error.into()),
        }
    }
}

// ============================================================================
// Pipeline Execution
// ============================================================================

/// A query pipeline for executing multiple queries efficiently.
///
/// Pipelines combine multiple queries and execute them with minimal
/// round-trips to the database. This is especially useful for:
///
/// - Fetching a parent record and its relations
/// - Performing multiple inserts in sequence
/// - Complex transactions with multiple operations
///
/// # Example
///
/// ```rust,ignore
/// use prax_query::batch::Pipeline;
///
/// let pipeline = Pipeline::new()
///     .query("SELECT * FROM users WHERE id = $1", vec![id.into()])
///     .query("SELECT * FROM posts WHERE author_id = $1", vec![id.into()])
///     .build();
///
/// let results = engine.execute_pipeline(pipeline).await?;
/// ```
#[derive(Debug, Clone)]
pub struct Pipeline {
    /// Queries in the pipeline.
    queries: Vec<PipelineQuery>,
}

impl Pipeline {
    /// Create a new empty pipeline.
    pub fn new() -> Self {
        Self {
            queries: Vec::new(),
        }
    }

    /// Create a pipeline with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queries: Vec::with_capacity(capacity),
        }
    }

    /// Add a query to the pipeline.
    pub fn push(&mut self, sql: impl Into<String>, params: Vec<FilterValue>) {
        self.queries.push(PipelineQuery {
            sql: sql.into(),
            params,
            expect_rows: true,
        });
    }

    /// Add an execute-only query (no result rows expected).
    pub fn push_execute(&mut self, sql: impl Into<String>, params: Vec<FilterValue>) {
        self.queries.push(PipelineQuery {
            sql: sql.into(),
            params,
            expect_rows: false,
        });
    }

    /// Get the queries.
    pub fn queries(&self) -> &[PipelineQuery] {
        &self.queries
    }

    /// Get the number of queries.
    pub fn len(&self) -> usize {
        self.queries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.queries.is_empty()
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// A single query in a pipeline.
#[derive(Debug, Clone)]
pub struct PipelineQuery {
    /// The SQL query.
    pub sql: String,
    /// Query parameters.
    pub params: Vec<FilterValue>,
    /// Whether this query returns rows.
    pub expect_rows: bool,
}

/// Builder for creating pipelines.
#[derive(Debug, Clone)]
pub struct PipelineBuilder {
    pipeline: Pipeline,
}

impl PipelineBuilder {
    /// Create a new pipeline builder.
    pub fn new() -> Self {
        Self {
            pipeline: Pipeline::new(),
        }
    }

    /// Create a builder with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            pipeline: Pipeline::with_capacity(capacity),
        }
    }

    /// Add a SELECT query.
    pub fn query(mut self, sql: impl Into<String>, params: Vec<FilterValue>) -> Self {
        self.pipeline.push(sql, params);
        self
    }

    /// Add an execute-only query (INSERT/UPDATE/DELETE).
    pub fn execute(mut self, sql: impl Into<String>, params: Vec<FilterValue>) -> Self {
        self.pipeline.push_execute(sql, params);
        self
    }

    /// Build the pipeline.
    pub fn build(self) -> Pipeline {
        self.pipeline
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of pipeline execution.
#[derive(Debug)]
pub struct PipelineResult {
    /// Results for each query in the pipeline.
    pub query_results: Vec<QueryResult>,
}

/// Result of a single query in a pipeline.
#[derive(Debug)]
pub enum QueryResult {
    /// Query returned rows.
    Rows {
        /// Number of rows returned.
        count: usize,
    },
    /// Query was executed (no rows).
    Executed {
        /// Rows affected.
        rows_affected: u64,
    },
    /// Query failed.
    Error {
        /// Error message.
        message: String,
    },
}

impl PipelineResult {
    /// Create a new pipeline result.
    pub fn new(query_results: Vec<QueryResult>) -> Self {
        Self { query_results }
    }

    /// Check if all queries succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.query_results
            .iter()
            .all(|r| !matches!(r, QueryResult::Error { .. }))
    }

    /// Get first error if any.
    pub fn first_error(&self) -> Option<&str> {
        self.query_results.iter().find_map(|r| {
            if let QueryResult::Error { message } = r {
                Some(message.as_str())
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_builder() {
        let mut data1 = HashMap::new();
        data1.insert("name".to_string(), FilterValue::String("Alice".into()));

        let mut data2 = HashMap::new();
        data2.insert("name".to_string(), FilterValue::String("Bob".into()));

        let batch = BatchBuilder::new()
            .insert("users", data1)
            .insert("users", data2)
            .build();

        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn test_combine_inserts_postgres() {
        let mut data1 = HashMap::new();
        data1.insert("name".to_string(), FilterValue::String("Alice".into()));
        data1.insert("age".to_string(), FilterValue::Int(30));

        let mut data2 = HashMap::new();
        data2.insert("name".to_string(), FilterValue::String("Bob".into()));
        data2.insert("age".to_string(), FilterValue::Int(25));

        let batch = BatchBuilder::new()
            .insert("users", data1)
            .insert("users", data2)
            .build();

        let result = batch.to_combined_sql(DatabaseType::PostgreSQL);
        assert!(result.is_some());

        let (sql, _) = result.unwrap();
        assert!(sql.starts_with("INSERT INTO users"));
        assert!(sql.contains("VALUES"));
    }

    #[test]
    fn test_batch_result() {
        let results = vec![
            OperationResult::success(1),
            OperationResult::success(1),
            OperationResult::success(1),
        ];

        let batch_result = BatchResult::new(results);
        assert_eq!(batch_result.total_affected, 3);
        assert!(batch_result.all_succeeded());
    }

    #[test]
    fn test_batch_result_with_failure() {
        let results = vec![
            OperationResult::success(1),
            OperationResult::failure("constraint violation"),
            OperationResult::success(1),
        ];

        let batch_result = BatchResult::new(results);
        assert_eq!(batch_result.total_affected, 2);
        assert!(!batch_result.all_succeeded());
    }
}
