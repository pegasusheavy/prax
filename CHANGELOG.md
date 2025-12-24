# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.2] - 2025-12-24

### Added

- **GraphQL Model Style Configuration** (`prax-codegen`, `prax-schema`)
  - New `model_style` option in `prax.toml`: `"standard"` (default) or `"graphql"`
  - When set to `"graphql"`, model structs generate with `#[derive(async_graphql::SimpleObject)]`
  - `CreateInput` and `UpdateInput` types generate with `#[derive(async_graphql::InputObject)]`
  - Auto-enables GraphQL plugins when `graphql` style is selected
  - Configuration example:
    ```toml
    [generator.client]
    model_style = "graphql"
    ```

## [0.3.1] - 2025-12-21

### Added

- **MySQL Execution Benchmarks** (`benches/database_execution.rs`)
  - Prax MySQL benchmarks with connection pooling
  - SQLx MySQL benchmarks for comparison
  - SELECT by ID, filtered SELECT, and COUNT operations

- **SQLite Execution Benchmarks** (`benches/database_execution.rs`)
  - Prax SQLite benchmarks with in-memory database seeding
  - SQLx SQLite benchmarks for comparison
  - Complete benchmark coverage across all three databases

### Fixed

- Resolved all clippy warnings across the codebase
- Renamed `from_str` methods to `parse` to avoid trait confusion
- Fixed `Include::add` → `Include::with` naming
- Fixed `PooledBuffer::as_mut` → `PooledBuffer::as_mut_str` naming
- Added proper allow attributes for API modules with intentionally unused code

### Changed

- Enabled sqlx `mysql` and `sqlite` features for benchmarks
- Added `prax-mysql`, `prax-sqlite`, `rusqlite` as dev-dependencies

## [0.3.0] - 2025-12-21

### Added

- **Zero-Copy Row Deserialization** (`prax-query`)
  - `RowRef` trait for borrowing string data directly from database rows
  - `FromRowRef<'a>` trait for zero-allocation struct deserialization
  - `FromRow` trait for traditional owning deserialization
  - `FromColumn` trait for type-specific column extraction
  - `RowData` enum for borrowed/owned string data (like `Cow`)
  - `impl_from_row!` macro for easy struct implementation

- **Batch & Pipeline Execution** (`prax-query`)
  - `Pipeline` and `PipelineBuilder` for grouping multiple queries
  - Execute multiple queries with minimal round-trips
  - `PipelineResult` with per-query status tracking
  - Enhanced `Batch::to_combined_sql()` for multi-row INSERT optimization

- **Query Plan Caching** (`prax-query`)
  - `ExecutionPlanCache` for caching query plans with metrics
  - `ExecutionPlan` with SQL, hints, and execution time tracking
  - `PlanHint` enum: `IndexScan`, `SeqScan`, `Parallel`, `Timeout`, etc.
  - `record_execution()` for automatic timing collection
  - `slowest_queries()` and `most_used()` for performance analysis

- **Type-Level Filter Optimizations** (`prax-query`)
  - `InI64Slice`, `InStrSlice` for zero-allocation IN filters
  - `NotInI64Slice`, `NotInStrSlice` for NOT IN filters
  - `And5` struct with `DirectSql` implementation
  - Pre-computed PostgreSQL IN patterns (`POSTGRES_IN_FROM_1`) for 1-32 elements

- **Documentation Website**
  - New "Advanced Performance" page with comprehensive examples
  - Updated Performance page with latest benchmark results
  - Added batch execution, zero-copy, and plan caching documentation

### Changed

- Optimized `write_postgres_in_pattern` for faster IN clause generation
- Updated benchmark results showing Prax matching Diesel for type-level filters
- Improved performance page with database execution benchmarks

### Performance

