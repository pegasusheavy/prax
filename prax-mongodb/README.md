# prax-mongodb

MongoDB query engine for Prax ORM.

## Overview

`prax-mongodb` provides an async MongoDB backend using the official `mongodb` Rust driver with built-in connection pooling.

## Features

- Async query execution with Tokio
- Built-in connection pooling
- Document serialization via BSON/serde
- Filter builder for type-safe queries
- Aggregation pipeline support
- Change streams support
- Transactions and sessions

## Usage

```rust
use prax_mongodb::{MongoClient, FilterBuilder, doc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = MongoClient::builder()
        .uri("mongodb://localhost:27017")
        .database("mydb")
        .max_pool_size(10)
        .build()
        .await?;

    // Get a typed collection
    let users = client.collection::<User>("users");

    // Insert a document
    users.insert_one(User { name: "Alice".into(), age: 30 }).await?;

    // Query with filter builder
    let filter = FilterBuilder::new()
        .eq("status", "active")
        .gte("age", 18)
        .build();

    let active_adults: Vec<User> = users
        .find(filter)
        .await?
        .try_collect()
        .await?;

    Ok(())
}
```

## Document Mapping

Models are mapped to MongoDB documents using serde:

```rust
use serde::{Deserialize, Serialize};
use prax_mongodb::ObjectId;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
    email: String,
    #[serde(default)]
    tags: Vec<String>,
}
```

## Filter Builder

Build type-safe MongoDB queries:

```rust
use prax_mongodb::FilterBuilder;

// Simple equality
let filter = FilterBuilder::new()
    .eq("status", "active")
    .build();

// Comparisons
let filter = FilterBuilder::new()
    .gte("age", 18)
    .lt("age", 65)
    .build();

// Array operations
let filter = FilterBuilder::new()
    .in_array("status", vec!["active", "pending"])
    .all("tags", vec!["rust", "async"])
    .build();

// Text search
let filter = FilterBuilder::new()
    .text_search("hello world")
    .build();

// Logical operators
let filter = FilterBuilder::new()
    .or(vec![
        doc! { "priority": "high" },
        doc! { "urgent": true },
    ])
    .build();

// Geospatial
let filter = FilterBuilder::new()
    .near("location", -73.97, 40.77, Some(1000.0))
    .build();
```

## Aggregation Pipelines

Build aggregation pipelines with helpers:

```rust
use prax_mongodb::types::aggregation::*;

let pipeline = vec![
    match_stage(doc! { "status": "completed" }),
    group_stage(
        "$category".into(),
        doc! {
            "total": sum("$amount"),
            "count": sum(1),
            "avg": avg("$amount"),
        },
    ),
    sort_stage(doc! { "total": -1 }),
    limit_stage(10),
];

let results = collection.aggregate(pipeline).await?;
```

## Configuration

```rust
use prax_mongodb::{MongoConfig, ReadPreference, WriteConcern};
use std::time::Duration;

let config = MongoConfig::builder()
    .uri("mongodb://localhost:27017,localhost:27018,localhost:27019/?replicaSet=rs0")
    .database("mydb")
    .app_name("my-service")
    .max_pool_size(100)
    .min_pool_size(10)
    .connect_timeout(Duration::from_secs(10))
    .read_preference(ReadPreference::SecondaryPreferred)
    .write_concern(WriteConcern::Majority)
    .retry_writes(true)
    .compressors(vec!["zstd".into()])
    .build()?;
```

## MongoDB Atlas

For MongoDB Atlas, use the connection string from Atlas:

```rust
let client = MongoClient::builder()
    .uri("mongodb+srv://user:pass@cluster0.xxxxx.mongodb.net/?retryWrites=true&w=majority")
    .database("mydb")
    .build()
    .await?;
```

## Transactions

Use sessions for multi-document transactions:

