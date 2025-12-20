# Docker Setup for Prax ORM

This directory contains Docker configuration for testing Prax against real databases.

## Quick Start

```bash
# Start all databases
docker compose up -d postgres mysql

# Run all tests
docker compose run --rm test

# Run specific database tests
docker compose run --rm test-postgres
docker compose run --rm test-mysql
docker compose run --rm test-sqlite
```

## Services

| Service | Description | Port |
|---------|-------------|------|
| `postgres` | PostgreSQL 16 | 5432 |
| `mysql` | MySQL 8.0 | 3306 |
| `test` | Run all tests | - |
| `test-postgres` | PostgreSQL tests only | - |
| `test-mysql` | MySQL tests only | - |
| `test-sqlite` | SQLite tests only | - |
| `dev` | Development shell | - |
| `bench` | Run benchmarks | - |
| `coverage` | Generate coverage report | - |

## Connection Strings

### PostgreSQL
```
postgres://prax:prax_test_password@localhost:5432/prax_test
```

### MySQL
```
mysql://prax:prax_test_password@localhost:3306/prax_test
```

### SQLite
```
file:./test.db
```

## Common Commands

### Start Databases

```bash
# Start all databases in background
docker compose up -d

# Start specific database
docker compose up -d postgres
docker compose up -d mysql

# View logs
docker compose logs -f postgres
```

### Run Tests

```bash
# All tests against all databases
docker compose run --rm test

# Specific database
docker compose run --rm test-postgres
docker compose run --rm test-mysql
docker compose run --rm test-sqlite

# With specific test filter
docker compose run --rm test cargo test query_builder

# With verbose output
docker compose run --rm -e RUST_LOG=debug test
```

### Development

```bash
# Open interactive development shell
docker compose run --rm dev

# Inside the container:
cargo test -p prax-postgres
cargo bench
```

### Benchmarks

```bash
# Run all benchmarks
docker compose run --rm bench

# Results are saved to ./target/criterion/
```

### Coverage

```bash
# Generate HTML coverage report
docker compose run --rm coverage

# Report is saved to ./coverage/
open coverage/html/index.html
```

### Cleanup

```bash
# Stop all services
docker compose down

# Stop and remove volumes (clean slate)
docker compose down -v

# Remove built images
docker compose down --rmi local
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `POSTGRES_URL` | PostgreSQL connection string | See above |
| `MYSQL_URL` | MySQL connection string | See above |
| `SQLITE_URL` | SQLite file path | `file:./test.db` |
| `RUST_BACKTRACE` | Enable backtraces | `1` |
| `RUST_LOG` | Log level | `info` |

## Database Initialization

### PostgreSQL

The `postgres/init.sql` script:
- Enables `uuid-ossp` and `pgcrypto` extensions
- Creates additional test databases
- Sets up migration tracking table

### MySQL

The `mysql/init.sql` script:
- Creates additional test databases
- Grants permissions to test user
- Sets up migration tracking table

## Troubleshooting

### Container won't start

```bash
# Check logs
docker compose logs postgres
docker compose logs mysql

# Rebuild from scratch
docker compose down -v
docker compose build --no-cache
docker compose up -d
```

### Tests can't connect to database

```bash
# Ensure database is healthy
docker compose ps

# Wait for healthy status
docker compose up -d postgres
docker compose exec postgres pg_isready -U prax
```

### Permission denied errors

```bash
# Fix ownership of mounted volumes
sudo chown -R $USER:$USER target/ coverage/
```

### Out of disk space

```bash
# Clean up Docker resources
docker system prune -a --volumes
```

## CI Integration

Example GitHub Actions workflow:

```yaml
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_USER: prax
          POSTGRES_PASSWORD: prax_test_password
          POSTGRES_DB: prax_test
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Run tests
        env:
          DATABASE_URL: postgres://prax:prax_test_password@localhost:5432/prax_test
        run: cargo test -p prax-postgres --all-features
```

