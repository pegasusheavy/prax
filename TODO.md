# Prax ORM - Implementation Status

A full-featured Prisma-like ORM for Rust with async support via tokio-postgres and similar clients.

---

## ✅ Performance Optimization - COMPLETE

All major performance optimizations have been implemented. Prax now **exceeds Diesel's performance** for query building.

### Latest Benchmark Results (with Docker PostgreSQL)

#### Query Building Performance

| Operation | Prax | Diesel | SQLx | Speedup |
|-----------|------|--------|------|---------|
| Simple SELECT | **40ns** | 278ns | 5ns | **7x** vs Diesel |
| SELECT + filters | **105ns** | 633ns | 5ns | **6x** vs Diesel |
| INSERT query | **81ns** | - | 5ns | - |
| UPDATE query | **101ns** | - | 5ns | - |
| PostgreSQL query | **46ns** | - | - | - |
| MySQL query | **46ns** | - | - | - |
| SQLite query | **46ns** | - | - | - |

#### Filter Construction Performance

| Operation | Prax (TypeLevel) | Prax (Runtime) | Diesel | Notes |
|-----------|------------------|----------------|--------|-------|
| Simple filter | **2.1ns** | 7ns | 4.7ns | DirectSql: 2.1ns |
| AND (2 filters) | **4.3ns** | 17ns | 5ns | DirectSql matches Diesel |
| AND (5 filters) | **5.1ns** | 32ns | 5ns | TypeLevel = Diesel! |
| AND (5) SQL gen | **17ns** | - | - | DirectSql write |
| AND (10 filters) | - | 68ns | - | Static field names |
| IN (10 values) | **3.8ns** | 21ns | 14ns | Pre-computed pattern |
| IN (32 values) | **5.0ns** | - | - | Pre-computed pattern |
| IN (100 values) | **158ns** | 160ns | - | Looped generation |

#### Database Execution (PostgreSQL Docker with Pooling)

| Operation | Prax | SQLx | Diesel-Async | Winner |
|-----------|------|------|--------------|--------|
| SELECT by ID | **193µs** | 276µs | 6.18ms | **Prax** |
| SELECT filtered | **192µs** | 269µs | 7.40ms | **Prax** |
| COUNT | **255µs** | 320µs | - | **Prax** |
| SELECT prepared | **191µs** | - | - | **Prax** |

> **Note**: Diesel-Async benchmarks establish a new connection per iteration (~6ms overhead).
> Prax and SQLx use connection pooling. Prax includes pool warmup.

### Memory Footprint

| Type | Size | Notes |
|------|------|-------|
| Filter enum | **64B** | Fits in single cache line |
| ValueList | **24B** | 91% reduction from SmallVec |

### Key Optimizations Implemented

- **DirectSql trait** - Zero-allocation SQL generation (~1.7ns)
- **Pre-computed placeholders** - 256-entry PostgreSQL placeholder table
- **SqlTemplateCache** - LRU cache with ~34ns lookup
- **Pre-compiled model SQL** - `model::sql::FIND_BY_ID` etc. as const strings
- **Global field name registry** - 57 pre-registered field names
- **Compile-time filter macros** - `filter!()`, `and_filter!()`, etc.

---

## 🔄 Ongoing Optimization Opportunities

Based on benchmark analysis against Diesel-Async and SQLx with real databases:

### ✅ High Priority - COMPLETE

- [x] **Add Prax database execution benchmarks** - Compare against Diesel-Async/SQLx with pooling
  - Prax is now **30% faster** than SQLx for filtered queries!
- [x] **Connection pool warmup** - `pool.warmup(n)` pre-establishes connections
  - Also added `warmup_with_statements()` for pre-preparing common queries
- [x] **Prepared statement caching per-connection** - All queries use `prepare_cached()`

### ✅ Medium Priority - COMPLETE

