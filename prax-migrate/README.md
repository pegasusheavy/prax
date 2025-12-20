# prax-migrate

Database migration engine for Prax ORM.

## Overview

`prax-migrate` provides automatic schema diffing and migration generation for Prax schemas.

## Features

- Automatic schema diffing
- Migration file generation
- Up/down migration support
- Migration history tracking
- Shadow database for safe migrations
- Introspection from existing databases

## Usage

```rust
use prax_migrate::{MigrationEngine, MigrationConfig};

let engine = MigrationEngine::new(config).await?;

// Generate migrations from schema changes
engine.generate("add_user_table").await?;

// Apply pending migrations
engine.migrate().await?;

// Rollback last migration
engine.rollback(1).await?;
```

## CLI

```bash
# Generate a new migration
prax migrate generate add_posts_table

# Apply all pending migrations
prax migrate up

# Rollback the last migration
prax migrate down

# Show migration status
prax migrate status
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

