import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-database-seeding-page',
  standalone: true,
  imports: [CodeBlockComponent, RouterLink],
  templateUrl: './database-seeding.page.html',
})
export class DatabaseSeedingPage {
  basicUsage = `# Auto-detect seed file and run
prax db seed

# Specify seed file explicitly
prax db seed --seed-file ./my-seed.sql

# Reset database before seeding
prax db seed --reset

# Specify environment (development, staging, production)
prax db seed --environment staging

# Force seeding in restricted environments
prax db seed --environment production --force`;

  configToml = `# prax.toml - Seed configuration
[seed]
# Path to seed file (supports .rs, .sql, .json, .toml)
script = "./seed.rs"

# Automatically run seed after migrations
auto_seed = false

# Per-environment seeding controls
[seed.environments]
development = true    # ✓ Enabled
test = true           # ✓ Enabled
staging = false       # ✗ Disabled
production = false    # ✗ Disabled (protected!)`;

  sqlSeedExample = `-- seed.sql - SQL seed file
-- Clear existing data (optional)
-- TRUNCATE TABLE users, posts CASCADE;

-- Insert users
INSERT INTO users (id, email, name, role, created_at) VALUES
    (1, 'admin@example.com', 'Admin User', 'ADMIN', CURRENT_TIMESTAMP),
    (2, 'john@example.com', 'John Doe', 'USER', CURRENT_TIMESTAMP),
    (3, 'jane@example.com', 'Jane Smith', 'USER', CURRENT_TIMESTAMP)
ON CONFLICT (id) DO NOTHING;

-- Insert posts
INSERT INTO posts (id, title, content, published, author_id) VALUES
    (1, 'Welcome to Prax', 'First post!', true, 1),
    (2, 'Getting Started', 'Learn Prax.', true, 1)
ON CONFLICT (id) DO NOTHING;`;

  jsonSeedExample = `{
  "truncate": false,
  "disable_fk_checks": true,
  "order": ["users", "posts", "comments"],
  "tables": {
    "users": [
      {
        "id": 1,
        "email": "admin@example.com",
        "name": "Admin User",
        "role": "ADMIN",
        "created_at": "now()"
      },
      {
        "id": 2,
        "email": "john@example.com",
        "name": "John Doe",
        "role": "USER",
        "created_at": "now()"
      }
    ],
    "posts": [
      {
        "id": 1,
        "title": "Welcome to Prax",
        "content": "First post using Prax ORM!",
        "published": true,
        "author_id": 1,
        "created_at": "now()"
      }
    ]
  }
}`;

  tomlSeedExample = `# seed.toml - TOML seed file
truncate = false
disable_fk_checks = true
order = ["users", "posts"]

[[tables.users]]
id = 1
email = "admin@example.com"
name = "Admin User"
role = "ADMIN"
created_at = "now()"

[[tables.users]]
id = 2
email = "john@example.com"
name = "John Doe"
role = "USER"
created_at = "now()"

[[tables.posts]]
id = 1
title = "Welcome to Prax"
content = "First post!"
published = true
author_id = 1
created_at = "now()"`;

  rustSeedExample = `// seed.rs - Rust seed script
use prax::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Prax client
    let client = PraxClient::new().await?;

    println!("🌱 Seeding database...");

    // Create admin user
    let admin = client.user().create(
        user::email::set("admin@example.com".to_string()),
        user::name::set("Admin User".to_string()),
        vec![user::role::set(Role::Admin)],
    ).exec().await?;
    println!("  Created admin user: {}", admin.email);

    // Create sample posts
    for i in 1..=5 {
        client.post().create(
            post::title::set(format!("Sample Post {}", i)),
            post::author::connect(user::id::equals(admin.id)),
            vec![
                post::content::set(format!("Content for post {}", i)),
                post::published::set(i % 2 == 0),
            ],
        ).exec().await?;
    }
    println!("  Created 5 sample posts");

    println!("✅ Database seeded successfully!");
    Ok(())
}`;

  cargoTomlBin = `# Cargo.toml - Add seed binary
[[bin]]
name = "seed"
path = "seed.rs"`;

  migrateIntegration = `# Migrate dev runs seed automatically
prax migrate dev

# Skip seed during migration
prax migrate dev --skip-seed

# Reset database and run seed
prax migrate reset --seed`;

  fileLocations = `# Prax looks for seed files in these locations (in order):
# 1. Configured in prax.toml [seed].script
# 2. ./seed.rs, ./seed.sql, ./seed.json, ./seed.toml
# 3. ./prax/seed.rs, ./prax/seed.sql, ...
# 4. ./prisma/seed.rs (Prisma compatibility)
# 5. ./src/seed.rs
# 6. ./seeds/seed.rs, ./seeds/seed.sql`;

  specialFunctions = `# Special functions in JSON/TOML seed files:
# now()  - Current timestamp
# uuid() - Generate UUID v4

# Example:
{
  "created_at": "now()",
  "id": "uuid()"
}

# Translates to:
# PostgreSQL: CURRENT_TIMESTAMP, gen_random_uuid()
# MySQL:      NOW(), UUID()
# SQLite:     datetime('now'), '<generated-uuid>'`;
}

