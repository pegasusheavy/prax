import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-queries-crud-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './queries-crud.page.html',
})
export class QueriesCrudPage {
  createCode = `use prax_query::data;

// Create a single record with the data! macro (recommended)
let user = client
    .user()
    .create(data! {
        email: "alice@example.com",
        name: "Alice",
        age: 30,
    })
    .exec()
    .await?;

// Create many records
let users = client
    .user()
    .create_many(vec![
        data! { email: "bob@example.com", name: "Bob" },
        data! { email: "charlie@example.com", name: "Charlie" },
    ])
    .exec()
    .await?;

// Skip duplicates
let users = client
    .user()
    .create_many(vec![...])
    .skip_duplicates()
    .exec()
    .await?;`;

  createBuilder = `use prax_query::DataBuilder;

// Using the builder pattern for more control
let user = client
    .user()
    .create(
        DataBuilder::new()
            .set("email", "alice@example.com")
            .set("name", "Alice")
            .set_default("createdAt")  // Use database default
            .connect("team", 1)        // Connect to existing team
    )
    .exec()
    .await?;`;

  readCode = `// Find unique by ID
let user = client
    .user()
    .find_unique()
    .where(user::id::equals(1))
    .exec()
    .await?;

// Find first matching
let user = client
    .user()
    .find_first()
    .where(user::email::contains("@example.com"))
    .exec()
    .await?;

// Find many
let users = client
    .user()
    .find_many()
    .where(user::active::equals(true))
    .exec()
    .await?;`;

  updateCode = `use prax_query::{data, increment};

// Update with the data! macro
let user = client
    .user()
    .update()
    .where(user::id::equals(1))
    .data(data! {
        name: "Alice Updated",
        updatedAt: chrono::Utc::now().to_rfc3339(),
    })
    .exec()
    .await?;

// Numeric operations
let user = client
    .user()
    .update()
    .where(user::id::equals(1))
    .data(data! {
        views: increment!(1),       // Increment by 1
        loginCount: increment!(1),
    })
    .exec()
    .await?;

// Update many
let count = client
    .user()
    .update_many()
    .where(user::active::equals(false))
    .data(data! { deletedAt: chrono::Utc::now().to_rfc3339() })
    .exec()
    .await?;`;

  deleteCode = `// Delete unique
let user = client
    .user()
    .delete()
    .where(user::id::equals(1))
    .exec()
    .await?;

// Delete many
let count = client
    .user()
    .delete_many()
    .where(user::active::equals(false))
    .exec()
    .await?;`;

  upsertCode = `use prax_query::data;

// Create or update
let user = client
    .user()
    .upsert()
    .where(user::email::equals("alice@example.com"))
    .create(data! {
        email: "alice@example.com",
        name: "Alice",
    })
    .update(data! {
        name: "Alice (Updated)",
        loginCount: increment!(1),
    })
    .exec()
    .await?;`;

  relationCreate = `use prax_query::{data, connect};

// Create with relation connection
let post = client
    .post()
    .create(data! {
        title: "Hello World",
        content: "My first post!",
        author: connect!(id: 1),  // Connect to existing user
    })
    .exec()
    .await?;

// Create with nested create
let user = client
    .user()
    .create(data! {
        email: "alice@example.com",
        name: "Alice",
        posts: vec![
            data! { title: "Post 1", content: "..." },
            data! { title: "Post 2", content: "..." },
        ],
    })
    .exec()
    .await?;`;
}
