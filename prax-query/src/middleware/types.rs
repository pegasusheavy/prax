//! Core middleware types and traits.

use crate::QueryError;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::context::QueryContext;

/// Result type for middleware operations.
pub type MiddlewareResult<T> = Result<T, QueryError>;

/// A boxed future for async middleware operations.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The next handler in the middleware chain.
///
/// Call this to continue processing to the next middleware or the actual query.
pub struct Next<'a> {
    pub(crate) inner:
        Box<dyn FnOnce(QueryContext) -> BoxFuture<'a, MiddlewareResult<QueryResponse>> + Send + 'a>,
}

impl<'a> Next<'a> {
    /// Execute the next handler in the chain.
    pub fn run(self, ctx: QueryContext) -> BoxFuture<'a, MiddlewareResult<QueryResponse>> {
        (self.inner)(ctx)
    }
}

/// Response from a query execution.
#[derive(Debug, Clone)]
pub struct QueryResponse {
    /// The raw response data (typically JSON).
    pub data: serde_json::Value,
    /// Number of rows affected (for mutations).
    pub rows_affected: Option<u64>,
    /// Execution time in microseconds.
    pub execution_time_us: u64,
    /// Whether the query was served from cache.
    pub from_cache: bool,
    /// Additional metadata.
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl QueryResponse {
    /// Create a new query response with data.
    pub fn new(data: serde_json::Value) -> Self {
        Self {
            data,
            rows_affected: None,
            execution_time_us: 0,
            from_cache: false,
            metadata: serde_json::Map::new(),
        }
    }

    /// Create an empty response.
    pub fn empty() -> Self {
        Self::new(serde_json::Value::Null)
    }

    /// Create a response with affected rows count.
    pub fn with_affected(count: u64) -> Self {
        Self {
            data: serde_json::Value::Null,
            rows_affected: Some(count),
            execution_time_us: 0,
            from_cache: false,
            metadata: serde_json::Map::new(),
        }
    }

    /// Set execution time.
    pub fn with_execution_time(mut self, us: u64) -> Self {
        self.execution_time_us = us;
        self
    }

    /// Mark as from cache.
    pub fn from_cache(mut self) -> Self {
        self.from_cache = true;
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Middleware trait for intercepting queries.
///
/// Implement this trait to create custom middleware that can:
/// - Modify queries before execution
/// - Modify responses after execution
/// - Short-circuit execution (e.g., for caching)
/// - Add logging, metrics, or other side effects
///
/// # Example
///
/// ```rust,ignore
/// use prax_query::middleware::{Middleware, QueryContext, QueryResponse, Next, MiddlewareResult};
///
/// struct MyMiddleware;
///
/// impl Middleware for MyMiddleware {
///     fn handle<'a>(
///         &'a self,
///         ctx: QueryContext,
///         next: Next<'a>,
///     ) -> BoxFuture<'a, MiddlewareResult<QueryResponse>> {
///         Box::pin(async move {
///             // Before query
///             println!("Executing: {}", ctx.sql());
///
///             // Call next middleware or execute query
///             let response = next.run(ctx).await?;
///
///             // After query
///             println!("Completed in {}us", response.execution_time_us);
///
///             Ok(response)
///         })
///     }
/// }
/// ```
pub trait Middleware: Send + Sync {
    /// Handle a query, optionally calling the next handler.
    fn handle<'a>(
        &'a self,
        ctx: QueryContext,
        next: Next<'a>,
    ) -> BoxFuture<'a, MiddlewareResult<QueryResponse>>;

    /// Name of this middleware (for debugging/logging).
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Whether this middleware is enabled.
    fn enabled(&self) -> bool {
        true
    }
}

/// A middleware that can be shared across threads.
pub type SharedMiddleware = Arc<dyn Middleware>;

/// Convenience trait for boxing middleware.
pub trait IntoSharedMiddleware {
    /// Convert into a shared middleware.
    fn into_shared(self) -> SharedMiddleware;
}

impl<T: Middleware + 'static> IntoSharedMiddleware for T {
    fn into_shared(self) -> SharedMiddleware {
        Arc::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_response_builder() {
        let response = QueryResponse::new(serde_json::json!({"id": 1}))
            .with_execution_time(1000)
            .with_metadata("cache_hit", serde_json::Value::Bool(false));

        assert_eq!(response.execution_time_us, 1000);
        assert!(!response.from_cache);
        assert!(response.metadata.contains_key("cache_hit"));
    }

    #[test]
    fn test_query_response_affected() {
        let response = QueryResponse::with_affected(5);
        assert_eq!(response.rows_affected, Some(5));
    }

    #[test]
    fn test_query_response_from_cache() {
        let response = QueryResponse::empty().from_cache();
        assert!(response.from_cache);
    }
}
