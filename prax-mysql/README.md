# prax-mysql

MySQL query engine for Prax ORM.

## Overview

`prax-mysql` provides an async MySQL backend using the `mysql_async` driver.

## Features

- Async query execution with Tokio
- Connection pooling
- Transaction support
- Prepared statement caching
- SSL/TLS support

## Usage

```rust
use prax_mysql::{MysqlEngine, MysqlPool};
use prax_orm::PraxClient;

let pool = MysqlPool::builder()
    .url("mysql://user:pass@localhost/db")
    .build()
    .await?;

// Execute queries through Prax client
let client = PraxClient::new(MysqlEngine::new(pool));
let users = client.user().find_many().exec().await?;
```

## Configuration

```rust
use prax_query::connection::PoolConfig;

let config = PoolConfig::new()
    .max_connections(20)
    .min_connections(5);
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

