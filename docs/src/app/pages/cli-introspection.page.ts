import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-cli-introspection-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './cli-introspection.page.html',
})
export class CliIntrospectionPage {
  basicPull = `# Generate schema from existing database
prax db pull

# Outputs to prax/schema.prax by default
# Introspects tables, columns, indexes, relations, views

# Specify output file
prax db pull --output ./my-schema.prax

# Overwrite existing without prompting
prax db pull --force`;

  filteringTables = `# Filter by table pattern
prax db pull --tables "user*"          # Tables starting with "user"
prax db pull --tables "auth_*"         # Auth-related tables

# Exclude tables
prax db pull --exclude "_prisma*"      # Skip Prisma internals
prax db pull --exclude "temp_*,log_*"  # Skip temp and log tables

# Specific schema/namespace
prax db pull --schema public           # PostgreSQL
prax db pull --schema dbo              # MSSQL`;

  includeViews = `# Include views in introspection
prax db pull --include-views

# Include materialized views
prax db pull --include-materialized-views

# Both
prax db pull --include-views --include-materialized-views`;

  outputFormats = `# Output as Prax schema (default)
prax db pull --format prax

# Output as JSON (for tooling integration)
prax db pull --format json --output schema.json

# Output as SQL (CREATE TABLE statements)
prax db pull --format sql --output schema.sql

# Print to stdout instead of file
prax db pull --print
prax db pull --format json --print | jq '.tables[].name'`;

  mongoOptions = `# MongoDB-specific options

# Sample size for schema inference
prax db pull --sample-size 100    # Default
prax db pull --sample-size 1000   # More accurate for varied documents

# MongoDB infers schema by sampling documents
# Larger sample = more accurate type inference
# Smaller sample = faster introspection`;

  generatedSchema = `# Example output from PostgreSQL database

generator client {
  provider = "prax-postgres"
  output   = "./generated"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

model User {
  id        Int      @id @default(autoincrement())
  email     String   @unique @db.VarChar(255)
  name      String?  @db.VarChar(100)
  createdAt DateTime @default(now()) @map("created_at")
  updatedAt DateTime @updatedAt @map("updated_at")

  posts     Post[]
  profile   Profile?

  @@map("users")
  @@index([email])
}

model Post {
  id        Int      @id @default(autoincrement())
  title     String   @db.VarChar(255)
  content   String?  @db.Text
  published Boolean  @default(false)
  authorId  Int      @map("author_id")
  createdAt DateTime @default(now()) @map("created_at")

  author    User     @relation(fields: [authorId], references: [id])

  @@map("posts")
  @@index([authorId])
}

// Introspected view
view UserStats {
  userId     Int     @map("user_id")
  postCount  Int     @map("post_count")
  totalViews Int     @map("total_views")

  @@sql("""
    SELECT user_id, COUNT(*) as post_count, SUM(views) as total_views
    FROM posts GROUP BY user_id
  """)
  @@map("user_stats")
}`;

  programmaticApi = `use prax_query::introspection::{
    Introspector, IntrospectionConfig, DatabaseSchema, generate_prax_schema
};

// Introspect database programmatically
let config = IntrospectionConfig::new()
    .schema("public")
    .include_tables(["users", "posts", "comments"])
    .exclude_tables(["_migrations"])
    .include_views(true)
    .include_materialized_views(true);

// PostgreSQL introspection
let schema = PostgresIntrospector::new(&client)
    .introspect(config)
    .await?;

// Access schema information
for table in &schema.tables {
    println!("Table: {}", table.name);
    for col in &table.columns {
        println!("  {} {} {}",
            col.name,
            col.data_type,
            if col.is_nullable { "NULL" } else { "NOT NULL" }
        );
    }
}

// Generate Prax schema string
let prax_schema = generate_prax_schema(&schema, "postgresql")?;
std::fs::write("schema.prax", prax_schema)?;

// Export as JSON
let json = serde_json::to_string_pretty(&schema)?;
std::fs::write("schema.json", json)?;`;

  mongoInference = `use prax_query::introspection::mongodb::SchemaInferrer;

// MongoDB schema inference
let inferrer = SchemaInferrer::new(&client)
    .sample_size(100)  // Documents to sample per collection
    .collections(["users", "posts", "comments"]);

let schema = inferrer.infer().await?;

// Inferred types based on document analysis:
// - String, Int, Float, Boolean, Date, ObjectId
// - Array<T> with element type inference
// - Embedded documents as nested types
// - Union types for fields with multiple types

// Example inferred schema:
model User {
  id        String   @id @map("_id") @db.ObjectId
  email     String
  name      String?
  age       Int?
  tags      String[]
  profile   Profile  // Embedded document
  createdAt DateTime
}

type Profile {
  bio       String?
  avatar    String?
  social    Social?
}`;

  typeMapping = `// Type mapping from database to Prax types

| PostgreSQL      | MySQL           | SQLite    | MSSQL           | Prax Type |
|-----------------|-----------------|-----------|-----------------|-----------|
| INTEGER         | INT             | INTEGER   | INT             | Int       |
| BIGINT          | BIGINT          | INTEGER   | BIGINT          | BigInt    |
| SMALLINT        | SMALLINT        | INTEGER   | SMALLINT        | Int       |
| SERIAL          | AUTO_INCREMENT  | -         | IDENTITY        | Int @auto |
| VARCHAR(n)      | VARCHAR(n)      | TEXT      | NVARCHAR(n)     | String    |
| TEXT            | TEXT            | TEXT      | NVARCHAR(MAX)   | String    |
| BOOLEAN         | TINYINT(1)      | INTEGER   | BIT             | Boolean   |
| TIMESTAMP       | DATETIME        | TEXT      | DATETIME2       | DateTime  |
| DATE            | DATE            | TEXT      | DATE            | DateTime  |
| DECIMAL(p,s)    | DECIMAL(p,s)    | REAL      | DECIMAL(p,s)    | Decimal   |
| FLOAT/REAL      | FLOAT/DOUBLE    | REAL      | FLOAT/REAL      | Float     |
| JSONB/JSON      | JSON            | TEXT      | NVARCHAR(MAX)   | Json      |
| BYTEA           | BLOB            | BLOB      | VARBINARY(MAX)  | Bytes     |
| UUID            | CHAR(36)        | TEXT      | UNIQUEIDENTIFIER| String    |
| ARRAY           | -               | -         | -               | Type[]    |`;
}



