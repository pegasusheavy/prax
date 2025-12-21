//! Demonstrates the Prax logging functionality.
//!
//! Prax uses JSON logging by default with `info!()` for bootstrap/initialization
//! messages and `debug!()` for operational details.
//!
//! ```bash
//! # Info level (JSON output, default)
//! PRAX_LOG_LEVEL=info cargo run --example logging_demo
//!
//! # Debug level - see all operational details
//! PRAX_DEBUG=true cargo run --example logging_demo
//!
//! # Pretty format for human-readable output
//! PRAX_LOG_FORMAT=pretty PRAX_LOG_LEVEL=info cargo run --example logging_demo
//! ```

use prax_query::{
    cache::QueryCache,
    connection::{ConnectionString, DatabaseConfig, PoolConfig},
    filter::{Filter, FilterValue},
    memory::StringPool,
    sql::{FastSqlBuilder, QueryCapacity},
};
use tracing_subscriber::EnvFilter;

fn main() {
    // Initialize tracing subscriber with JSON format by default
    let level = std::env::var("PRAX_LOG_LEVEL").unwrap_or_else(|_| {
        if std::env::var("PRAX_DEBUG").is_ok() {
            "debug".into()
        } else {
            "info".into()
        }
    });

    let format = std::env::var("PRAX_LOG_FORMAT").unwrap_or_else(|_| "json".into());

    let filter = EnvFilter::try_new(format!(
        "prax={},prax_query={},prax_schema={}",
        level, level, level
    ))
    .unwrap_or_else(|_| EnvFilter::new("warn"));

    match format.as_str() {
        "pretty" => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_target(true)
                .pretty()
                .init();
        }
        "compact" => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_target(true)
                .compact()
                .init();
        }
        _ => {
            // JSON is the default
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_target(true)
                .json()
                .init();
        }
    }

    println!("=== Prax Logging Demo ===\n");

    // 1. Bootstrap: Pool Configuration (info! logging)
    println!("1. Creating pool configurations (info! logged)...");
    let _pool_dev = PoolConfig::development();
    let _pool_read = PoolConfig::read_heavy();
    let _pool_server = PoolConfig::serverless();
    println!("   Pool configurations created");

    // 2. Bootstrap: Database config from URL (info! logged)
    println!("\n2. Loading database configuration (info! logged)...");
    let _config = DatabaseConfig::from_url("postgres://user:pass@localhost:5432/mydb").unwrap();
    println!("   Database config loaded");

    // 3. Bootstrap: Query cache (info! logged)
    println!("\n3. Creating query cache (info! logged)...");
    let cache = QueryCache::new(100);
    println!("   Query cache created");

    // 4. Bootstrap: String pool (info! logged)
    println!("\n4. Creating string pool with capacity (info! logged)...");
    let _pool = StringPool::with_capacity(1000);
    println!("   String pool created");

    // 5. Operations: Filter construction (debug! logged)
    println!("\n5. Creating filters (debug! logged)...");
    let filter = Filter::and([
        Filter::Equals("id".into(), FilterValue::Int(1)),
        Filter::Equals("name".into(), FilterValue::String("John".into())),
        Filter::Gt("age".into(), FilterValue::Int(18)),
    ]);
    println!(
        "   Filter created: {} conditions",
        if let Filter::And(filters) = &filter {
            filters.len()
        } else {
            1
        }
    );

    // 6. Operations: SQL building (debug! logged)
    println!("\n6. Building SQL (debug! logged)...");
    let mut builder = FastSqlBuilder::postgres(QueryCapacity::SimpleSelect);
    builder.push_str("SELECT * FROM users WHERE ");
    builder.push_identifier("id");
    builder.push_str(" = ");
    builder.bind(42i64);
    let (sql, params) = builder.build();
    println!("   SQL: {}", sql);
    println!("   Params: {} values", params.len());

    // 7. Operations: Connection parsing (debug! logged)
    println!("\n7. Parsing connection string (debug! logged)...");
    let conn = ConnectionString::parse("postgres://user:pass@localhost:5432/mydb").unwrap();
    println!(
        "   Parsed: {} @ {}",
        conn.driver(),
        conn.host().unwrap_or("unknown")
    );

    // 8. Operations: Cache operations (debug! logged)
    println!("\n8. Cache operations (debug! logged)...");
    cache.insert("users_by_id", "SELECT * FROM users WHERE id = $1");
    let cached = cache.get("users_by_id");
    println!("   Cache hit: {}", cached.is_some());
    let missed = cache.get("nonexistent");
    println!("   Cache miss: {}", missed.is_none());

    println!("\n=== Demo Complete ===");
    println!("\nLog level examples (JSON output by default):");
    println!(
        "  PRAX_LOG_LEVEL=info cargo run --example logging_demo    # Bootstrap messages (JSON)"
    );
    println!("  PRAX_DEBUG=true cargo run --example logging_demo        # Debug details (JSON)");
    println!("\nLog format examples:");
    println!(
        "  PRAX_LOG_FORMAT=pretty PRAX_LOG_LEVEL=info cargo run --example logging_demo  # Human-readable"
    );
    println!(
        "  PRAX_LOG_FORMAT=compact PRAX_LOG_LEVEL=info cargo run --example logging_demo # Compact format"
    );
}
