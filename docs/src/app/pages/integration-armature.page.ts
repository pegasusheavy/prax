import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-integration-armature-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './integration-armature.page.html',
})
export class IntegrationArmaturePage {
  installCode = `[dependencies]
prax-orm = "0.3"
prax-armature = "0.3"
armature = { version = "0.1", features = ["macros"] }
tokio = { version = "1", features = ["full"] }`;

  moduleCode = `use armature::prelude::*;
use prax_armature::{PraxClient, PraxClientBuilder};
use std::sync::Arc;

#[module_impl]
impl DatabaseModule {
    #[provider(singleton)]
    async fn prax_client() -> Arc<PraxClient> {
        let client = PraxClientBuilder::new()
            .url("postgresql://localhost/mydb")
            .pool_size(10)
            .build()
            .await
            .expect("Failed to connect to database");

        Arc::new(client)
    }
}

#[module(
    imports = [DatabaseModule],
    controllers = [UserController],
)]
struct AppModule;`;

  controllerCode = `use armature::prelude::*;
use prax::prelude::*;
use std::sync::Arc;

#[controller("/users")]
impl UserController {
    #[get("/")]
    async fn list_users(
        &self,
        #[inject] db: Arc<PraxClient>,
    ) -> Result<Json<Vec<User>>, HttpError> {
        let users = db
            .user()
            .find_many()
            .exec()
            .await
            .map_err(|e| HttpError::internal(e.to_string()))?;

        Ok(Json(users))
    }

    #[get("/:id")]
    async fn get_user(
        &self,
        #[inject] db: Arc<PraxClient>,
        #[param] id: i32,
    ) -> Result<Json<User>, HttpError> {
        let user = db
            .user()
            .find_unique()
            .where(user::id::equals(id))
            .exec()
            .await
            .map_err(|e| HttpError::internal(e.to_string()))?
            .ok_or_else(|| HttpError::not_found("User not found"))?;

        Ok(Json(user))
    }

    #[post("/")]
    async fn create_user(
        &self,
        #[inject] db: Arc<PraxClient>,
        #[body] input: CreateUserInput,
    ) -> Result<Json<User>, HttpError> {
        let user = db
            .user()
            .create(user::Create {
                email: input.email,
                name: input.name,
                ..Default::default()
            })
            .exec()
            .await
            .map_err(|e| HttpError::internal(e.to_string()))?;

        Ok(Json(user))
    }
}`;

  transactionCode = `use armature::prelude::*;
use prax_armature::{DatabaseMiddleware, RequestTransaction};

// Add middleware to your module
#[module(
    imports = [DatabaseModule],
    middleware = [DatabaseMiddleware],
    controllers = [OrderController],
)]
struct AppModule;

#[controller("/orders")]
impl OrderController {
    #[post("/")]
    async fn create_order(
        &self,
        #[inject] tx: RequestTransaction,
        #[body] input: CreateOrderInput,
    ) -> Result<Json<Order>, HttpError> {
        // All operations in this handler use the same transaction
        let order = tx.order().create(/* ... */).exec().await?;

        // Update inventory
        tx.product()
            .update(product::id::equals(input.product_id))
            .data(product::stock::decrement(input.quantity))
            .exec()
            .await?;

        // Transaction commits automatically on success
        // Rolls back automatically on error
        Ok(Json(order))
    }
}`;
}

