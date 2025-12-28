import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-advanced-performance-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './advanced-performance.page.html',
})
export class AdvancedPerformancePage {
  rowRefExample = `use prax_query::row::RowRef;

// The RowRef trait provides zero-copy access
pub trait RowRef {
    // Zero-copy string access
    fn get_str(&self, column: &str) -> Result<&str, RowError>;
    fn get_str_opt(&self, column: &str) -> Result<Option<&str>, RowError>;

    // Zero-copy bytes access
    fn get_bytes(&self, column: &str) -> Result<&[u8], RowError>;

    // Copy types (always copy)
    fn get_i32(&self, column: &str) -> Result<i32, RowError>;
    fn get_i64(&self, column: &str) -> Result<i64, RowError>;
    fn get_f64(&self, column: &str) -> Result<f64, RowError>;
    fn get_bool(&self, column: &str) -> Result<bool, RowError>;
}`;

  fromRowRefExample = `use prax_query::row::{FromRowRef, RowRef, RowError};

// Struct that borrows from the row
struct UserRef<'a> {
    id: i32,
    email: &'a str,      // Zero-copy!
    name: Option<&'a str>,
    bio: &'a str,        // Zero-copy!
}

impl<'a> FromRowRef<'a> for UserRef<'a> {
    fn from_row_ref(row: &'a impl RowRef) -> Result<Self, RowError> {
        Ok(Self {
            id: row.get_i32("id")?,
            email: row.get_str("email")?,     // No allocation
            name: row.get_str_opt("name")?,   // No allocation
            bio: row.get_str("bio")?,         // No allocation
        })
    }
}

// Use it with zero-copy iteration
let users: Vec<UserRef> = rows.iter()
    .map(|row| UserRef::from_row_ref(row))
    .collect::<Result<Vec<_>, _>>()?;`;

  batchExample = `use prax_query::batch::{BatchBuilder, DatabaseType};
use std::collections::HashMap;

// Build a batch of inserts
let batch = BatchBuilder::new()
    .insert("users", hashmap! {
        "name" => "Alice".into(),
        "email" => "alice@example.com".into(),
    })
    .insert("users", hashmap! {
        "name" => "Bob".into(),
        "email" => "bob@example.com".into(),
    })
    .insert("users", hashmap! {
        "name" => "Charlie".into(),
        "email" => "charlie@example.com".into(),
    })
    .build();

// Convert to single multi-row INSERT
if let Some((sql, params)) = batch.to_combined_sql(DatabaseType::PostgreSQL) {
    // sql = "INSERT INTO users (name, email) VALUES ($1, $2), ($3, $4), ($5, $6)"
    engine.execute_raw(&sql, params).await?;
}`;

  pipelineExample = `use prax_query::batch::{PipelineBuilder, FilterValue};

// Build a query pipeline
let pipeline = PipelineBuilder::new()
    // Fetch user
    .query(
        "SELECT * FROM users WHERE id = $1",
        vec![FilterValue::Int(user_id)]
    )
    // Fetch user's posts
    .query(
        "SELECT * FROM posts WHERE author_id = $1 ORDER BY created_at DESC LIMIT 10",
        vec![FilterValue::Int(user_id)]
    )
    // Update last seen
    .execute(
        "UPDATE users SET last_seen = NOW() WHERE id = $1",
        vec![FilterValue::Int(user_id)]
    )
    .build();

// Execute all in one go
let result = engine.execute_pipeline(pipeline).await?;

// Check results
if result.all_succeeded() {
    println!("All queries succeeded");
} else if let Some(err) = result.first_error() {
    eprintln!("Pipeline error: {}", err);
}`;

  planCacheExample = `use prax_query::cache::{ExecutionPlanCache, PlanHint};

// Create cache with max 1000 plans
let cache = ExecutionPlanCache::new(1000);

// Register frequently used queries with hints
cache.register(
    "users_by_email",
    "SELECT * FROM users WHERE email = $1",
    PlanHint::IndexScan("idx_users_email".into()),
);

cache.register(
    "posts_by_author",
    "SELECT * FROM posts WHERE author_id = $1 ORDER BY created_at DESC",
    PlanHint::IndexScan("idx_posts_author_created".into()),
);

cache.register_with_cost(
    "analytics_daily",
    "SELECT date, COUNT(*) FROM events GROUP BY date",
    PlanHint::SeqScan,  // Force sequential scan for analytics
    1500.0,             // Estimated cost
);`;