```rust
let mut session = client.start_session().await?;

session.start_transaction(None).await?;

// Perform operations within transaction
users.insert_one_with_session(user, &mut session).await?;
orders.insert_one_with_session(order, &mut session).await?;

session.commit_transaction().await?;
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.





MongoDB query engine for Prax ORM.

## Overview

`prax-mongodb` provides an async MongoDB backend using the official `mongodb` Rust driver with built-in connection pooling.

## Features

- Async query execution with Tokio
- Built-in connection pooling
- Document serialization via BSON/serde
- Filter builder for type-safe queries
- Aggregation pipeline support
- Change streams support
- Transactions and sessions

## Usage

```rust
use prax_mongodb::{MongoClient, FilterBuilder, doc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = MongoClient::builder()
        .uri("mongodb://localhost:27017")
        .database("mydb")
        .max_pool_size(10)
        .build()
        .await?;

    // Get a typed collection
    let users = client.collection::<User>("users");

    // Insert a document
    users.insert_one(User { name: "Alice".into(), age: 30 }).await?;

    // Query with filter builder
    let filter = FilterBuilder::new()
        .eq("status", "active")
        .gte("age", 18)
        .build();

    let active_adults: Vec<User> = users
        .find(filter)
        .await?
        .try_collect()
        .await?;

    Ok(())
}
```

## Document Mapping

Models are mapped to MongoDB documents using serde:

```rust
use serde::{Deserialize, Serialize};
use prax_mongodb::ObjectId;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
    email: String,
    #[serde(default)]
    tags: Vec<String>,
}
```

## Filter Builder

Build type-safe MongoDB queries:

```rust
use prax_mongodb::FilterBuilder;

// Simple equality
let filter = FilterBuilder::new()
    .eq("status", "active")
    .build();

// Comparisons
let filter = FilterBuilder::new()
    .gte("age", 18)
    .lt("age", 65)
    .build();

// Array operations
let filter = FilterBuilder::new()
    .in_array("status", vec!["active", "pending"])
    .all("tags", vec!["rust", "async"])
    .build();

// Text search
let filter = FilterBuilder::new()
    .text_search("hello world")
    .build();

// Logical operators
let filter = FilterBuilder::new()
    .or(vec![
        doc! { "priority": "high" },
        doc! { "urgent": true },
    ])
    .build();

// Geospatial
let filter = FilterBuilder::new()
    .near("location", -73.97, 40.77, Some(1000.0))
    .build();
```

## Aggregation Pipelines

Build aggregation pipelines with helpers:

```rust
use prax_mongodb::types::aggregation::*;

let pipeline = vec![
    match_stage(doc! { "status": "completed" }),
    group_stage(
        "$category".into(),
        doc! {
            "total": sum("$amount"),
            "count": sum(1),
            "avg": avg("$amount"),
        },
    ),
    sort_stage(doc! { "total": -1 }),
    limit_stage(10),
];

let results = collection.aggregate(pipeline).await?;
```

## Configuration

```rust
use prax_mongodb::{MongoConfig, ReadPreference, WriteConcern};
use std::time::Duration;

let config = MongoConfig::builder()
    .uri("mongodb://localhost:27017,localhost:27018,localhost:27019/?replicaSet=rs0")
    .database("mydb")
    .app_name("my-service")
    .max_pool_size(100)
    .min_pool_size(10)
    .connect_timeout(Duration::from_secs(10))
    .read_preference(ReadPreference::SecondaryPreferred)
    .write_concern(WriteConcern::Majority)
    .retry_writes(true)
    .compressors(vec!["zstd".into()])
    .build()?;
```

## MongoDB Atlas

For MongoDB Atlas, use the connection string from Atlas:

```rust
let client = MongoClient::builder()
    .uri("mongodb+srv://user:pass@cluster0.xxxxx.mongodb.net/?retryWrites=true&w=majority")
    .database("mydb")
    .build()
    .await?;
```

## Transactions

Use sessions for multi-document transactions:

```rust
let mut session = client.start_session().await?;

session.start_transaction(None).await?;

// Perform operations within transaction
users.insert_one_with_session(user, &mut session).await?;
orders.insert_one_with_session(order, &mut session).await?;

