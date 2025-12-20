# prax-axum

Axum framework integration for Prax ORM.

## Overview

`prax-axum` provides Tower-compatible middleware and extractors for the Axum web framework.

## Features

- `PraxLayer` Tower middleware
- `DatabaseConnection` extractor
- Compatible with Axum state management
- Zero-cost layer design

## Usage

```rust
use axum::{Router, routing::get};
use prax_axum::{PraxClientBuilder, PraxLayer, DatabaseConnection};

#[tokio::main]
async fn main() {
    let prax = PraxClientBuilder::new()
        .url("postgresql://localhost/mydb")
        .build()
        .await
        .unwrap();

    let app = Router::new()
        .route("/users", get(list_users))
        .layer(PraxLayer::new(prax));

    axum::serve(listener, app).await.unwrap();
}

async fn list_users(
    DatabaseConnection(db): DatabaseConnection,
) -> Json<Vec<User>> {
    let users = db.user().find_many().exec().await.unwrap();
    Json(users)
}
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

