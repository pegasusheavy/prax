# prax-query

Type-safe query builder for Prax ORM.

## Overview

`prax-query` provides a fluent, type-safe API for building database queries with support for filtering, pagination, relations, and transactions.

## Features

- Fluent query API: `findMany`, `findUnique`, `create`, `update`, `delete`, `upsert`
- Type-safe filtering with AND/OR/NOT combinators
- Pagination with `skip`/`take` and cursor-based navigation
- Aggregation queries: `count`, `sum`, `avg`, `min`, `max`, `groupBy`
- Raw SQL escape hatch with type interpolation
- Connection pooling and configuration
- Middleware/hooks system for query interception
- Multi-tenant support

## Performance

- **1.7ns** simple filter creation (3x faster than Diesel)
- **64 bytes** Filter enum size (fits in single cache line)
- **6.6x faster** SQL string construction vs Diesel

## Usage

```rust
use prax_query::prelude::*;

// Find users with filters
let users = client
    .user()
    .find_many()
    .where(user::email::contains("@example.com"))
    .order_by(user::created_at::desc())
    .take(10)
    .exec()
    .await?;

// Create with nested relations
let user = client
    .user()
    .create(user::Create {
        email: "alice@example.com".into(),
        name: Some("Alice".into()),
        ..Default::default()
    })
    .exec()
    .await?;
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

