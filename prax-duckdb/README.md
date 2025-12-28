# prax-duckdb

DuckDB database driver for the Prax ORM, optimized for analytical workloads (OLAP).

## Features

- **In-process analytics**: No server required, runs embedded in your application
- **Columnar storage**: Optimized for analytical queries with fast aggregations
- **Parquet support**: Native reading/writing of Parquet files
- **JSON support**: Query JSON data directly
- **CSV support**: Import and export CSV files
- **SQL compatibility**: Full SQL support with analytical extensions
- **Async support**: Async operations via Tokio task spawning
- **Connection pooling**: Efficient connection management for concurrent access

## When to Use DuckDB

DuckDB excels at:

- **Analytical queries**: Aggregations, joins, window functions
- **Data transformation**: ETL pipelines and data processing
- **File-based querying**: Direct queries on Parquet, CSV, and JSON files
- **Embedded analytics**: Adding analytics to applications without a separate database server

For OLTP workloads (many small transactions), consider PostgreSQL or SQLite instead.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
prax-duckdb = "0.3"
```

## Quick Start

```rust
use prax_duckdb::{DuckDbPool, DuckDbConfig, DuckDbEngine};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // In-memory database
    let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await?;
    let engine = DuckDbEngine::new(pool);

    // Create a table
    engine.raw_sql_batch(r#"
        CREATE TABLE sales (
            date DATE,
            region VARCHAR,
            revenue DECIMAL(10,2)
        );
        INSERT INTO sales VALUES
            ('2024-01-01', 'North', 1000.00),
            ('2024-01-01', 'South', 1500.00),
            ('2024-01-02', 'North', 1200.00);
    "#).await?;

    // Query with aggregation
    let results = engine.execute_raw(
        "SELECT region, SUM(revenue) as total FROM sales GROUP BY region",
        &[]
    ).await?;

    for result in results {
        println!("{}", result.json());
    }

    Ok(())
}
```

## Configuration

### In-Memory Database

```rust
let config = DuckDbConfig::in_memory();
```

### File-Based Database

```rust
let config = DuckDbConfig::from_path("./analytics.duckdb")?;
```

### From URL

```rust
let config = DuckDbConfig::from_url(
    "duckdb:///path/to/db.duckdb?threads=4&memory_limit=4GB"
)?;
```

### Builder Pattern

```rust
let config = DuckDbConfig::builder()
    .path("./analytics.duckdb")
    .threads(8)
    .memory_limit("8GB")
    .read_only()
    .build();
```

## Analytical Features

### Window Functions

```rust
let sql = r#"
    SELECT
        date,
        revenue,
        SUM(revenue) OVER (
            PARTITION BY region
            ORDER BY date
            ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
        ) as cumulative_revenue,
        AVG(revenue) OVER (
            PARTITION BY region
            ORDER BY date
            ROWS BETWEEN 6 PRECEDING AND CURRENT ROW
        ) as rolling_avg
    FROM sales
"#;

let results = engine.execute_raw(sql, &[]).await?;
```

### Parquet Files

```rust
// Query Parquet files directly
let results = engine.query_parquet("./data/*.parquet").await?;

// Export to Parquet
engine.copy_to_parquet(
    "SELECT * FROM sales WHERE date >= '2024-01-01'",
    "./export.parquet"
).await?;
```

### CSV Files

```rust
// Query CSV files
let results = engine.query_csv("./data.csv", true).await?; // true = has header

// Export to CSV
engine.copy_to_csv(
    "SELECT * FROM sales",
    "./export.csv",
    true // include header
).await?;
```

### JSON Files

```rust
// Query JSON files
let results = engine.query_json("./data.json").await?;
```

## Connection Pooling

```rust
let pool = DuckDbPool::builder()
    .in_memory()
    .max_connections(10)
    .min_connections(2)
    .build()
    .await?;

// Get a connection
let conn = pool.get().await?;

// Connection is automatically returned to pool when dropped
```

## Transactions

```rust
let conn = pool.get().await?;

// Manual transaction management
conn.execute_batch("BEGIN TRANSACTION").await?;
conn.execute("INSERT INTO table VALUES (?)", &[value]).await?;
conn.execute_batch("COMMIT").await?;

// Or use savepoints
conn.execute_batch("SAVEPOINT sp1").await?;
// ... operations ...
conn.execute_batch("RELEASE SAVEPOINT sp1").await?;
```

## Error Handling

```rust
use prax_duckdb::{DuckDbError, DuckDbResult};

fn handle_error(result: DuckDbResult<()>) {
    match result {
        Ok(_) => println!("Success"),
        Err(DuckDbError::Query(msg)) => println!("Query error: {}", msg),
        Err(DuckDbError::Connection(msg)) => println!("Connection error: {}", msg),
        Err(DuckDbError::Parquet(msg)) => println!("Parquet error: {}", msg),
        Err(e) => println!("Other error: {}", e),
    }
}
```

## Features

| Feature | Description |
|---------|-------------|
| `bundled` | Bundle DuckDB library (default) |
| `json` | JSON extension support |
| `parquet` | Parquet file support |
| `chrono` | Chrono date/time support |
| `serde_json` | Serde JSON support |
| `uuid` | UUID support |
| `extensions-full` | All extensions (json, parquet, etc.) |

## Performance Tips

1. **Use Parquet for large datasets**: Columnar format is much faster for analytical queries
2. **Limit memory for large queries**: Set `memory_limit` to prevent OOM
3. **Use appropriate thread count**: DuckDB can parallelize queries across threads
4. **Batch operations**: Use multi-row inserts and batch statements
5. **Avoid SELECT ***: Only select columns you need

## License

MIT OR Apache-2.0

