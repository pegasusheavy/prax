import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-schema-overview-page',
  standalone: true,
  imports: [CodeBlockComponent, RouterLink],
  templateUrl: './schema-overview.page.html',
})
export class SchemaOverviewPage {
  schemaExample = `// =============================================================================
// Datasource: Database connection configuration
// =============================================================================
datasource db {
    provider = "postgresql"
    url      = env("DATABASE_URL")
}

// =============================================================================
// Generator: Code generation settings
// =============================================================================
generator client {
    provider = "prax-client-rust"
    output   = "./src/generated"
    plugins  = ["serde", "graphql"]
}

// =============================================================================
// Enums: Type-safe enumerated values
// =============================================================================
enum Role {
    USER
    ADMIN
    MODERATOR
}

enum Status {
    DRAFT
    PUBLISHED
    ARCHIVED
}

// =============================================================================
// Models: Database tables
// =============================================================================

/// User accounts in the system
model User {
    id        Int       @id @auto
    email     String    @unique @validate.email
    name      String?
    role      Role      @default(USER)
    posts     Post[]
    profile   Profile?
    createdAt DateTime  @default(now())
    updatedAt DateTime  @updatedAt

    @@index([email])
    @@map("users")
}

/// Blog posts
model Post {
    id        Int      @id @auto
    title     String
    content   String?  @db.Text
    status    Status   @default(DRAFT)
    author    User     @relation(fields: [authorId], references: [id])
    authorId  Int      @map("author_id")
    tags      Tag[]
    createdAt DateTime @default(now())

    @@index([authorId, status])
    @@map("posts")
}

/// User profiles (one-to-one with User)
model Profile {
    id     Int     @id @auto
    bio    String? @db.Text
    avatar String?
    userId Int     @unique
    user   User    @relation(fields: [userId], references: [id])
}

/// Tags for posts (many-to-many)
model Tag {
    id    Int    @id @auto
    name  String @unique
    posts Post[]
}

// =============================================================================
// Views: Read-only aggregated data
// =============================================================================
view UserStats {
    id        Int
    email     String
    postCount Int @map("post_count")

    @@sql("""
        SELECT u.id, u.email, COUNT(p.id) as post_count
        FROM users u
        LEFT JOIN posts p ON p.author_id = u.id
        GROUP BY u.id, u.email
    """)
}`;

  fileStructure = `my-project/
├── prax.toml          # Configuration (project root)
├── prax/              # Prax directory
│   ├── schema.prax    # Your schema definition
│   └── migrations/    # Database migrations
│       ├── 001_init/
│       └── 002_add_roles/
├── src/
│   ├── generated/     # Generated client code
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   └── post.rs
│   └── main.rs
└── .env               # Environment variables`;

  simpleSchema = `// Minimal schema example
datasource db {
    provider = "postgresql"
    url      = env("DATABASE_URL")
}

generator client {
    provider = "prax-client-rust"
    output   = "./src/generated"
}

model User {
    id    Int    @id @auto
    email String @unique
    name  String?
}`;

  workflowSteps = `# 1. Define your schema
edit prax/schema.prax

# 2. Generate the Rust client
prax generate

# 3. Create a migration
prax migrate dev --name init

# 4. Apply migrations to database
prax migrate deploy

# 5. Use in your Rust code
use generated::*;`;
}
