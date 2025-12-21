import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-config-reference-page',
  standalone: true,
  imports: [CodeBlockComponent, RouterLink],
  templateUrl: './config-reference.page.html',
})
export class ConfigReferencePage {
  // Helper to create env var syntax without Angular interpretation issues
  private env(name: string): string {
    return '$' + '{' + name + '}';
  }

  // Minimal config example
  get minimalConfig(): string {
    return `# prax.toml - Minimal configuration
[database]
provider = "postgresql"
url = "${this.env('DATABASE_URL')}"`;
  }

  // Full config example
  get fullConfig(): string {
    return `# =============================================================================
# Prax Configuration File
# =============================================================================
# This file configures database connections, code generation, migrations,
# and runtime behavior for your Prax ORM project.

# =============================================================================
# Database Configuration
# =============================================================================
[database]
# Database provider: "postgresql", "mysql", "sqlite", "mongodb"
provider = "postgresql"

# Connection URL - supports environment variable interpolation with ${this.env('VAR_NAME')}
url = "${this.env('DATABASE_URL')}"

# Connection pool settings
[database.pool]
min_connections = 2        # Minimum idle connections to maintain
max_connections = 10       # Maximum connections in the pool
connect_timeout = "30s"    # Timeout for new connections
idle_timeout = "10m"       # Close idle connections after this duration
max_lifetime = "30m"       # Maximum connection lifetime

# =============================================================================
# Schema Configuration
# =============================================================================
[schema]
# Path to the schema file (relative to project root)
path = "prax/schema.prax"

# =============================================================================
# Generator Configuration
# =============================================================================
[generator.client]
# Output directory for generated code
output = "./src/generated"

# Generate async client (default: true)
async_client = true

# Enable tracing instrumentation for query observability
tracing = true

# Preview features to enable
preview_features = ["full_text_search", "multi_schema"]

# =============================================================================
# Migration Configuration
# =============================================================================
[migrations]
# Directory for migration files
directory = "./prax/migrations"

# Auto-apply migrations in development (default: false)
auto_migrate = false

# Migration history table name
table_name = "_prax_migrations"

# =============================================================================
# Seeding Configuration
# =============================================================================
[seed]
# Seed script path (Rust or shell script)
script = "./seed.rs"

# Run seed after migrations (default: false)
auto_seed = false

# Environment-specific seeding
[seed.environments]
development = true
staging = false
production = false

# =============================================================================
# Debug/Logging Configuration
# =============================================================================
[debug]
# Log all SQL queries (default: false)
log_queries = false

# Pretty print SQL in logs (default: true)
pretty_sql = true

# Slow query threshold in milliseconds (default: 1000)
slow_query_threshold = 1000

# =============================================================================
# Environment-Specific Overrides
# =============================================================================
# Override any configuration for specific environments

[environments.development]
[environments.development.database]
url = "${this.env('DEV_DATABASE_URL')}"

[environments.development.debug]
log_queries = true
slow_query_threshold = 500

[environments.production]
[environments.production.database.pool]
min_connections = 5
max_connections = 50

[environments.production.debug]
log_queries = false`;
  }

  // Database section
  get databaseConfig(): string {
    return `[database]
# Required: Database provider
# Options: "postgresql", "postgres", "mysql", "sqlite", "sqlite3", "mongodb", "mongo"
provider = "postgresql"

# Required: Database connection URL
# Supports environment variable interpolation: ${this.env('VAR_NAME')}
url = "${this.env('DATABASE_URL')}"

# Optional: Connection pool settings
[database.pool]
min_connections = 2        # Default: 2
max_connections = 10       # Default: 10
connect_timeout = "30s"    # Default: "30s"
idle_timeout = "10m"       # Default: "10m"
max_lifetime = "30m"       # Default: "30m"`;
  }

  // Connection URL examples
  connectionUrls = `# PostgreSQL
url = "postgresql://user:password@localhost:5432/mydb"
url = "postgres://user:password@host:5432/db?sslmode=require"

# MySQL
url = "mysql://user:password@localhost:3306/mydb"
url = "mysql://user:password@host:3306/db?ssl-mode=REQUIRED"

# SQLite
url = "sqlite:./data/myapp.db"
url = "sqlite::memory:"   # In-memory database

# MongoDB
url = "mongodb://user:password@localhost:27017/mydb"
url = "mongodb+srv://user:password@cluster.mongodb.net/mydb"`;

