import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-integration-actix-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './integration-actix.page.html',
})
export class IntegrationActixPage {
  installCode = `[dependencies]
prax = "0.1"
prax-actix = "0.1"
actix-web = "4"
tokio = { version = "1", features = ["full"] }`;

  setupCode = `use actix_web::{web, App, HttpServer};
use prax_actix::{PraxClient, PraxClientBuilder};
use std::sync::Arc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Build the Prax client
    let prax = PraxClientBuilder::new()
        .url("postgresql://localhost/mydb")
        .pool_size(10)
        .build()
        .await
        .expect("Failed to connect to database");

    let prax = Arc::new(prax);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(prax.clone()))
            .route("/users", web::get().to(list_users))
            .route("/users/{id}", web::get().to(get_user))
            .route("/users", web::post().to(create_user))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}`;

  handlerCode = `use actix_web::{web, HttpResponse, Result};
use prax::prelude::*;
use prax_actix::DatabaseConnection;

async fn list_users(
    db: DatabaseConnection,
) -> Result<HttpResponse> {
    let users = db
        .user()
        .find_many()
        .exec()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(users))
}

async fn get_user(
    db: DatabaseConnection,
    path: web::Path<i32>,
) -> Result<HttpResponse> {
    let id = path.into_inner();

    let user = db
        .user()
        .find_unique()
        .where(user::id::equals(id))
        .exec()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    match user {
        Some(user) => Ok(HttpResponse::Ok().json(user)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

async fn create_user(
    db: DatabaseConnection,
    body: web::Json<CreateUserInput>,
) -> Result<HttpResponse> {
    let user = db
        .user()
        .create(user::Create {
            email: body.email.clone(),
            name: body.name.clone(),
            ..Default::default()
        })
        .exec()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Created().json(user))
}`;

  middlewareCode = `use actix_web::{web, App, HttpServer};
use prax_actix::{PraxMiddleware, PraxClientBuilder};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let prax = PraxClientBuilder::new()
        .url("postgresql://localhost/mydb")
        .build()
        .await
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .wrap(PraxMiddleware::new(prax.clone()))
            .route("/users", web::get().to(list_users))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

// The middleware injects the connection into request extensions
async fn list_users(
    db: DatabaseConnection,
) -> Result<HttpResponse> {
    // db is extracted from request extensions
    let users = db.user().find_many().exec().await?;
    Ok(HttpResponse::Ok().json(users))
}`;
}

