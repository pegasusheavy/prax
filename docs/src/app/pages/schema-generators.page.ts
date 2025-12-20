import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-schema-generators-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './schema-generators.page.html',
})
export class SchemaGeneratorsPage {
  datasourceBasic = `// Database connection configuration
datasource db {
    // Database provider
    provider = "postgresql"  // postgresql | mysql | sqlite

    // Connection URL (supports env vars)
    url = env("DATABASE_URL")
}`;

  datasourceAdvanced = `// Full datasource configuration
datasource db {
    provider = "postgresql"

    // Primary connection URL
    url = env("DATABASE_URL")

    // Direct connection (bypasses pooling)
    directUrl = env("DIRECT_DATABASE_URL")

    // Shadow database for migrations
    shadowDatabaseUrl = env("SHADOW_DATABASE_URL")

    // Connection pool settings
    connectionLimit = 10
    poolTimeout = 10

    // Relation mode for PlanetScale/Vitess
    relationMode = "prisma"  // "foreignKeys" | "prisma"
}`;

  multipleProviders = `// Multiple datasources (for different schemas/databases)
datasource primary {
    provider = "postgresql"
    url      = env("PRIMARY_DATABASE_URL")
}

datasource analytics {
    provider = "postgresql"
    url      = env("ANALYTICS_DATABASE_URL")
}

// Models can reference specific datasources
model User {
    id    Int    @id @auto
    email String @unique

    @@datasource(primary)
}

model AnalyticsEvent {
    id        Int      @id @auto
    eventType String
    data      Json
    timestamp DateTime @default(now())

    @@datasource(analytics)
}`;

  generatorBasic = `// Client generator configuration
generator client {
    // Generator provider
    provider = "prax-client-rust"

    // Output directory for generated code
    output = "./src/generated"
}`;

  generatorAdvanced = `// Full generator configuration
generator client {
    provider = "prax-client-rust"
    output   = "./src/generated"

    // Preview features to enable
    previewFeatures = ["fullTextSearch", "multiSchema", "views"]

    // Binary targets for different platforms
    binaryTargets = ["native", "linux-musl"]

    // Engine configuration
    engineType = "library"  // "library" | "binary"
}`;

  generatorPlugins = `// Generator with plugins
generator client {
    provider = "prax-client-rust"
    output   = "./src/generated"

    // Enable built-in plugins
    plugins = [
        "serde",      // Serialize/Deserialize derives
        "graphql",    // async-graphql integration
        "validator",  // Input validation
        "debug",      // Debug derives
        "json_schema" // JSON Schema generation
    ]
}

// Plugin-specific configuration
generator graphql {
    provider = "prax-graphql"
    output   = "./src/graphql"

    // GraphQL-specific options
    federation = true           // Apollo Federation support
    subscriptions = true        // Enable subscriptions
    complexity = true           // Query complexity analysis
}`;

  customGenerators = `// Custom generator
generator docs {
    provider = "prax-generator-docs"
    output   = "./docs/api"

    format = "markdown"
    includeExamples = true
}

// TypeScript client for frontend
generator typescript {
    provider = "prax-generator-typescript"
    output   = "../frontend/src/api"

    runtime = "fetch"           // "fetch" | "axios"
    includeTypes = true
    generateHooks = true        // React Query hooks
}

// OpenAPI specification
generator openapi {
    provider = "prax-generator-openapi"
    output   = "./api/openapi.yaml"

    version = "3.1.0"
    servers = ["https://api.example.com"]
}`;

  envVariables = `// Environment variable interpolation
datasource db {
    provider = "postgresql"

    // Direct env reference
    url = env("DATABASE_URL")

    // With default value (shell syntax not supported, use .env)
    // url = env("DATABASE_URL")
}

// .env file example:
// DATABASE_URL="postgresql://user:password@localhost:5432/mydb"
// SHADOW_DATABASE_URL="postgresql://user:password@localhost:5432/mydb_shadow"
// DIRECT_DATABASE_URL="postgresql://user:password@localhost:5432/mydb?connection_limit=1"`;

  schemaExample = `// Complete schema file structure
// =============================================================================
// Datasource: Database connection
// =============================================================================
datasource db {
    provider = "postgresql"
    url      = env("DATABASE_URL")
}

// =============================================================================
// Generator: Code generation settings
// =============================================================================
generator client {
    provider        = "prax-client-rust"
    output          = "./src/generated"
    previewFeatures = ["fullTextSearch"]
    plugins         = ["serde", "graphql"]
}

// =============================================================================
// Enums: Enumerated types
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
model User {
    id        Int      @id @auto
    email     String   @unique
    name      String?
    role      Role     @default(USER)
    posts     Post[]
    createdAt DateTime @default(now())
    updatedAt DateTime @updatedAt

    @@map("users")
    @@index([email])
}

model Post {
    id        Int      @id @auto
    title     String
    content   String?
    status    Status   @default(DRAFT)
    author    User     @relation(fields: [authorId], references: [id])
    authorId  Int      @map("author_id")
    createdAt DateTime @default(now())

    @@map("posts")
    @@index([authorId, status])
}

// =============================================================================
// Views: Read-only aggregations
// =============================================================================
view UserStats {
    id        Int
    email     String
    postCount Int @map("post_count")

    @@sql("SELECT u.id, u.email, COUNT(p.id) as post_count FROM users u LEFT JOIN posts p ON p.author_id = u.id GROUP BY u.id")
}`;
}


