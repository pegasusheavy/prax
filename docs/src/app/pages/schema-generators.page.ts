import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-schema-generators-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './schema-generators.page.html',
})
export class SchemaGeneratorsPage {
  datasourceBasic = `// Datasource declares the database provider and extensions
// Actual connection URL is configured in prax.toml
datasource db {
    // Database provider
    provider = "postgresql"  // postgresql | mysql | sqlite

    // PostgreSQL extensions (optional)
    // extensions = [vector, pg_trgm, uuid-ossp]
}`;

  datasourceAdvanced = `// Datasource in schema.prax
datasource db {
    provider   = "postgresql"
    extensions = [vector, pg_trgm]  // PostgreSQL extensions
}

// Connection settings go in prax.toml:
// [database]
// provider = "postgresql"
// url = "\${DATABASE_URL}"
//
// [database.pool]
// max_connections = 10
// connect_timeout = 30
//
// [database.migrations]
// shadow_url = "\${SHADOW_DATABASE_URL}"  # For migration diffing`;

  multipleProviders = `// Multiple datasources for different databases
// Declare datasources in schema.prax:
datasource primary {
    provider = "postgresql"
}

datasource analytics {
    provider = "postgresql"
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
}

// Configure URLs in prax.toml:
// [datasources.primary]
// url = "\${PRIMARY_DATABASE_URL}"
//
// [datasources.analytics]
// url = "\${ANALYTICS_DATABASE_URL}"`;

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

  envVariables = `// Environment variables are used in prax.toml, not in the schema
// schema.prax - declares database type
datasource db {
    provider = "postgresql"
}

// prax.toml - uses environment variables
// [database]
// provider = "postgresql"
// url = "\${DATABASE_URL}"  # Interpolated from environment
//
// [database.migrations]
// shadow_url = "\${SHADOW_DATABASE_URL}"

// .env file example:
// DATABASE_URL="postgresql://user:password@localhost:5432/mydb"
// SHADOW_DATABASE_URL="postgresql://user:password@localhost:5432/mydb_shadow"`;

  schemaExample = `// Complete schema file structure
// =============================================================================
// Datasource: Database provider and extensions
// (Connection URL is in prax.toml)
// =============================================================================
datasource db {
    provider   = "postgresql"
    extensions = [vector]  // Optional: PostgreSQL extensions
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


