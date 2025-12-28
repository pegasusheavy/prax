import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-quickstart-page',
  standalone: true,
  imports: [CodeBlockComponent, RouterLink],
  templateUrl: './quickstart.page.html',
})
export class QuickstartPage {
  step1Code = `[dependencies]
prax-orm = "0.3"
tokio = { version = "1", features = ["full"] }

# Choose your database driver
prax-postgres = "0.3"  # PostgreSQL
# prax-mysql = "0.3"   # MySQL
# prax-sqlite = "0.3"  # SQLite
# prax-duckdb = "0.3"  # DuckDB (analytics)`;

  step2Code = `// User model with profile relation
model User {
    id        Int      @id @auto
    email     String   @unique
    name      String?
    posts     Post[]
    profile   Profile?
    createdAt DateTime @default(now())
    updatedAt DateTime @updatedAt
}

// Post model with author relation
model Post {
    id        Int      @id @auto
    title     String
    content   String?
    published Boolean  @default(false)
    author    User     @relation(fields: [authorId], references: [id])
    authorId  Int
    createdAt DateTime @default(now())
}

// Profile model (one-to-one with User)
model Profile {
    id     Int     @id @auto
    bio    String?
    user   User    @relation(fields: [userId], references: [id])
    userId Int     @unique
}`;

  step3Code = `[database]
provider = "postgresql"
url = "postgres://user:password@localhost:5432/myapp"

[schema]
path = "prax/schema.prax"  # Default location

[generator]
output = "./src/generated"

[migrations]
directory = "./prax/migrations"

# Optional: Enable preview features
# preview_features = ["json_protocol"]`;

  step4Code = `# Generate Prax client
cargo prax generate

# Or use the CLI directly
prax generate`;

  step5Code = `# Create a new migration
prax migrate dev --name init

# Apply migrations to production
prax migrate deploy`;

  step6Code = `use prax::prelude::*;
use prax_query::data;
mod generated;
use generated::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the database
    let client = PraxClient::new().await?;

    // Create a new user with the data! macro
    let user = client
        .user()
        .create(data! {
            email: "alice@example.com",
            name: "Alice",
        })
        .exec()
        .await?;

    println!("Created user: {} ({})", user.name.unwrap(), user.email);

    // Query all users
    let users = client
        .user()
        .find_many()
        .exec()
        .await?;

    println!("Found {} users", users.len());

    Ok(())
}`;
}