  // Environment variable examples
  get envVarConfig(): string {
    return `# Environment variable interpolation
# Use ${this.env('VAR_NAME')} syntax to reference environment variables

[database]
url = "${this.env('DATABASE_URL')}"

# With fallback in your .env file
# DATABASE_URL=postgresql://localhost/myapp

# Multiple variables
[database]
url = "postgresql://${this.env('DB_USER')}:${this.env('DB_PASSWORD')}@${this.env('DB_HOST')}:${this.env('DB_PORT')}/${this.env('DB_NAME')}"`;
  }

  // Pool config explained
  poolConfig = `[database.pool]
# Minimum connections to keep in the pool
# Higher = faster queries (no wait for connection)
# Lower = less resource usage
min_connections = 2

# Maximum connections allowed
# Set based on your database's max_connections setting
# Rule of thumb: (database max_connections - 10) / number_of_app_instances
max_connections = 10

# How long to wait when establishing a new connection
# Increase if your database server is slow or remote
connect_timeout = "30s"

# Close connections that have been idle for this long
# Helps release database resources
idle_timeout = "10m"

# Maximum lifetime of any connection
# Prevents issues with stale connections
# Should be less than database's wait_timeout
max_lifetime = "30m"`;

  // Schema configuration
  schemaConfig = `[schema]
# Path to your schema file (relative to project root)
# Default: "schema.prax"
path = "prax/schema.prax"

# Prax looks for schema files in these locations (in order):
# 1. Path specified in prax.toml
# 2. prax/schema.prax (default)
# 3. schema.prax
# 4. prisma/schema.prax`;

  // Generator configuration
  generatorConfig = `[generator.client]
# Where to output generated Rust code
# Default: "./src/generated"
output = "./src/generated"

# Generate async client (using tokio)
# Default: true
# Set to false only for sync-only applications
async_client = true

# Enable tracing instrumentation
# Adds #[tracing::instrument] to generated functions
# Default: false
tracing = true

# Preview features to enable
# These are experimental features that may change
preview_features = [
    "full_text_search",   # PostgreSQL full-text search
    "multi_schema",       # Multiple schema support
    "json_filtering",     # JSON field filtering
    "views",              # Database views
]`;

  // Migration configuration
  migrationConfig = `[migrations]
# Directory for migration files
# Default: "./migrations"
directory = "./prax/migrations"

# Auto-apply pending migrations on startup (development only!)
# Default: false
# WARNING: Never enable in production
auto_migrate = false

# Name of the migration history table
# Default: "_prax_migrations"
table_name = "_prax_migrations"`;

  // Seed configuration
  seedConfig = `[seed]
# Path to seed script - supports multiple formats:
# - .rs   - Rust seed script (compiled and executed)
# - .sql  - Raw SQL file (executed directly)
# - .json - JSON data file (declarative seeding)
# - .toml - TOML data file (declarative seeding)
script = "./seed.rs"

# Automatically run seed after migrations
# Default: false
auto_seed = false

# Control seeding per environment
# Prevents accidental seeding in production
[seed.environments]
development = true    # ✓ Seed in development
test = true           # ✓ Seed in test
staging = false       # ✗ Don't seed in staging
production = false    # ✗ Never seed in production`;

  // Seed script example
  seedScriptExample = `// seed.rs - Example seed script
use prax::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PraxClient::new().await?;

    // Create admin user
    client.user().create(
        data! {
            email: "admin@example.com",
            name: "Administrator",
            role: Role::Admin
        }
    ).exec().await?;

    // Create sample data
    for i in 1..=10 {
        client.post().create(
            data! {
                title: format!("Sample Post {}", i),
                content: "Lorem ipsum...",
                published: true
            }
        ).exec().await?;
    }

    println!("✓ Database seeded successfully");
    Ok(())
}`;

  // Debug configuration
  debugConfig = `[debug]
# Log all executed SQL queries
# Useful for debugging, but verbose in production
# Default: false
log_queries = false

# Pretty-print SQL in logs (with indentation)
# Default: true
pretty_sql = true

# Warn about queries taking longer than this (milliseconds)
# Default: 1000 (1 second)
slow_query_threshold = 1000`;

  // Debug output example
  debugOutput = `# With log_queries = true and pretty_sql = true:

[PRAX] Executing query:
    SELECT
        "users"."id",
        "users"."email",
        "users"."name"
    FROM "users"
    WHERE "users"."email" = $1
    LIMIT 1
[PRAX] Query completed in 3ms

# With slow_query_threshold = 500:
[PRAX] ⚠️ Slow query detected (523ms):
    SELECT * FROM "posts" WHERE "published" = true`;

