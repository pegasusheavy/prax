# prax-armature

Armature framework integration for Prax ORM.

## Overview

`prax-armature` provides seamless integration with the Armature HTTP framework using dependency injection.

## Features

- First-class DI integration with Armature
- Singleton connection pool provider
- Request-scoped transactions
- DatabaseMiddleware for automatic connection handling

## Usage

```rust
use armature::prelude::*;
use prax_armature::{PraxClient, PraxClientBuilder};

#[module_impl]
impl DatabaseModule {
    #[provider(singleton)]
    async fn prax_client() -> Arc<PraxClient> {
        let client = PraxClientBuilder::new()
            .url("postgresql://localhost/mydb")
            .build()
            .await
            .unwrap();
        Arc::new(client)
    }
}

#[controller("/users")]
impl UserController {
    #[get("/")]
    async fn list_users(
        &self,
        #[inject] db: Arc<PraxClient>,
    ) -> Result<Json<Vec<User>>, HttpError> {
        let users = db.user().find_many().exec().await?;
        Ok(Json(users))
    }
}
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

