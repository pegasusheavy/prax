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

### Planned

- Schema definition language (SDL) parser
- Proc-macro code generation for models
- Type-safe query builder with fluent API
- Async query engines (tokio-postgres, SQLx)
- Relation loading (eager/lazy)
- Transaction API with savepoints
- Migration engine
- CLI tooling
- Multi-database support (PostgreSQL, MySQL, SQLite)
- Framework integrations (Armature, Axum, Actix-web)

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