  // Environment overrides
  get environmentOverrides(): string {
    return `# Base configuration
[database]
url = "postgresql://localhost/myapp_dev"

[database.pool]
max_connections = 10

[debug]
log_queries = false

# =============================================================================
# Development environment overrides
# =============================================================================
[environments.development]
[environments.development.database]
url = "${this.env('DEV_DATABASE_URL')}"

[environments.development.debug]
log_queries = true
pretty_sql = true
slow_query_threshold = 100

# =============================================================================
# Test environment overrides
# =============================================================================
[environments.test]
[environments.test.database]
url = "postgresql://localhost/myapp_test"

[environments.test.database.pool]
min_connections = 1
max_connections = 5

# =============================================================================
# Production environment overrides
# =============================================================================
[environments.production]
[environments.production.database]
url = "${this.env('PRODUCTION_DATABASE_URL')}"

[environments.production.database.pool]
min_connections = 10
max_connections = 100
connect_timeout = "10s"

[environments.production.debug]
log_queries = false
slow_query_threshold = 5000`;
  }

  // Usage in code
  loadingConfig = `use prax_schema::config::PraxConfig;

// Load configuration from file
let config = PraxConfig::from_file("prax.toml")?;

// Apply environment-specific overrides
let config = config.with_environment("production");

// Access configuration values
let db_url = config.database_url();
let pool_size = config.database.pool.max_connections;
let output_dir = config.generator.client.output;`;

  // Environment variable setup
  envFileExample = `# .env file example
DATABASE_URL=postgresql://user:password@localhost:5432/myapp

# Development
DEV_DATABASE_URL=postgresql://localhost/myapp_dev

# Production (set in your deployment environment)
PRODUCTION_DATABASE_URL=postgresql://produser:secret@prod.db.server:5432/myapp`;

  // Common configurations
  get webAppConfig(): string {
    return `# prax.toml for a typical web application

[database]
provider = "postgresql"
url = "${this.env('DATABASE_URL')}"

[database.pool]
min_connections = 5
max_connections = 20
connect_timeout = "30s"
idle_timeout = "10m"
max_lifetime = "30m"

[schema]
path = "prax/schema.prax"

[generator.client]
output = "./src/db"
tracing = true

[migrations]
directory = "./prax/migrations"

[debug]
log_queries = false
slow_query_threshold = 1000

[environments.development]
[environments.development.debug]
log_queries = true`;
  }

  cliAppConfig = `# prax.toml for a CLI application

[database]
provider = "sqlite"
url = "sqlite:./data/app.db"

[database.pool]
min_connections = 1
max_connections = 1

[schema]
path = "prax/schema.prax"

[generator.client]
output = "./src/generated"
async_client = true

[migrations]
directory = "./prax/migrations"
auto_migrate = true   # OK for CLI apps with local database`;

  get microserviceConfig(): string {
    return `# prax.toml for a microservice

[database]
provider = "postgresql"
url = "${this.env('DATABASE_URL')}"

[database.pool]
min_connections = 2
max_connections = 10
connect_timeout = "10s"
idle_timeout = "5m"
max_lifetime = "15m"

[schema]
path = "prax/schema.prax"

[generator.client]
output = "./src/db"
tracing = true
preview_features = ["multi_schema"]

[migrations]
directory = "./prax/migrations"
table_name = "_prax_migrations_orders_svc"

[debug]
slow_query_threshold = 500

[environments.production]
[environments.production.database.pool]
min_connections = 5
max_connections = 25`;
  }

  // File location
  fileStructure = `my-project/
├── prax.toml              # ← Configuration file (project root)
├── .env                   # Environment variables
├── prax/
│   ├── schema.prax        # Schema definition
│   └── migrations/        # Migration files
├── src/
│   ├── generated/         # Generated code (from generator.client.output)
│   └── main.rs
└── Cargo.toml`;

  // Validation errors
  get validationErrors(): string {
    return `# Common configuration errors and fixes

# ❌ Error: Unknown field 'databse'
[databse]   # Typo!
url = "..."
# ✅ Fix: Use correct spelling
[database]
url = "..."

# ❌ Error: Invalid provider 'postgre'
[database]
provider = "postgre"   # Invalid
# ✅ Fix: Use valid provider name
provider = "postgresql"  # or "postgres"

# ❌ Error: Environment variable not found
url = "${this.env('UNDEFINED_VAR')}"
# ✅ Fix: Set the environment variable or use a default
# In your .env: UNDEFINED_VAR=postgresql://localhost/db

# ❌ Error: Invalid duration format
connect_timeout = 30   # Missing unit
# ✅ Fix: Include time unit
connect_timeout = "30s"  # seconds
idle_timeout = "10m"     # minutes
max_lifetime = "1h"      # hours`;
  }
}
