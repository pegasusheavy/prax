# Prax ORM - Implementation TODO

A full-featured Prisma-like ORM for Rust with async support via tokio-postgres and similar clients.

---

## 📋 Schema & Parsing Layer

- [ ] **Design schema definition language (SDL) parser** - Custom DSL similar to Prisma's `.prisma` schema files
- [ ] **Create AST types for schema** - Models, fields, relations, enums, attributes representation
- [ ] **Implement schema validation and semantic analysis** - Type checking, relation validation, error reporting

---

## 🔧 Code Generation

- [ ] **Build proc-macro crate for compile-time model code generation** - `#[derive(Model)]` and related macros for type-safe structs

---

## 🔍 Query Builder

- [ ] **Create type-safe query builder with fluent API** - `findMany`, `findUnique`, `findFirst`, `create`, `update`, `delete`
- [ ] **Add filtering system** - Where clauses, AND/OR/NOT combinators, nested filters on relations
- [ ] **Implement sorting, pagination** - `orderBy`, `skip`/`take`, cursor-based pagination
- [ ] **Add aggregation queries** - `count`, `sum`, `avg`, `min`, `max`, `groupBy`
- [ ] **Create raw SQL escape hatch with type interpolation** - Safe raw query execution with parameter binding
- [ ] **Implement upsert, createMany, updateMany, deleteMany operations** - Bulk operations with type safety

---

## ⚡ Async Query Engines

- [ ] **Implement async connection pool manager** - Using `deadpool-postgres` or `bb8` for connection pooling
- [ ] **Build tokio-postgres query engine with prepared statement caching** - Primary PostgreSQL driver
- [ ] **Add SQLx query engine as alternative backend** - Compile-time checked queries option

---

## 🔗 Relations & Nested Operations

- [ ] **Implement relation loading (eager/lazy)** - `include` and `select` operations for related data
- [ ] **Add nested writes** - Create/connect/disconnect/set relations in single mutation operations

---

## 💾 Transactions

- [ ] **Create transaction API with async closures and savepoints** - `$transaction()` with automatic rollback on error

---

## 🚀 Migrations

- [ ] **Build migration engine** - Schema diffing, SQL generation, migration history tracking
- [ ] **Implement database introspection** - Reverse engineer schema from existing database

---

## 🎛️ Infrastructure & Tooling

- [ ] **Build middleware/hooks system for query interception** - Before/after query hooks, logging, metrics
- [ ] **Create CLI tool for schema management, migrations, and generation** - `prax generate`, `prax migrate`, `prax db push`
- [ ] **Implement connection string parsing and multi-database config** - DATABASE_URL parsing, connection options
- [ ] **Add comprehensive error types with actionable messages** - Detailed errors with suggestions for fixes

---

## 🗄️ Multi-Database Support

- [ ] **Add MySQL support** - via `mysql_async` client
- [ ] **Add SQLite support** - via `rusqlite` with tokio wrapper

---

## 🔌 Framework Integrations