session.commit_transaction().await?;
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.






MongoDB query engine for Prax ORM.

## Overview

`prax-mongodb` provides an async MongoDB backend using the official `mongodb` Rust driver with built-in connection pooling.

## Features

- Async query execution with Tokio
- Built-in connection pooling
- Document serialization via BSON/serde
- Filter builder for type-safe queries
- Aggregation pipeline support
- Change streams support
- Transactions and sessions

## Usage

```rust
use prax_mongodb::{MongoClient, FilterBuilder, doc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = MongoClient::builder()
        .uri("mongodb://localhost:27017")
        .database("mydb")
        .max_pool_size(10)
        .build()
        .await?;

    // Get a typed collection
    let users = client.collection::<User>("users");

    // Insert a document
    users.insert_one(User { name: "Alice".into(), age: 30 }).await?;

    // Query with filter builder
    let filter = FilterBuilder::new()
        .eq("status", "active")
        .gte("age", 18)
        .build();

    let active_adults: Vec<User> = users
        .find(filter)
        .await?
        .try_collect()
        .await?;

    Ok(())
}
```

## Document Mapping

Models are mapped to MongoDB documents using serde:

```rust
use serde::{Deserialize, Serialize};
use prax_mongodb::ObjectId;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
    email: String,
    #[serde(default)]
    tags: Vec<String>,
}
```

## Filter Builder

Build type-safe MongoDB queries:

```rust
use prax_mongodb::FilterBuilder;

// Simple equality
let filter = FilterBuilder::new()
    .eq("status", "active")
    .build();

// Comparisons
let filter = FilterBuilder::new()
    .gte("age", 18)
    .lt("age", 65)
    .build();

// Array operations
let filter = FilterBuilder::new()
    .in_array("status", vec!["active", "pending"])
    .all("tags", vec!["rust", "async"])
    .build();

// Text search
let filter = FilterBuilder::new()
    .text_search("hello world")
    .build();

// Logical operators
let filter = FilterBuilder::new()
    .or(vec![
        doc! { "priority": "high" },
        doc! { "urgent": true },
    ])
    .build();

// Geospatial
let filter = FilterBuilder::new()
    .near("location", -73.97, 40.77, Some(1000.0))
    .build();
```

## Aggregation Pipelines

Build aggregation pipelines with helpers:

```rust
use prax_mongodb::types::aggregation::*;

let pipeline = vec![
    match_stage(doc! { "status": "completed" }),
    group_stage(
        "$category".into(),
        doc! {
            "total": sum("$amount"),
            "count": sum(1),
            "avg": avg("$amount"),
        },
    ),
    sort_stage(doc! { "total": -1 }),
    limit_stage(10),
];

let results = collection.aggregate(pipeline).await?;
```

## Configuration

```rust
use prax_mongodb::{MongoConfig, ReadPreference, WriteConcern};
use std::time::Duration;

let config = MongoConfig::builder()
    .uri("mongodb://localhost:27017,localhost:27018,localhost:27019/?replicaSet=rs0")
    .database("mydb")
    .app_name("my-service")
    .max_pool_size(100)
    .min_pool_size(10)
    .connect_timeout(Duration::from_secs(10))
    .read_preference(ReadPreference::SecondaryPreferred)
    .write_concern(WriteConcern::Majority)
    .retry_writes(true)
    .compressors(vec!["zstd".into()])
    .build()?;
```

## MongoDB Atlas

For MongoDB Atlas, use the connection string from Atlas:

```rust
let client = MongoClient::builder()
    .uri("mongodb+srv://user:pass@cluster0.xxxxx.mongodb.net/?retryWrites=true&w=majority")
    .database("mydb")
    .build()
    .await?;
```

## Transactions

Use sessions for multi-document transactions:

