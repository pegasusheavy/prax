import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-home-page',
  standalone: true,
  imports: [RouterLink, CodeBlockComponent],
  templateUrl: './home.page.html',
})
export class HomePage {
  schemaCode = `model User {
    id        Int      @id @auto
    email     String   @unique
    name      String?
    posts     Post[]
    profile   Profile?
    createdAt DateTime @default(now())
}

model Post {
    id        Int      @id @auto
    title     String
    content   String?
    published Boolean  @default(false)
    author    User     @relation(fields: [authorId], references: [id])
    authorId  Int
}`;

  queryCode = `use prax::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = PraxClient::new().await?;

    // Find all published posts with their authors
    let posts = client
        .post()
        .find_many()
        .where(post::published::equals(true))
        .include(post::author::fetch())
        .order_by(post::created_at::desc())
        .exec()
        .await?;

    for post in posts {
        println!("{} by {}", post.title, post.author.name);
    }

    Ok(())
}`;

  createExample = `use prax_query::{data, connect};

// Create a user with nested posts
let user = client
    .user()
    .create(data! {
        email: "alice@example.com",
        name: "Alice",
        posts: vec![
            data! { title: "Hello World", content: "My first post!" },
            data! { title: "Rust is Awesome", published: true },
        ],
    })
    .exec()
    .await?;`;

  filterExample = `// Complex filtering with AND, OR, NOT
let users = client
    .user()
    .find_many()
    .where(user::AND(vec![
        user::email::ends_with("@example.com"),
        user::OR(vec![
            user::name::contains("Admin"),
            user::posts::some(post::published::equals(true)),
        ]),
    ]))
    .skip(10)
    .take(20)
    .exec()
    .await?;`;

  installCode = `[dependencies]
prax-orm = "0.3"
tokio = { version = "1", features = ["full"] }`;
}
