import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-database-duckdb-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './database-duckdb.page.html',
})
export class DatabaseDuckdbPage {
  configCode = `[database]
provider = "duckdb"
url = "duckdb:///analytics.duckdb"

# Or in-memory
# url = "duckdb://:memory:"

# With options
# url = "duckdb://:memory:?threads=4&memory_limit=4GB"`;

  basicCode = `use prax_duckdb::{DuckDbPool, DuckDbConfig, DuckDbEngine};

// In-memory database
let pool = DuckDbPool::new(DuckDbConfig::in_memory()).await?;
let engine = DuckDbEngine::new(pool);

// Or file-based
let config = DuckDbConfig::from_path("./analytics.duckdb")?;
let pool = DuckDbPool::new(config).await?;`;

  analyticsCode = `// Analytical query with window function
let sql = r#"
    SELECT
        date,
        revenue,
        SUM(revenue) OVER (
            PARTITION BY region
            ORDER BY date
        ) as cumulative_revenue
    FROM sales
"#;
let results = engine.execute_raw(sql, &[]).await?;

// Aggregation
let results = engine.execute_raw(
    "SELECT region, SUM(revenue) as total FROM sales GROUP BY region",
    &[]
).await?;`;

  parquetCode = `// Query Parquet files directly
let results = engine.query_parquet("./data/*.parquet").await?;

// Export to Parquet
engine.copy_to_parquet(
    "SELECT * FROM sales WHERE year = 2024",
    "./export.parquet"
).await?;

// CSV export
engine.copy_to_csv("SELECT * FROM users", "./users.csv", true).await?;

// JSON file query
let results = engine.query_json("./data.json").await?;`;

  poolCode = `use prax_duckdb::{DuckDbPool, DuckDbConfig};

let pool = DuckDbPool::builder()
    .in_memory()
    .max_connections(10)
    .min_connections(2)
    .build()
    .await?;

// Get a pooled connection
let conn = pool.get().await?;
let results = conn.query("SELECT * FROM analytics", &[]).await?;
// Connection returned to pool on drop`;
}

