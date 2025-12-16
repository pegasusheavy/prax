# Contributing to Prax

Thank you for your interest in contributing to Prax! This document provides guidelines and information for contributors.

## Getting Started

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone git@github.com:YOUR_USERNAME/prax.git
   cd prax
   ```
3. Install dependencies and set up hooks:
   ```bash
   cargo build
   ```
   This will automatically install git hooks via `cargo-husky`.

## Development Workflow

### Git Hooks

This project uses `cargo-husky` to manage git hooks automatically:

| Hook | Purpose |
|------|---------|
| `pre-commit` | Runs `cargo fmt` and `cargo clippy` |
| `pre-push` | Runs full test suite |
| `commit-msg` | Validates commit message format |

Hooks are installed automatically when you run `cargo build`.

### Commit Message Format

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification.

#### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

#### Types

| Type | Description |
|------|-------------|
| `feat` | A new feature |
| `fix` | A bug fix |
| `docs` | Documentation changes |
| `style` | Code style changes (formatting, etc) |
| `refactor` | Code refactoring |
| `perf` | Performance improvements |
| `test` | Adding or updating tests |
| `build` | Build system or dependencies |
| `ci` | CI/CD configuration |
| `chore` | Other changes |
| `revert` | Reverting a previous commit |

#### Scopes

Common scopes for this project:

- `query` - Query builder
- `schema` - Schema parsing
- `postgres` - PostgreSQL driver
- `mysql` - MySQL driver
- `sqlite` - SQLite driver
- `migrate` - Migration engine
- `codegen` - Code generation
- `cli` - CLI tool
- `armature` - Armature integration
- `deps` - Dependencies

#### Examples

```bash
# Feature
git commit -m "feat(query): add support for nested where clauses"

# Bug fix
git commit -m "fix(postgres): handle connection pool exhaustion"

# Documentation
git commit -m "docs: add migration guide to README"

# Breaking change (note the !)
git commit -m "feat(api)!: redesign query builder interface"

# With body
git commit -m "fix(schema): parse enum variants correctly

Previously, enum variants with underscores were not parsed.
This fix handles snake_case and PascalCase variants.

Fixes #123"
```

## Code Style

### Formatting

- Use `cargo fmt` for formatting
- Run before committing (enforced by pre-commit hook)

### Linting

- Use `cargo clippy` for linting
- All warnings are treated as errors in CI
- Run before committing (enforced by pre-commit hook)

### Documentation

- Document all public APIs
- Include examples in doc comments
- Run `cargo doc --open` to preview documentation

## Testing

### Running Tests

```bash
# Run all tests
cargo test --all-features

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run doc tests
cargo test --doc
```

### Writing Tests

- Place unit tests in a `#[cfg(test)]` module
- Place integration tests in `tests/`
- Use descriptive test names

## Pull Request Process

1. Create a feature branch from `main`:
   ```bash
   git checkout -b feat/my-feature
   ```

2. Make your changes and commit using conventional commits

3. Ensure all checks pass:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features
   cargo test --all-features
   ```

4. Push your branch and create a pull request

5. Wait for review and address any feedback

## Reporting Issues

- Use GitHub Issues for bug reports and feature requests
- Include reproduction steps for bugs
- Check existing issues before creating new ones

## License

By contributing to Prax, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.

