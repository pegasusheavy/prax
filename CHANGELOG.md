# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

