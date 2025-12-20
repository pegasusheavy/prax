# Prax CLI

The official command-line interface for the Prax ORM.

## Installation

```bash
cargo install prax-cli
```

Or build from source:

```bash
cargo build --release -p prax-cli
```

## Usage

### Initialize a New Project

```bash
# Initialize with interactive prompts
prax init my-project

# Initialize with defaults (no prompts)
prax init my-project --yes --provider postgresql
```

### Generate Client Code

```bash
# Generate from default schema location
prax generate

# Generate from specific schema file
prax generate --schema ./path/to/schema.prax

# Generate with output directory
prax generate --output ./src/generated
```

### Validate Schema

```bash
# Validate the schema
prax validate

# Validate a specific schema file
prax validate --schema ./path/to/schema.prax
```

### Format Schema

```bash
# Format and display the schema
prax format

# Format and write back to file
prax format --write
```

### Database Migrations

```bash
# Development workflow (create and apply migrations)
prax migrate dev

# Apply pending migrations
prax migrate deploy

# Show migration status
prax migrate status

# Reset database (WARNING: destructive)
prax migrate reset
```

### Direct Database Operations

```bash
# Push schema to database (creates tables without migrations)
prax db push

# Pull schema from existing database
prax db pull

# Seed database
prax db seed
```

### Version Information

```bash
prax version
```

## Project Structure

Prax uses a dedicated `prax/` directory for schema and migrations:

```
my-project/
├── prax.toml              # Configuration (project root)
├── prax/                  # Prax directory
│   ├── schema.prax        # Database schema definition
│   └── migrations/        # Migration files
│       └── .gitkeep
├── src/
│   └── generated/         # Generated Rust code
└── .env                   # Environment variables
```

## Configuration

The CLI uses a `prax.toml` configuration file in your project root:

```toml
[database]
provider = "postgresql"
url = "${DATABASE_URL}"

[schema]
# Schema file location (default: prax/schema.prax)
path = "prax/schema.prax"

[generator]
# Generated code output directory
output = "./src/generated"

[migrations]
# Migrations directory (default: prax/migrations)
directory = "./prax/migrations"

# Enabled plugins for code generation
plugins = ["serde", "graphql"]
```

## Environment Variables

- `DATABASE_URL`: Database connection URL (overrides config file)
- `PRAX_SCHEMA`: Path to schema file (overrides config file)
- `PRAX_OUTPUT`: Output directory for generated code

## Exit Codes

| Code | Meaning |
|------|---------|
| 0    | Success |
| 1    | General error |
| 2    | Configuration error |
| 3    | Schema validation error |
| 4    | Database connection error |
| 5    | Migration error |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

