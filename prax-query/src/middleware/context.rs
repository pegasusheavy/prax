//! Query context for middleware.

use crate::filter::FilterValue;
use std::collections::HashMap;
use std::time::Instant;

/// The type of query being executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryType {
    /// SELECT query.
    Select,
    /// INSERT query.
    Insert,
    /// UPDATE query.
    Update,
    /// DELETE query.
    Delete,
    /// COUNT query.
    Count,
    /// Raw SQL query.
    Raw,
    /// Transaction begin.
    TransactionBegin,
    /// Transaction commit.
    TransactionCommit,
    /// Transaction rollback.
    TransactionRollback,
    /// Unknown query type.
    Unknown,
}

impl QueryType {
    /// Detect query type from SQL string.
    pub fn from_sql(sql: &str) -> Self {
        let sql = sql.trim().to_uppercase();
        if sql.starts_with("SELECT") {
            // Check if it's a COUNT query
            if sql.contains("COUNT(") {
                Self::Count
            } else {
                Self::Select
            }
        } else if sql.starts_with("INSERT") {
            Self::Insert
        } else if sql.starts_with("UPDATE") {
            Self::Update
        } else if sql.starts_with("DELETE") {
            Self::Delete
        } else if sql.starts_with("BEGIN") || sql.starts_with("START TRANSACTION") {
            Self::TransactionBegin
        } else if sql.starts_with("COMMIT") {
            Self::TransactionCommit
        } else if sql.starts_with("ROLLBACK") {
            Self::TransactionRollback
        } else {
            Self::Unknown
        }
    }

    /// Check if this is a read operation.
    pub fn is_read(&self) -> bool {
        matches!(self, Self::Select | Self::Count)
    }

    /// Check if this is a write operation.
    pub fn is_write(&self) -> bool {
        matches!(self, Self::Insert | Self::Update | Self::Delete)
    }

    /// Check if this is a transaction operation.
    pub fn is_transaction(&self) -> bool {
        matches!(
            self,
            Self::TransactionBegin | Self::TransactionCommit | Self::TransactionRollback
        )
    }
}

/// The current phase of query execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryPhase {
    /// Before the query is executed.
    Before,
    /// During query execution.
    During,
    /// After the query has completed successfully.
    AfterSuccess,
    /// After the query has failed.
    AfterError,
}

/// Metadata about a query.
#[derive(Debug, Clone)]
pub struct QueryMetadata {
    /// The model being queried (if known).
    pub model: Option<String>,
    /// The operation name (e.g., "findMany", "create").
    pub operation: Option<String>,
    /// Request ID for tracing.
    pub request_id: Option<String>,
    /// User ID for auditing.
    pub user_id: Option<String>,
    /// Tenant ID for multi-tenancy.
    pub tenant_id: Option<String>,
    /// Schema override for multi-tenancy.
    pub schema_override: Option<String>,
    /// Custom tags for filtering.
    pub tags: HashMap<String, String>,
    /// Custom attributes.
    pub attributes: HashMap<String, serde_json::Value>,
}

impl Default for QueryMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryMetadata {
    /// Create new empty metadata.
    pub fn new() -> Self {
        Self {
            model: None,
            operation: None,
            request_id: None,
            user_id: None,
            tenant_id: None,
            schema_override: None,
            tags: HashMap::new(),
            attributes: HashMap::new(),
        }
    }

    /// Set the model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the operation name.
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    /// Set the request ID.
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Set the user ID.
    pub fn with_user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = Some(id.into());
        self
    }

    /// Set the tenant ID.
    pub fn with_tenant_id(mut self, id: impl Into<String>) -> Self {
        self.tenant_id = Some(id.into());
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Add an attribute.
    pub fn with_attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    /// Set the schema override for multi-tenancy.
    pub fn set_schema_override(&mut self, schema: Option<String>) {
        self.schema_override = schema;
    }

    /// Get the schema override.
    pub fn schema_override(&self) -> Option<&str> {
        self.schema_override.as_deref()
    }
}

/// Context passed through the middleware chain.
#[derive(Debug, Clone)]
pub struct QueryContext {
    /// The SQL query string.
    sql: String,
    /// Query parameters.
    params: Vec<FilterValue>,
    /// Query type.
    query_type: QueryType,
    /// Query metadata.
    metadata: QueryMetadata,
    /// When the query started.
    started_at: Instant,
    /// Current execution phase.
    phase: QueryPhase,
    /// Whether the query should be skipped (e.g., cache hit).
    skip_execution: bool,
    /// Cached response (if skipping execution).
    cached_response: Option<serde_json::Value>,
}

impl QueryContext {
    /// Create a new query context.
    pub fn new(sql: impl Into<String>, params: Vec<FilterValue>) -> Self {
        let sql = sql.into();
        let query_type = QueryType::from_sql(&sql);
        Self {
            sql,
            params,
            query_type,
            metadata: QueryMetadata::new(),
            started_at: Instant::now(),
            phase: QueryPhase::Before,
            skip_execution: false,
            cached_response: None,
        }
    }

