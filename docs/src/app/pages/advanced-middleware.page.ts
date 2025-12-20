import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-advanced-middleware',
  standalone: true,
  imports: [CommonModule, CodeBlockComponent],
  templateUrl: './advanced-middleware.page.html',
})
export class AdvancedMiddlewarePage {
  basicExample = `use prax_query::middleware::{
    MiddlewareStack, LoggingMiddleware, MetricsMiddleware,
    TimingMiddleware, LogLevel
};

// Create a middleware stack
let stack = MiddlewareStack::new()
    .with(LoggingMiddleware::new().with_level(LogLevel::Debug))
    .with(TimingMiddleware::new())
    .with(MetricsMiddleware::in_memory().0);

// Use with your client
let client = PraxClient::new(config)
    .with_middleware(stack);`;

  loggingMiddleware = `use prax_query::middleware::{LoggingMiddleware, LogLevel, LoggingConfig};

// Simple setup
let logging = LoggingMiddleware::new()
    .with_level(LogLevel::Debug)
    .with_params(true)  // Log query parameters
    .with_slow_threshold(500_000);  // 500ms slow query warning

// Advanced configuration
let config = LoggingConfig {
    level: LogLevel::Info,
    slow_query_threshold_us: 1_000_000,  // 1 second
    log_params: false,
    log_response: false,
    max_sql_length: 500,
    prefix: "prax".to_string(),
};
let logging = LoggingMiddleware::with_config(config);`;

  metricsMiddleware = `use prax_query::middleware::{MetricsMiddleware, InMemoryMetricsCollector};
use std::sync::Arc;

// Create metrics collector
let collector = Arc::new(InMemoryMetricsCollector::new());
let metrics = MetricsMiddleware::new(collector.clone());

// Use with client...

// Later, get metrics
let stats = collector.get_metrics();
println!("Total queries: {}", stats.total_queries);
println!("Success rate: {:.2}%", stats.success_rate() * 100.0);
println!("Average time: {}μs", stats.avg_time_us);
println!("Slow queries: {}", stats.slow_queries);
println!("Cache hits: {}", stats.cache_hits);

// Queries by type
for (query_type, count) in &stats.queries_by_type {
    println!("  {}: {}", query_type, count);
}`;

  retryMiddleware = `use prax_query::middleware::{RetryMiddleware, RetryConfig, RetryPredicate};
use std::time::Duration;

// Basic retry with defaults (3 retries, exponential backoff)
let retry = RetryMiddleware::default_config();

// Custom retry configuration
let retry = RetryMiddleware::new(
    RetryConfig::new()
        .max_retries(5)
        .initial_delay(Duration::from_millis(100))
        .max_delay(Duration::from_secs(10))
        .backoff_multiplier(2.0)
        .jitter(true)  // Add randomness to prevent thundering herd
        .retry_on(RetryPredicate::Default)  // Connection + timeout errors
);

// Retry only specific errors
let retry = RetryMiddleware::new(
    RetryConfig::new()
        .retry_on(RetryPredicate::Custom(vec![
            RetryableError::Connection,
            RetryableError::Timeout,
        ]))
);`;

  customMiddleware = `use prax_query::middleware::{
    Middleware, QueryContext, QueryResponse, Next,
    MiddlewareResult, BoxFuture
};

struct AuthMiddleware {
    tenant_id: String,
}

impl Middleware for AuthMiddleware {
    fn handle<'a>(
        &'a self,
        mut ctx: QueryContext,
        next: Next<'a>,
    ) -> BoxFuture<'a, MiddlewareResult<QueryResponse>> {
        Box::pin(async move {
            // Add tenant context to all queries
            ctx.metadata_mut().tenant_id = Some(self.tenant_id.clone());

            // Modify SQL to add tenant filter (example)
            if ctx.query_type().is_read() {
                let sql = ctx.sql().to_string();
                // Add tenant filtering logic...
            }

            // Continue to next middleware
            next.run(ctx).await
        })
    }

    fn name(&self) -> &'static str {
        "AuthMiddleware"
    }
}`;

  queryContext = `use prax_query::middleware::{QueryContext, QueryMetadata, QueryType};

// Query context provides information about the current query
fn inspect_context(ctx: &QueryContext) {
    // Get SQL and parameters
    println!("SQL: {}", ctx.sql());
    println!("Params: {:?}", ctx.params());

    // Query type detection
    match ctx.query_type() {
        QueryType::Select => println!("This is a SELECT query"),
        QueryType::Insert => println!("This is an INSERT query"),
        QueryType::Update => println!("This is an UPDATE query"),
        QueryType::Delete => println!("This is a DELETE query"),
        QueryType::Count => println!("This is a COUNT query"),
        _ => println!("Other query type"),
    }

    // Check query characteristics
    if ctx.is_read() {
        println!("Read operation - safe for replicas");
    }
    if ctx.is_write() {
        println!("Write operation - use primary");
    }

    // Timing
    println!("Elapsed: {:?}", ctx.elapsed());
}

// Query metadata for tracing
let metadata = QueryMetadata::new()
    .with_model("User")
    .with_operation("findMany")
    .with_request_id("req-abc123")
    .with_user_id("user-456")
    .with_tenant_id("tenant-789")
    .with_tag("env", "production");`;
}