- [x] **Reduce boxed filter overhead** - **ACHIEVED ~5ns for type-level AND(5)!**
  - Implemented `And5`, `And3`, `Or5`, `Or3` type-level filter constructors
  - `and5_type_construction`: **~5.1ns** (matches Diesel!)
  - `and5_chained_construction`: **~5.2ns**
  - DirectSql SQL generation: **~17ns** for AND(5)
  - Runtime Filter conversion adds ~25ns overhead (expected for dynamic dispatch)
- [x] **IN filter optimization** - **Pre-computed patterns for instant lookup!**
  - Added `POSTGRES_IN_FROM_1` patterns for 1-32 elements
  - `in_slice_10_write_sql`: **~3.8ns** (from ~22ns, 5.8x faster!)
  - `in_slice_32_write_sql`: **~5.0ns** (uses pre-computed pattern)
  - `in_slice_100_write_sql`: **~158ns** (limited by string ops)
  - Added `InI64Slice`, `InStrSlice` for zero-allocation DirectSql

### ✅ Low Priority (Nice to Have) - COMPLETE

- [x] **Zero-copy row deserialization** - Implemented `RowRef` trait, `FromRowRef`, `FromRow`
  - `RowRef` trait for zero-copy string access via `get_str()` and `get_bytes()`
  - `FromRowRef<'a>` trait for deserializing with borrowed data
  - `RowData` enum for `Cow`-like borrowed/owned string data
  - `impl_from_row!` macro for easy struct deserialization
- [x] **Batch query execution** - Implemented `Pipeline` and `PipelineBuilder`
  - `Pipeline` for grouping multiple queries for efficient execution
  - `PipelineBuilder` with fluent `.query()` and `.execute()` methods
  - `PipelineResult` with per-query results and error handling
  - Enhanced `Batch` with combined INSERT optimization
- [x] **Query plan caching** - Implemented `ExecutionPlanCache` with performance tracking
  - `ExecutionPlan` with SQL, hints, and execution metrics
  - `PlanHint` enum: `IndexScan`, `SeqScan`, `Parallel`, `Timeout`, etc.
  - Automatic execution time tracking via `record_execution()`
  - `slowest_queries()` and `most_used()` for performance analysis

### Benchmark Infrastructure

- [x] Docker PostgreSQL setup with seeded data (1000 users, 5000 posts)
- [x] Docker MySQL setup with seeded data
- [x] Criterion benchmarks for query building
- [x] Criterion benchmarks for filter construction
- [x] Prax async database execution benchmarks
- [x] Add MySQL execution benchmarks
- [x] Add SQLite execution benchmarks

---

## ✅ Framework Integrations - COMPLETE

- **prax-armature** - Armature DI integration
- **prax-axum** - Tower middleware & extractors
- **prax-actix** - Actix-web middleware & extractors

---

## 🏗️ Architecture

```
prax/
├── prax-schema/         # Schema parser and AST
├── prax-codegen/        # Proc-macro crate for code generation
├── prax-query/          # Query builder implementation
├── prax-postgres/       # tokio-postgres query engine
├── prax-mysql/          # mysql_async query engine
├── prax-sqlite/         # rusqlite query engine
├── prax-sqlx/           # SQLx-based query engine
├── prax-migrate/        # Migration engine
├── prax-cli/            # CLI tool
├── prax-armature/       # Armature framework integration
├── prax-axum/           # Axum framework integration
├── prax-actix/          # Actix-web framework integration
└── prax/                # Main crate re-exporting everything
```

---

## 📖 Example Usage

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
        .where(user::email::contains("@example.com"))
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

## 🔗 Armature Integration Example

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
}
```

---

## 📚 References

- [Prisma Documentation](https://www.prisma.io/docs)
- [tokio-postgres](https://docs.rs/tokio-postgres)
- [SQLx](https://docs.rs/sqlx)
- [SeaORM](https://www.sea-ql.org/SeaORM/) - Existing Rust ORM for reference
- [Diesel](https://diesel.rs/) - Existing Rust ORM for reference
- [Armature](https://github.com/pegasusheavy/armature) - Pegasus Heavy Industries HTTP framework for Rust
