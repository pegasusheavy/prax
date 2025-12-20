# Prax SQLx Backend

SQLx-based query engine backend for Prax ORM with compile-time checked queries.

## Features

- **Compile-time query checking** - Validate SQL queries at compile time using SQLx macros
- **Multi-database support** - PostgreSQL, MySQL, and SQLite through a unified API
- **Type-safe queries** - Strong typing for query parameters and results
- **Connection pooling** - Built-in connection pool management via SQLx
- **Async/await** - Full async support with tokio runtime

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
prax-sqlx = { version = "0.1", features = ["postgres"] }
```

### Available Features

- `postgres` - PostgreSQL support (default)
- `mysql` - MySQL/MariaDB support
- `sqlite` - SQLite support
- `all-databases` - Enable all database backends

## Usage

### Basic Setup

```rust
use prax_sqlx::{SqlxEngine, SqlxConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration from URL
    let config = SqlxConfig::from_url("postgres://user:pass@localhost/mydb")?;

    // Or use the builder for more options
    let config = SqlxConfig::builder("postgres://localhost/mydb")
        .max_connections(20)
        .min_connections(5)
        .connect_timeout(std::time::Duration::from_secs(10))
        .build();

    // Create engine
    let engine = SqlxEngine::new(config).await?;

    Ok(())
}
```

### Raw Queries

```rust
use prax_query::filter::FilterValue;

// Execute a query and get rows
let rows = engine.raw_query_many(
    "SELECT * FROM users WHERE active = $1",
    &[FilterValue::Bool(true)]
).await?;

// Execute a query expecting one result
let row = engine.raw_query_one(
    "SELECT * FROM users WHERE id = $1",
    &[FilterValue::Int(1)]
).await?;

// Execute an INSERT/UPDATE/DELETE
let affected = engine.raw_execute(
    "UPDATE users SET active = $1 WHERE id = $2",
    &[FilterValue::Bool(false), FilterValue::Int(1)]
).await?;

// Count rows
let count = engine.count_table("users", Some("active = true")).await?;
```

### Compile-Time Checked Queries

Use SQLx's query macros for compile-time SQL verification:

```rust
use prax_sqlx::checked;

// The query! macro validates SQL at compile time
let users = checked::query_as!(
    User,
    "SELECT id, name, email FROM users WHERE id = $1",
    user_id
)
.fetch_all(engine.pool().as_postgres().unwrap())
.await?;
```

### Database-Specific Features

#### PostgreSQL

```rust
use prax_sqlx::postgres::{PgHelpers, AdvisoryLock};

// Generate upsert SQL
let sql = PgHelpers::upsert(
    pool,
    "users",
    &["id", "name", "email"],
    &["id"],
    &["name", "email"]
).await?;

// Use advisory locks
AdvisoryLock::acquire(pool, 12345).await?;
// ... do work ...
AdvisoryLock::release(pool, 12345).await?;

// Check PostgreSQL version
let version = PgHelpers::version(pool).await?;
```

#### MySQL

```rust
use prax_sqlx::mysql::{MySqlHelpers, MySqlLock};

// Generate upsert SQL
let sql = MySqlHelpers::upsert_sql("users", &["id", "name"], &["name"]);

// Use named locks
MySqlLock::get_lock(pool, "my_lock", 10).await?;
MySqlLock::release_lock(pool, "my_lock").await?;

// Get last insert ID
let id = MySqlHelpers::last_insert_id(pool).await?;
```

#### SQLite

```rust
use prax_sqlx::sqlite::{SqliteHelpers, JournalMode, SynchronousMode};

// Enable foreign keys
SqliteHelpers::enable_foreign_keys(pool).await?;

// Set WAL mode for better concurrency
SqliteHelpers::set_journal_mode(pool, JournalMode::Wal).await?;

// Vacuum database
SqliteHelpers::vacuum(pool).await?;

// Check integrity
let results = SqliteHelpers::integrity_check(pool).await?;
```

### Transactions

```rust
use prax_sqlx::pool::SqlxPool;

// Begin a transaction
let tx = engine.pool().begin().await?;

// Execute queries within the transaction
// ...

// Commit
tx.commit().await?;

// Or rollback
// tx.rollback().await?;
```

### Using with Prax QueryEngine

The `SqlxEngine` implements the `QueryEngine` trait, so it can be used with Prax's query builder:

```rust
use prax_query::traits::QueryEngine;

// The engine implements QueryEngine trait
let results = engine.query_many::<User>(
    "SELECT * FROM users",
    vec![]
).await?;
```

## Connection Pool Options

```rust
let config = SqlxConfig::builder("postgres://localhost/mydb")
    // Pool size
    .max_connections(10)
    .min_connections(1)

    // Timeouts
    .connect_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))

    // SSL (PostgreSQL)
    .ssl_mode(SslMode::Require)

    // Application name (PostgreSQL)
    .application_name("my-app")

    .build();
```

## Error Handling

```rust
use prax_sqlx::SqlxError;

match engine.raw_query_one("SELECT * FROM users WHERE id = $1", &[FilterValue::Int(1)]).await {
    Ok(row) => { /* process row */ }
    Err(SqlxError::Sqlx(e)) => { /* handle SQLx error */ }
    Err(SqlxError::Connection(msg)) => { /* handle connection error */ }
    Err(SqlxError::Timeout(ms)) => { /* handle timeout */ }
    Err(e) => { /* handle other errors */ }
}
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.


