import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-integration-axum-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './integration-axum.page.html',
})
export class IntegrationAxumPage {
  installCode = `[dependencies]
prax = "0.1"
prax-axum = "0.1"
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.5", features = ["trace"] }`;

  setupCode = `use axum::{Router, routing::get};
use prax_axum::{PraxClientBuilder, PraxLayer};

#[tokio::main]
async fn main() {
    // Build the Prax client
    let prax = PraxClientBuilder::new()
        .url("postgresql://localhost/mydb")
        .pool_size(10)
        .build()
        .await
        .expect("Failed to connect to database");

    // Create router with Prax layer
    let app = Router::new()
        .route("/users", get(list_users))
        .route("/users/:id", get(get_user))
        .layer(PraxLayer::new(prax));

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}`;

  handlerCode = `use axum::{extract::Path, Json};
use prax::prelude::*;
use prax_axum::DatabaseConnection;

async fn list_users(
    DatabaseConnection(db): DatabaseConnection,
) -> Result<Json<Vec<User>>, axum::http::StatusCode> {
    let users = db
        .user()
        .find_many()
        .exec()
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(users))
}

async fn get_user(
    DatabaseConnection(db): DatabaseConnection,
    Path(id): Path<i32>,
) -> Result<Json<User>, axum::http::StatusCode> {
    let user = db
        .user()
        .find_unique()
        .where(user::id::equals(id))
        .exec()
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(user))
}

async fn create_user(
    DatabaseConnection(db): DatabaseConnection,
    Json(input): Json<CreateUserInput>,
) -> Result<Json<User>, axum::http::StatusCode> {
    let user = db
        .user()
        .create(user::Create {
            email: input.email,
            name: input.name,
            ..Default::default()
        })
        .exec()
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(user))
}`;

  stateCode = `use axum::{Router, Extension, extract::State};
use prax_axum::PraxClient;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    db: Arc<PraxClient>,
    config: Arc<Config>,
}

#[tokio::main]
async fn main() {
    let prax = PraxClientBuilder::new()
        .url("postgresql://localhost/mydb")
        .build()
        .await
        .unwrap();

    let state = AppState {
        db: Arc::new(prax),
        config: Arc::new(Config::load()),
    };

    let app = Router::new()
        .route("/users", get(list_users))
        .with_state(state);

    // ...
}

async fn list_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<User>>, StatusCode> {
    let users = state.db
        .user()
        .find_many()
        .exec()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(users))
}`;
}

