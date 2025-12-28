# prax-scylladb

ScyllaDB database driver for Prax ORM - high-performance Cassandra-compatible database.

## Overview

ScyllaDB is a drop-in replacement for Apache Cassandra that delivers significantly better performance through a modern C++ implementation. This crate provides async Rust support for ScyllaDB within the Prax ORM ecosystem.

## Features

- **High Performance**: Built on the official `scylla` async driver
- **Connection Pooling**: Automatic connection management with the ScyllaDB driver's built-in pooling
- **Prepared Statements**: Efficient query execution with automatic caching
- **Async/Await**: Full async support with Tokio runtime
- **Type Safety**: Strong typing with automatic CQL type conversions
- **Lightweight Transactions**: Support for conditional updates (LWT)
- **Batch Operations**: Logged, unlogged, and counter batches

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
prax-scylladb = "0.3"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use prax_scylladb::{ScyllaConfig, ScyllaPool, ScyllaEngine};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect using URL
    let pool = ScyllaPool::from_url("scylla://localhost:9042/my_keyspace").await?;

    // Or use the builder
    let config = ScyllaConfig::builder()
        .known_nodes(["node1:9042", "node2:9042", "node3:9042"])
        .default_keyspace("my_keyspace")
        .username("cassandra")
        .password("cassandra")
        .consistency(prax_scylladb::config::ConsistencyLevel::LocalQuorum)
        .build();

    let pool = ScyllaPool::connect(config).await?;

    // Create engine for queries
    let engine = pool.engine();

    // Insert data
    engine.insert(
        "users",
        &["id", "email", "name"],
        (uuid::Uuid::new_v4(), "alice@example.com", "Alice"),
    ).await?;

    // Query data
    let users: Vec<User> = engine
        .query("SELECT * FROM users WHERE email = ?", ("alice@example.com",))
        .await?;

    Ok(())
}
```

## Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `known_nodes` | Cluster node addresses | `["127.0.0.1:9042"]` |
| `default_keyspace` | Default keyspace to use | None |
| `username` | Authentication username | None |
| `password` | Authentication password | None |
| `connection_timeout_secs` | Connection timeout | 5 |
| `request_timeout_secs` | Request timeout | 12 |
| `pool_size` | Connections per node | 4 |
| `local_datacenter` | Preferred datacenter | None |
| `ssl_enabled` | Enable SSL/TLS | false |
| `compression` | Compression (lz4, snappy) | None |
| `consistency` | Default consistency level | Quorum |

## URL Format

```
scylla://[user:pass@]host1:port1[,host2:port2,...]/keyspace[?options]
```

Options:
- `timeout=30` - Request timeout in seconds
- `pool_size=8` - Connections per node
- `datacenter=dc1` - Preferred datacenter
- `ssl=true` - Enable SSL
- `compression=lz4` - Enable compression
- `consistency=LOCAL_QUORUM` - Default consistency

## Consistency Levels

| Level | Description |
|-------|-------------|
| `Any` | Any node (writes only) |
| `One` | Single node |
| `Two` | Two nodes |
| `Three` | Three nodes |
| `Quorum` | Majority of replicas |
| `All` | All replicas |
| `LocalQuorum` | Quorum in local datacenter |
| `EachQuorum` | Quorum in each datacenter |
| `LocalOne` | One node in local datacenter |

## Batch Operations

```rust
// Logged batch (atomic)
engine.batch()
    .logged()
    .add("INSERT INTO users (id, email) VALUES (?, ?)")
    .add("INSERT INTO user_by_email (email, id) VALUES (?, ?)")
    .execute_with_values((
        (user_id, email),
        (email, user_id),
    ))
    .await?;

// Unlogged batch (faster, non-atomic)
engine.batch()
    .unlogged()
    .add("UPDATE stats SET count = count + 1 WHERE id = ?")
    .add("UPDATE stats SET count = count + 1 WHERE id = ?")
    .execute()
    .await?;
```

## Lightweight Transactions (LWT)

```rust
// Insert if not exists
let applied = engine.insert_if_not_exists(
    "users",
    &["id", "email"],
    (user_id, email),
).await?;

if !applied {
    println!("User already exists!");
}

// Conditional update
let applied = engine.update_if(
    "users",
    "email = ?",
    "id = ?",
    "email = ?",
    (new_email, user_id, old_email),
).await?;
```

## Type Mapping

| CQL Type | Rust Type |
|----------|-----------|
| `boolean` | `bool` |
| `tinyint` | `i8` |
| `smallint` | `i16` |
| `int` | `i32` |
| `bigint` | `i64` |
| `float` | `f32` |
| `double` | `f64` |
| `text`, `varchar` | `String` |
| `blob` | `Vec<u8>` |
| `uuid`, `timeuuid` | `uuid::Uuid` |
| `timestamp` | `chrono::DateTime<Utc>` |
| `date` | `chrono::NaiveDate` |
| `inet` | `std::net::IpAddr` |
| `list<T>` | `Vec<T>` |
| `set<T>` | `Vec<T>` |
| `map<K,V>` | `Vec<(K, V)>` |

## Features

- `ssl` - Enable SSL/TLS support
- `cloud` - Enable ScyllaDB Cloud support

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