```rust
let mut session = client.start_session().await?;

session.start_transaction(None).await?;

// Perform operations within transaction
users.insert_one_with_session(user, &mut session).await?;
orders.insert_one_with_session(order, &mut session).await?;

session.commit_transaction().await?;
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.





MongoDB query engine for Prax ORM.

## Overview

`prax-mongodb` provides an async MongoDB backend using the official `mongodb` Rust driver with built-in connection pooling.

## Features

- Async query execution with Tokio
- Built-in connection pooling
- Document serialization via BSON/serde
- Filter builder for type-safe queries
- Aggregation pipeline support
- Change streams support
- Transactions and sessions

## Usage

```rust
use prax_mongodb::{MongoClient, FilterBuilder, doc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = MongoClient::builder()
        .uri("mongodb://localhost:27017")
        .database("mydb")
        .max_pool_size(10)
        .build()
        .await?;

    // Get a typed collection
    let users = client.collection::<User>("users");

    // Insert a document
    users.insert_one(User { name: "Alice".into(), age: 30 }).await?;

    // Query with filter builder
    let filter = FilterBuilder::new()
        .eq("status", "active")
        .gte("age", 18)
        .build();

    let active_adults: Vec<User> = users
        .find(filter)
        .await?
        .try_collect()
        .await?;

    Ok(())
}
```

## Document Mapping

Models are mapped to MongoDB documents using serde:

```rust
use serde::{Deserialize, Serialize};
use prax_mongodb::ObjectId;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
    email: String,
    #[serde(default)]
    tags: Vec<String>,
}
```

## Filter Builder

Build type-safe MongoDB queries:

```rust
use prax_mongodb::FilterBuilder;

// Simple equality
let filter = FilterBuilder::new()
    .eq("status", "active")
    .build();

// Comparisons
let filter = FilterBuilder::new()
    .gte("age", 18)
    .lt("age", 65)
    .build();

// Array operations
let filter = FilterBuilder::new()
    .in_array("status", vec!["active", "pending"])
    .all("tags", vec!["rust", "async"])
    .build();

// Text search
let filter = FilterBuilder::new()
    .text_search("hello world")
    .build();

// Logical operators
let filter = FilterBuilder::new()
    .or(vec![
        doc! { "priority": "high" },
        doc! { "urgent": true },
    ])
    .build();

// Geospatial
let filter = FilterBuilder::new()
    .near("location", -73.97, 40.77, Some(1000.0))
    .build();
```

## Aggregation Pipelines

Build aggregation pipelines with helpers:

```rust
use prax_mongodb::types::aggregation::*;

let pipeline = vec![
    match_stage(doc! { "status": "completed" }),
    group_stage(
        "$category".into(),
        doc! {
            "total": sum("$amount"),
            "count": sum(1),
            "avg": avg("$amount"),
        },
    ),
    sort_stage(doc! { "total": -1 }),
    limit_stage(10),
];

let results = collection.aggregate(pipeline).await?;
```

## Configuration

```rust
use prax_mongodb::{MongoConfig, ReadPreference, WriteConcern};
use std::time::Duration;

let config = MongoConfig::builder()
    .uri("mongodb://localhost:27017,localhost:27018,localhost:27019/?replicaSet=rs0")
    .database("mydb")
    .app_name("my-service")
    .max_pool_size(100)
    .min_pool_size(10)
    .connect_timeout(Duration::from_secs(10))
    .read_preference(ReadPreference::SecondaryPreferred)
    .write_concern(WriteConcern::Majority)
    .retry_writes(true)
    .compressors(vec!["zstd".into()])
    .build()?;
```

## MongoDB Atlas

For MongoDB Atlas, use the connection string from Atlas:

```rust
let client = MongoClient::builder()
    .uri("mongodb+srv://user:pass@cluster0.xxxxx.mongodb.net/?retryWrites=true&w=majority")
    .database("mydb")
    .build()
    .await?;
```

## Transactions

Use sessions for multi-document transactions:

```rust
let mut session = client.start_session().await?;

session.start_transaction(None).await?;

// Perform operations within transaction
users.insert_one_with_session(user, &mut session).await?;
orders.insert_one_with_session(order, &mut session).await?;

session.commit_transaction().await?;
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.








