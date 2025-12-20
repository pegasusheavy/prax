import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-examples-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './examples.page.html',
})
export class ExamplesPage {
  blogSchema = `model User {
    id        Int       @id @auto
    email     String    @unique
    name      String
    posts     Post[]
    comments  Comment[]
    createdAt DateTime  @default(now())
}

model Post {
    id        Int       @id @auto
    title     String
    slug      String    @unique
    content   String
    published Boolean   @default(false)
    author    User      @relation(fields: [authorId], references: [id])
    authorId  Int
    comments  Comment[]
    tags      Tag[]
    createdAt DateTime  @default(now())
    updatedAt DateTime  @updatedAt
}

model Comment {
    id        Int      @id @auto
    content   String
    author    User     @relation(fields: [authorId], references: [id])
    authorId  Int
    post      Post     @relation(fields: [postId], references: [id])
    postId    Int
    createdAt DateTime @default(now())
}

model Tag {
    id    Int    @id @auto
    name  String @unique
    posts Post[]
}`;

  blogUsage = `use prax_query::{data, connect};

// Create a post with tags
let post = client
    .post()
    .create(data! {
        title: "Getting Started with Prax",
        slug: "getting-started-prax",
        content: "...",
        author: connect!(id: author_id),
        tags: connect_or_create![
            { where: { name: "rust" }, create: { name: "rust" } },
            { where: { name: "orm" }, create: { name: "orm" } },
        ],
    })
    .exec()
    .await?;

// Get post with author and comments
let post = client
    .post()
    .find_unique()
    .where(post::slug::equals("getting-started-prax"))
    .include(post::author::fetch())
    .include(post::comments::fetch().include(comment::author::fetch()))
    .exec()
    .await?;`;

  ecomSchema = `model Product {
    id          Int         @id @auto
    name        String
    description String?
    price       Decimal
    stock       Int         @default(0)
    category    Category    @relation(fields: [categoryId], references: [id])
    categoryId  Int
    orderItems  OrderItem[]
    createdAt   DateTime    @default(now())
}

model Category {
    id       Int       @id @auto
    name     String    @unique
    products Product[]
}

model Order {
    id         Int         @id @auto
    customer   Customer    @relation(fields: [customerId], references: [id])
    customerId Int
    items      OrderItem[]
    total      Decimal
    status     OrderStatus @default(PENDING)
    createdAt  DateTime    @default(now())
}

model OrderItem {
    id        Int     @id @auto
    order     Order   @relation(fields: [orderId], references: [id])
    orderId   Int
    product   Product @relation(fields: [productId], references: [id])
    productId Int
    quantity  Int
    price     Decimal
}

enum OrderStatus {
    PENDING
    PROCESSING
    SHIPPED
    DELIVERED
    CANCELLED
}`;

  ecomUsage = `use prax_query::{data, connect};

// Create order with items
let order = client
    .order()
    .create(data! {
        customer: connect!(id: customer_id),
        total: total,
        items: cart_items.iter().map(|item| data! {
            product: connect!(id: item.product_id),
            quantity: item.quantity,
            price: item.price,
        }).collect(),
    })
    .exec()
    .await?;

// Get orders with low stock products
let orders = client
    .order()
    .find_many()
    .where(order::items::some(
        order_item::product::is(product::stock::lt(10))
    ))
    .include(order::items::fetch().include(order_item::product::fetch()))
    .exec()
    .await?;`;

  axumExample = `use axum::{extract::{Path, State}, http::StatusCode, routing::get, Json, Router};
use prax::prelude::*;
use std::sync::Arc;

struct AppState {
    db: PraxClient,
}

#[tokio::main]
async fn main() {
    let client = PraxClient::new().await.unwrap();
    let state = Arc::new(AppState { db: client });

    let app = Router::new()
        .route("/users", get(list_users))
        .route("/users/:id", get(get_user))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn list_users(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<User>> {
    let users = state.db
        .user()
        .find_many()
        .exec()
        .await
        .unwrap();

    Json(users)
}

async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<User>, StatusCode> {
    state.db
        .user()
        .find_unique()
        .where(user::id::equals(id))
        .exec()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}`;

  transactionExample = `use prax_query::{data, decrement, increment};

// Transfer funds between accounts (must be atomic)
let result = client
    .transaction(|tx| async move {
        // Debit from source
        let source = tx
            .account()
            .update()
            .where(account::id::equals(from_id))
            .data(data! { balance: decrement!(amount) })
            .exec()
            .await?;

        // Check sufficient balance
        if source.balance < 0.0 {
            return Err(PraxError::custom("Insufficient funds"));
        }

        // Credit to destination
        tx
            .account()
            .update()
            .where(account::id::equals(to_id))
            .data(data! { balance: increment!(amount) })
            .exec()
            .await?;

        // Record the transfer
        tx
            .transfer()
            .create(data! {
                from_id: from_id,
                to_id: to_id,
                amount: amount,
            })
            .exec()
            .await?;

        Ok(())
    })
    .await?;`;
}
