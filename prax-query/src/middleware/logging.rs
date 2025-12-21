//! Logging middleware for query tracing.

use super::context::QueryContext;
use super::types::{BoxFuture, Middleware, MiddlewareResult, Next, QueryResponse};
use std::sync::atomic::{AtomicU64, Ordering};

/// Log level for query logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Log nothing.
    Off,
    /// Log only errors.
    Error,
    /// Log errors and warnings (slow queries).
    Warn,
    /// Log all queries.
    Info,
    /// Log queries with parameters.
    Debug,
    /// Log everything including internal details.
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// Configuration for the logging middleware.
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Minimum log level.
    pub level: LogLevel,
    /// Threshold for slow query warnings (microseconds).
    pub slow_query_threshold_us: u64,
    /// Whether to log query parameters.
    pub log_params: bool,
    /// Whether to log response data.
    pub log_response: bool,
    /// Maximum length of logged SQL (0 = unlimited).
    pub max_sql_length: usize,
    /// Prefix for log messages.
    pub prefix: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            slow_query_threshold_us: 1_000_000, // 1 second
            log_params: false,
            log_response: false,
            max_sql_length: 500,
            prefix: "prax".to_string(),
        }
    }
}

/// Middleware that logs queries.
///
/// # Example
///
/// ```rust,ignore
/// use prax_query::middleware::{LoggingMiddleware, LogLevel};
///
/// let logging = LoggingMiddleware::new()
///     .with_level(LogLevel::Debug)
///     .with_params(true)
///     .with_slow_threshold(500_000); // 500ms
/// ```
pub struct LoggingMiddleware {
    config: LoggingConfig,
    query_count: AtomicU64,
}

impl LoggingMiddleware {
    /// Create a new logging middleware with default settings.
    pub fn new() -> Self {
        Self {
            config: LoggingConfig::default(),
            query_count: AtomicU64::new(0),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: LoggingConfig) -> Self {
        Self {
            config,
            query_count: AtomicU64::new(0),
        }
    }

    /// Set the log level.
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.config.level = level;
        self
    }

    /// Enable parameter logging.
    pub fn with_params(mut self, enabled: bool) -> Self {
        self.config.log_params = enabled;
        self
    }

    /// Enable response logging.
    pub fn with_response(mut self, enabled: bool) -> Self {
        self.config.log_response = enabled;
        self
    }

    /// Set slow query threshold in microseconds.
    pub fn with_slow_threshold(mut self, threshold_us: u64) -> Self {
        self.config.slow_query_threshold_us = threshold_us;
        self
    }

    /// Set the log prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.config.prefix = prefix.into();
        self
    }

    /// Get the total query count.
    pub fn query_count(&self) -> u64 {
        self.query_count.load(Ordering::Relaxed)
    }

    fn truncate_sql(&self, sql: &str) -> String {
        if self.config.max_sql_length == 0 || sql.len() <= self.config.max_sql_length {
            sql.to_string()
        } else {
            format!("{}...", &sql[..self.config.max_sql_length])
        }
    }

    fn log_before(&self, ctx: &QueryContext, query_id: u64) {
        if self.config.level < LogLevel::Debug {
            return;
        }

        let sql = self.truncate_sql(ctx.sql());
        let query_type = format!("{:?}", ctx.query_type());

        if self.config.log_params && self.config.level >= LogLevel::Trace {
            tracing::debug!(
                target: "prax::query",
                query_id = query_id,
                query_type = %query_type,
                sql = %sql,
                params = ?ctx.params(),
                model = ?ctx.metadata().model,
                operation = ?ctx.metadata().operation,
                request_id = ?ctx.metadata().request_id,
                "[{}] Starting query",
                self.config.prefix
            );
        } else {
            tracing::debug!(
                target: "prax::query",
                query_id = query_id,
                query_type = %query_type,
                sql = %sql,
                "[{}] Starting query",
                self.config.prefix
            );
        }
    }

    fn log_after(&self, ctx: &QueryContext, response: &QueryResponse, query_id: u64) {
        let duration_us = response.execution_time_us;
        let is_slow = duration_us >= self.config.slow_query_threshold_us;

        if is_slow && self.config.level >= LogLevel::Warn {
            let sql = self.truncate_sql(ctx.sql());
            tracing::warn!(
                target: "prax::query",
                query_id = query_id,
                duration_us = duration_us,
                duration_ms = duration_us / 1000,
                sql = %sql,
                threshold_us = self.config.slow_query_threshold_us,
                "[{}] Slow query detected",
                self.config.prefix
            );
        } else if self.config.level >= LogLevel::Info {
            let sql = self.truncate_sql(ctx.sql());

            if self.config.log_response && self.config.level >= LogLevel::Trace {
                tracing::info!(
                    target: "prax::query",
                    query_id = query_id,
                    duration_us = duration_us,
                    rows_affected = ?response.rows_affected,
                    from_cache = response.from_cache,
                    sql = %sql,
                    response = ?response.data,
                    "[{}] Query completed",
                    self.config.prefix
                );
            } else {
                tracing::info!(
                    target: "prax::query",
                    query_id = query_id,
                    duration_us = duration_us,
                    rows_affected = ?response.rows_affected,
                    from_cache = response.from_cache,
                    "[{}] Query completed",
                    self.config.prefix
                );
            }
        }
    }

    fn log_error(&self, ctx: &QueryContext, error: &crate::QueryError, query_id: u64) {
        if self.config.level >= LogLevel::Error {
            let sql = self.truncate_sql(ctx.sql());
            tracing::error!(
                target: "prax::query",
                query_id = query_id,
                sql = %sql,
                error = %error,
                "[{}] Query failed",
                self.config.prefix
            );
        }
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for LoggingMiddleware {
    fn handle<'a>(
        &'a self,
        ctx: QueryContext,
        next: Next<'a>,
    ) -> BoxFuture<'a, MiddlewareResult<QueryResponse>> {
        Box::pin(async move {
            let query_id = self.query_count.fetch_add(1, Ordering::SeqCst);

            // Log before
            self.log_before(&ctx, query_id);

            // Execute query
            let result = next.run(ctx.clone()).await;

            // Log after
            match &result {
                Ok(response) => self.log_after(&ctx, response, query_id),
                Err(error) => self.log_error(&ctx, error, query_id),
            }

            result
        })
    }

    fn name(&self) -> &'static str {
        "LoggingMiddleware"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }

    #[test]
    fn test_logging_middleware_builder() {
        let middleware = LoggingMiddleware::new()
            .with_level(LogLevel::Debug)
            .with_params(true)
            .with_slow_threshold(500_000);

        assert_eq!(middleware.config.level, LogLevel::Debug);
        assert!(middleware.config.log_params);
        assert_eq!(middleware.config.slow_query_threshold_us, 500_000);
    }

    #[test]
    fn test_truncate_sql() {
        let middleware = LoggingMiddleware::new();

        let short = "SELECT * FROM users";
        assert_eq!(middleware.truncate_sql(short), short);

        let config = LoggingConfig {
            max_sql_length: 10,
            ..Default::default()
        };
        let middleware = LoggingMiddleware::with_config(config);
        let long = "SELECT * FROM users WHERE id = 1";
        assert!(middleware.truncate_sql(long).ends_with("..."));
    }

    #[test]
    fn test_query_count() {
        let middleware = LoggingMiddleware::new();
        assert_eq!(middleware.query_count(), 0);
    }
}
