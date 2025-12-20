# prax-actix

Actix-web framework integration for Prax ORM.

## Overview

`prax-actix` provides middleware and extractors for the Actix-web framework.

## Features

- `PraxMiddleware` for request handling
- `DatabaseConnection` FromRequest extractor
- App data integration
- High-performance async design

## Usage

```rust
use actix_web::{web, App, HttpServer};
use prax_actix::{PraxClientBuilder, DatabaseConnection};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let prax = PraxClientBuilder::new()
        .url("postgresql://localhost/mydb")
        .build()
        .await
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(prax.clone()))
            .route("/users", web::get().to(list_users))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

async fn list_users(db: DatabaseConnection) -> impl Responder {
    let users = db.user().find_many().exec().await.unwrap();
    HttpResponse::Ok().json(users)
}
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

