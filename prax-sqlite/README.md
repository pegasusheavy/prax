# prax-sqlite

SQLite query engine for Prax ORM.

## Overview

`prax-sqlite` provides an async SQLite backend using `tokio-rusqlite`.

## Features

- Async query execution with Tokio
- Connection pooling with reuse optimization
- WAL mode support for concurrent reads
- In-memory database support
- Transaction support

## Usage

```rust
use prax_sqlite::SqliteEngine;

// File-based database
let engine = SqliteEngine::new("sqlite:./data.db").await?;

// In-memory database
let engine = SqliteEngine::new("sqlite::memory:").await?;

// Execute queries through Prax client
let client = PraxClient::with_engine(engine);
let users = client.user().find_many().exec().await?;
```

## Performance

SQLite operations are highly optimized:
- **~145ns** connection acquisition (with pooling)
- WAL mode for concurrent read/write

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