- [ ] **Armature framework integration** - First-class DI integration with [Armature](https://github.com/pegasusheavy/armature) HTTP framework
  - [ ] Implement `#[provider]` compatible service factory for PraxClient
  - [ ] Add connection pool as injectable singleton via `#[module_impl]`
  - [ ] Create `prax-armature` integration crate
  - [ ] Support request-scoped transactions via Armature's DI container
  - [ ] Add middleware for automatic connection handling
- [ ] **Axum integration** - Tower-compatible middleware and extractors
- [ ] **Actix-web integration** - Actor-based connection management

---

## 📚 Documentation & Testing

- [ ] **Write documentation, examples, and integration test suite** - API docs, usage examples, comprehensive tests

---

## Implementation Order (Suggested)

### Phase 1: Core Foundation
1. Schema parsing (SDL parser, AST types, validation)
2. Core query engine for PostgreSQL (connection pool, tokio-postgres)
3. Proc-macro code generation

### Phase 2: Query API
4. Query builder with fluent API
5. Filtering and sorting
6. Relations and eager loading

### Phase 3: Advanced Features
7. Transactions with savepoints
8. Migrations engine
9. Database introspection

### Phase 4: Ecosystem
10. CLI tooling
11. MySQL and SQLite support
12. Middleware system

### Phase 5: Framework Integrations
13. Armature integration (`prax-armature`)
14. Axum integration
15. Actix-web integration

### Phase 6: Polish
16. Documentation and examples
17. Integration test suite
18. Performance optimization

---

## Architecture Overview

```
prax/
├── prax-core/           # Core types, traits, and abstractions
├── prax-schema/         # Schema parser and AST
├── prax-codegen/        # Proc-macro crate for code generation
├── prax-query/          # Query builder implementation
├── prax-postgres/       # tokio-postgres query engine
├── prax-mysql/          # mysql_async query engine
├── prax-sqlite/         # rusqlite query engine
├── prax-migrate/        # Migration engine
├── prax-cli/            # CLI tool
├── prax-armature/       # Armature framework integration
└── prax/                # Main crate re-exporting everything
```

---

## Example Usage (Target API)

```rust
use prax::prelude::*;

// Schema-generated model
#[derive(Model)]
#[prax(table = "users")]
struct User {
    #[prax(id, auto_increment)]
    id: i32,
    email: String,
    name: Option<String>,
    #[prax(relation(has_many))]
    posts: Vec<Post>,
}

// Queries
async fn example(client: &PraxClient) -> Result<()> {
    // Find many with filtering
    let users = client
        .user()
        .find_many()
        .where_(user::email::contains("@example.com"))
        .include(user::posts::fetch())
        .order_by(user::created_at::desc())
        .take(10)
        .exec()
        .await?;

    // Create with nested relation
    let user = client
        .user()
        .create(user::Create {
            email: "new@example.com".into(),
            name: Some("New User".into()),
            posts: Some(user::posts::create_many(vec![
                post::Create { title: "First Post".into(), .. },
            ])),
        })
        .exec()
        .await?;

    // Transaction
    client
        .transaction(|tx| async move {
            tx.user().update(user::id::equals(1)).data(user::Update {
                name: Some("Updated".into()),
                ..Default::default()
            }).exec().await?;
            Ok(())
        })
        .await?;

    Ok(())
}
```

---

## Armature Integration Example (Target API)

```rust
use armature::prelude::*;
use prax_armature::PraxModule;

// Register Prax as a module in Armature's DI system
#[module_impl]
impl DatabaseModule {
    #[provider(singleton)]
    async fn prax_client() -> Arc<PraxClient> {
        let client = PraxClient::new("postgresql://localhost/mydb")
            .await
            .expect("Failed to connect to database");
        Arc::new(client)
    }
}

#[module(
    imports = [DatabaseModule],
    controllers = [UserController],
)]
struct AppModule;

// Inject PraxClient into controllers
#[controller("/users")]
impl UserController {
    #[get("/")]
    async fn list_users(
        &self,
        #[inject] db: Arc<PraxClient>,
    ) -> Result<Json<Vec<User>>, HttpError> {
        let users = db
            .user()
            .find_many()
            .exec()
            .await
            .map_err(|e| HttpError::internal(e.to_string()))?;

        Ok(Json(users))
    }

    #[post("/")]
    async fn create_user(
        &self,
        #[inject] db: Arc<PraxClient>,
        #[body] input: CreateUserInput,
    ) -> Result<Json<User>, HttpError> {
        let user = db
            .user()
            .create(user::Create {
                email: input.email,
                name: input.name,
                ..Default::default()
            })
            .exec()
            .await
            .map_err(|e| HttpError::internal(e.to_string()))?;

        Ok(Json(user))
    }
}
```

---

## References

- [Prisma Documentation](https://www.prisma.io/docs)
- [tokio-postgres](https://docs.rs/tokio-postgres)
- [SQLx](https://docs.rs/sqlx)
- [SeaORM](https://www.sea-ql.org/SeaORM/) - Existing Rust ORM for reference
- [Diesel](https://diesel.rs/) - Existing Rust ORM for reference
- [Armature](https://github.com/pegasusheavy/armature) - Pegasus Heavy Industries HTTP framework for Rust (primary integration target)

