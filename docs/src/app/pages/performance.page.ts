import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-performance-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './performance.page.html',
})
export class PerformancePage {
  directSqlExample = `use prax_query::typed_filter::{And5, Eq, DirectSql};

// Create type-level filters (stack allocated, ~5ns)
let filter = And5::new(
    Eq::new("id", 42i64),
    Eq::new("active", true),
    Eq::new("age", 18i64),
    Eq::new("score", 100i64),
    Eq::new("status", "approved"),
);

// Generate SQL with zero allocations (~17ns)
let mut sql = String::with_capacity(256);
filter.write_sql(&mut sql, 1);
// sql = "id = $1 AND active = $2 AND age = $3 AND score = $4 AND status = $5"`;

  placeholderExample = `// Pre-computed placeholders for PostgreSQL (256 entries)
pub static POSTGRES_PLACEHOLDERS: &[&str] = &[
    "$1", "$2", "$3", "$4", "$5", // ... up to $256
];

// Pre-computed IN patterns (1-32 elements)
pub const POSTGRES_IN_FROM_1: &[&str] = &[
    "",          // 0 (empty)
    "$1",        // 1
    "$1, $2",    // 2
    "$1, $2, $3", // ... up to 32
];

// Zero-cost lookup: 3.8ns for IN(10)
let placeholder = POSTGRES_IN_FROM_1[10]; // "$1, $2, ... $10"`;

  planCacheExample = `use prax_query::cache::{ExecutionPlanCache, PlanHint};

// Create a plan cache
let cache = ExecutionPlanCache::new(1000);

// Register with execution hints
let plan = cache.register(
    "users_by_email",
    "SELECT * FROM users WHERE email = $1",
    PlanHint::IndexScan("idx_users_email".into()),
);

// Track execution timing automatically
cache.record_execution("users_by_email", duration_us);

// Find slow queries for optimization
let slow_queries = cache.slowest_queries(10);`;

  zeroCopyExample = `use prax_query::row::{RowRef, FromRowRef, RowError};

// Zero-copy struct borrows from row
struct UserRef<'a> {
    id: i32,
    email: &'a str,  // Borrowed - no allocation!
    name: Option<&'a str>,
}

impl<'a> FromRowRef<'a> for UserRef<'a> {
    fn from_row_ref(row: &'a impl RowRef) -> Result<Self, RowError> {
        Ok(Self {
            id: row.get_i32("id")?,
            email: row.get_str("email")?,  // Zero-copy
            name: row.get_str_opt("name")?,
        })
    }
}`;

  pipelineExample = `use prax_query::batch::{PipelineBuilder, Pipeline};

// Build a query pipeline
let pipeline = PipelineBuilder::new()
    .query("SELECT * FROM users WHERE id = $1", vec![user_id.into()])
    .query("SELECT * FROM posts WHERE author_id = $1", vec![user_id.into()])
    .execute("UPDATE users SET last_seen = NOW() WHERE id = $1", vec![user_id.into()])
    .build();

// Execute all queries in minimal round-trips
let results = engine.execute_pipeline(pipeline).await?;

// Also: Batch combines multiple INSERTs into one statement
let batch = BatchBuilder::new()
    .insert("users", user1_data)
    .insert("users", user2_data)
    .insert("users", user3_data)
    .build();

let (sql, params) = batch.to_combined_sql(DatabaseType::PostgreSQL).unwrap();
// INSERT INTO users (name, email) VALUES ($1, $2), ($3, $4), ($5, $6)`;

  typeLevelExample = `use prax_query::typed_filter::{And5, Eq, TypedFilter};

// Type-level filter composition (~5.1ns - matches Diesel!)
let filter = And5::new(
    Eq::new("id", 42i64),
    Eq::new("age", 18i64),
    Eq::new("active", true),
    Eq::new("score", 100i64),
    Eq::new("status", "approved"),
);

// Or use chained construction (~5.2ns)
let filter = Eq::new("id", 42i64)
    .and(Eq::new("age", 18i64))
    .and(Eq::new("active", true))
    .and(Eq::new("score", 100i64))
    .and(Eq::new("status", "approved"));

// Also available: And3, Or5, Or3 for common sizes
// Plus: InI64Slice, InStrSlice for zero-allocation IN clauses`;
}