  planHintsExample = `use prax_query::cache::PlanHint;

// Available plan hints
let hints = vec![
    PlanHint::None,                          // No hint
    PlanHint::IndexScan("idx_name".into()),  // Force index
    PlanHint::SeqScan,                       // Force sequential scan
    PlanHint::Parallel(4),                   // Enable parallel execution
    PlanHint::CachePlan,                     // Cache this plan
    PlanHint::Timeout(Duration::from_secs(30)), // Query timeout
    PlanHint::Custom("pg_hint(...)".into()), // Database-specific hint
];`;

  analysisExample = `// Track execution time
let start = Instant::now();
let result = engine.execute(&sql, &params).await?;
let duration_us = start.elapsed().as_micros() as u64;

// Record for analysis
cache.record_execution("users_by_email", duration_us);

// Later: Find slow queries
let slow = cache.slowest_queries(10);
for plan in slow {
    println!(
        "Query: {} - Avg: {}µs - Used: {} times",
        plan.sql,
        plan.avg_execution_us(),
        plan.use_count(),
    );
}

// Find hot queries for optimization
let hot = cache.most_used(10);`;

  typeLevelFiltersExample = `use prax_query::typed_filter::{And5, Or3, Eq, Gt, Lt};

// Stack-allocated AND filter (no heap allocation)
let filter = And5::new(
    Eq::new("status", "active"),
    Gt::new("age", 18i64),
    Lt::new("score", 1000i64),
    Eq::new("verified", true),
    Eq::new("tier", "premium"),
);

// Or use chained construction
let filter = Eq::new("status", "active")
    .and(Gt::new("age", 18i64))
    .and(Lt::new("score", 1000i64));

// Also available:
// - And3, Or3 for 3 conditions
// - And5, Or5 for 5 conditions
// - Chaining for arbitrary lengths`;

  directSqlTraitExample = `use prax_query::typed_filter::{DirectSql, And5, Eq};

// DirectSql generates SQL without intermediate allocations
pub trait DirectSql {
    fn write_sql(&self, buf: &mut String, param_idx: usize) -> usize;
    fn param_count(&self) -> usize;
}

// Usage
let filter = And5::new(
    Eq::new("id", 42i64),
    Eq::new("age", 18i64),
    Eq::new("active", true),
    Eq::new("score", 100i64),
    Eq::new("status", "approved"),
);

let mut sql = String::with_capacity(256);
let next_param = filter.write_sql(&mut sql, 1);
// sql = "id = $1 AND age = $2 AND active = $3 AND score = $4 AND status = $5"
// next_param = 6`;

  sliceInExample = `use prax_query::typed_filter::{InI64Slice, InStrSlice, DirectSql};

// Zero-allocation IN filter for i64 values
let ids = [1i64, 2, 3, 4, 5, 6, 7, 8, 9, 10];
let filter = InI64Slice::<10>::new("id", &ids);

let mut sql = String::with_capacity(64);
filter.write_sql(&mut sql, 1);
// sql = "id IN ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
// Time: ~3.8ns (uses pre-computed pattern)

// For string values
let statuses = ["active", "pending", "approved"];
let filter = InStrSlice::<3>::new("status", &statuses);

// Pre-computed patterns available for 1-32 elements
// Larger sizes use optimized looped generation`;

  poolWarmupExample = `use prax_postgres::PgPool;

// Create pool
let pool = PgPool::builder()
    .max_size(20)
    .min_idle(5)
    .build(config)?;

// Warmup: pre-establish connections
pool.warmup(5).await?;

// Warmup with prepared statements
pool.warmup_with_statements(&[
    "SELECT * FROM users WHERE id = $1",
    "SELECT * FROM posts WHERE author_id = $1",
    "INSERT INTO events (type, data) VALUES ($1, $2)",
]).await?;

// Now first requests won't have connection/prepare overhead`;

  preparedStmtExample = `// All queries automatically use prepare_cached()
// This means:
// 1. First execution: prepare + execute
// 2. Subsequent: just execute (plan reused)

// The SQL template cache works with prepared statements
use prax_query::cache::{register_global_template, get_global_template};

// Register at startup
register_global_template("users_find", "SELECT * FROM users WHERE id = $1");

// Use in request handler
async fn get_user(pool: &PgPool, id: i64) -> Result<User> {
    let template = get_global_template("users_find").unwrap();

    let conn = pool.get().await?;
    // prepare_cached() reuses the prepared statement
    let stmt = conn.prepare_cached(template.sql()).await?;
    let row = conn.query_one(&stmt, &[&id]).await?;

    User::from_row(&row)
}`;
}