- Type-level `And5` filter: **~5.1ns** (matches Diesel!)
- `IN(10)` SQL generation: **~3.8ns** (5.8x faster with pre-computed patterns)
- `IN(32)` SQL generation: **~5.0ns** (uses pre-computed pattern lookup)
- Database SELECT by ID: **193µs** (30% faster than SQLx)

## [0.2.0] - 2025-12-20

### Added

- Initial project structure and configuration
- Dual MIT/Apache-2.0 licensing
- Project README with API examples and documentation
- Implementation roadmap (TODO.md)
- Git hooks via cargo-husky:
  - Pre-commit hook for formatting and linting
  - Pre-push hook for test suite validation
  - Commit-msg hook for Conventional Commits enforcement
- Contributing guidelines (CONTRIBUTING.md)
- Schema definition language (SDL) parser (`prax-schema`)
  - Custom `.prax` schema files with Prisma-like syntax
  - AST types for models, fields, relations, enums, views
  - Schema validation and semantic analysis
  - Documentation comments with validation directives (`@validate`)
  - Field metadata and visibility controls (`@hidden`, `@deprecated`, etc.)
  - GraphQL and async-graphql support with federation
- Proc-macro code generation (`prax-codegen`)
  - `#[derive(Model)]` and `prax_schema!` macros
  - Plugin system for extensible code generation
  - Built-in plugins: debug, JSON Schema, GraphQL, serde, validator
- Type-safe query builder (`prax-query`)
  - Fluent API: `findMany`, `findUnique`, `findFirst`, `create`, `update`, `delete`, `upsert`, `count`
  - Filtering system with WHERE clauses, AND/OR/NOT combinators
  - Scalar filters: equals, in, contains, startsWith, endsWith, lt, gt, etc.
  - Sorting with `orderBy`, pagination with `skip`/`take` and cursor-based
  - Aggregation queries: `count`, `sum`, `avg`, `min`, `max`, `groupBy` with `HAVING`
  - Raw SQL escape hatch with type interpolation via `Sql` builder
  - Ergonomic create API with `data!` macro and builder pattern
  - Middleware/hooks system for query interception (logging, metrics, timing, retry)
  - Connection string parsing and multi-database configuration
  - Comprehensive error types with error codes, suggestions, and colored output
  - Multi-tenant support (row-level, schema-based, database-based isolation)
- Async query engines
  - PostgreSQL via `tokio-postgres` with `deadpool-postgres` connection pool (`prax-postgres`)
  - MySQL via `mysql_async` driver (`prax-mysql`)
  - SQLite via `tokio-rusqlite` (`prax-sqlite`)
  - SQLx alternative backend with compile-time checked queries (`prax-sqlx`)
- Relation loading (eager/lazy)
  - `include` and `select` operations for related data
  - Nested writes: create/connect/disconnect/set relations
- Transaction API with async closures, savepoints, isolation levels
- Migration engine (`prax-migrate`)
  - Schema diffing and SQL generation
  - Migration history tracking
  - Database introspection (reverse engineer existing databases)
  - Shadow database support for safe migration testing
  - View migration support (CREATE/DROP/ALTER VIEW, materialized views)
  - Migration resolution system (checksum handling, skip, baseline)
- CLI tool (`prax-cli`)
  - Commands: `init`, `generate`, `migrate`, `db`, `validate`, `format`
  - User-friendly colored output and error handling
- Documentation website with Angular
- Docker setup for testing with real databases
- Benchmarking suite with Criterion
- Profiling support (CPU, memory, tracing)
- Fuzzing infrastructure

### Planned

- Framework integrations (Armature, Axum, Actix-web)
- Integration test suite expansion

---

## Release History

<!--
## [0.1.0] - YYYY-MM-DD

### Added
- Initial release
- Core query builder functionality
- PostgreSQL support via tokio-postgres

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- N/A
-->

[Unreleased]: https://github.com/pegasusheavy/prax/compare/main...HEAD
<!-- [0.1.0]: https://github.com/pegasusheavy/prax/releases/tag/v0.1.0 -->