    /// Get the SQL string.
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Get mutable SQL string (for query modification).
    pub fn sql_mut(&mut self) -> &mut String {
        &mut self.sql
    }

    /// Set a new SQL string.
    pub fn set_sql(&mut self, sql: impl Into<String>) {
        self.sql = sql.into();
        self.query_type = QueryType::from_sql(&self.sql);
    }

    /// Set a new SQL string (builder pattern).
    pub fn with_sql(mut self, sql: impl Into<String>) -> Self {
        self.set_sql(sql);
        self
    }

    /// Get the query parameters.
    pub fn params(&self) -> &[FilterValue] {
        &self.params
    }

    /// Get mutable parameters.
    pub fn params_mut(&mut self) -> &mut Vec<FilterValue> {
        &mut self.params
    }

    /// Get the query type.
    pub fn query_type(&self) -> QueryType {
        self.query_type
    }

    /// Get the metadata.
    pub fn metadata(&self) -> &QueryMetadata {
        &self.metadata
    }

    /// Get mutable metadata.
    pub fn metadata_mut(&mut self) -> &mut QueryMetadata {
        &mut self.metadata
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: QueryMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Get elapsed time since query started.
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    /// Get elapsed time in microseconds.
    pub fn elapsed_us(&self) -> u64 {
        self.started_at.elapsed().as_micros() as u64
    }

    /// Get the current phase.
    pub fn phase(&self) -> QueryPhase {
        self.phase
    }

    /// Set the current phase.
    pub fn set_phase(&mut self, phase: QueryPhase) {
        self.phase = phase;
    }

    /// Check if execution should be skipped.
    pub fn should_skip(&self) -> bool {
        self.skip_execution
    }

    /// Mark query to skip execution (e.g., for cache hit).
    pub fn skip_with_response(&mut self, response: serde_json::Value) {
        self.skip_execution = true;
        self.cached_response = Some(response);
    }

    /// Get the cached response if skipping.
    pub fn cached_response(&self) -> Option<&serde_json::Value> {
        self.cached_response.as_ref()
    }

    /// Check if this is a read query.
    pub fn is_read(&self) -> bool {
        self.query_type.is_read()
    }

    /// Check if this is a write query.
    pub fn is_write(&self) -> bool {
        self.query_type.is_write()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_type_detection() {
        assert_eq!(
            QueryType::from_sql("SELECT * FROM users"),
            QueryType::Select
        );
        assert_eq!(
            QueryType::from_sql("INSERT INTO users VALUES (1)"),
            QueryType::Insert
        );
        assert_eq!(
            QueryType::from_sql("UPDATE users SET name = 'test'"),
            QueryType::Update
        );
        assert_eq!(
            QueryType::from_sql("DELETE FROM users WHERE id = 1"),
            QueryType::Delete
        );
        assert_eq!(
            QueryType::from_sql("SELECT COUNT(*) FROM users"),
            QueryType::Count
        );
        assert_eq!(QueryType::from_sql("BEGIN"), QueryType::TransactionBegin);
        assert_eq!(QueryType::from_sql("COMMIT"), QueryType::TransactionCommit);
        assert_eq!(
            QueryType::from_sql("ROLLBACK"),
            QueryType::TransactionRollback
        );
    }

    #[test]
    fn test_query_type_categories() {
        assert!(QueryType::Select.is_read());
        assert!(QueryType::Count.is_read());
        assert!(!QueryType::Insert.is_read());

        assert!(QueryType::Insert.is_write());
        assert!(QueryType::Update.is_write());
        assert!(QueryType::Delete.is_write());
        assert!(!QueryType::Select.is_write());

        assert!(QueryType::TransactionBegin.is_transaction());
        assert!(QueryType::TransactionCommit.is_transaction());
        assert!(QueryType::TransactionRollback.is_transaction());
    }

    #[test]
    fn test_query_context() {
        let ctx = QueryContext::new("SELECT * FROM users", vec![]);
        assert_eq!(ctx.sql(), "SELECT * FROM users");
        assert_eq!(ctx.query_type(), QueryType::Select);
        assert!(ctx.is_read());
        assert!(!ctx.is_write());
    }

    #[test]
    fn test_query_metadata() {
        let metadata = QueryMetadata::new()
            .with_model("User")
            .with_operation("findMany")
            .with_request_id("req-123")
            .with_tag("env", "production");

        assert_eq!(metadata.model, Some("User".to_string()));
        assert_eq!(metadata.operation, Some("findMany".to_string()));
        assert_eq!(metadata.tags.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_context_skip_execution() {
        let mut ctx = QueryContext::new("SELECT * FROM users", vec![]);
        assert!(!ctx.should_skip());

        ctx.skip_with_response(serde_json::json!({"cached": true}));
        assert!(ctx.should_skip());
        assert!(ctx.cached_response().is_some());
    }
}
