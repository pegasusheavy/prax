# prax-postgres

PostgreSQL query engine for Prax ORM.

## Overview

`prax-postgres` provides an async PostgreSQL backend using `tokio-postgres` with `deadpool-postgres` connection pooling.

## Features

- Async query execution with Tokio
- Connection pooling via deadpool-postgres
- Transaction support with savepoints
- Prepared statement caching
- SSL/TLS support

## Usage

```rust
use prax_postgres::PostgresEngine;

let engine = PostgresEngine::new("postgresql://user:pass@localhost/db").await?;

// Execute queries through Prax client
let client = PraxClient::with_engine(engine);
let users = client.user().find_many().exec().await?;
```

## Configuration

```rust
use prax_query::connection::PoolConfig;

let config = PoolConfig::new()
    .max_connections(20)
    .min_connections(5)
    .idle_timeout(Duration::from_secs(300));
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

