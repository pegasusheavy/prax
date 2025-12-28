# Prax ORM - Feature Reference

A full-featured Prisma-like ORM for Rust with async support.

---

## 🏗️ Architecture

```
prax/
├── prax-schema/         # Schema parser and AST
├── prax-codegen/        # Proc-macro code generation
├── prax-query/          # Query builder + optimizations
├── prax-postgres/       # PostgreSQL driver
├── prax-mysql/          # MySQL driver
├── prax-sqlite/         # SQLite driver
├── prax-mssql/          # MSSQL driver
├── prax-mongodb/        # MongoDB driver
├── prax-duckdb/         # DuckDB analytical driver
├── prax-scylladb/       # ScyllaDB driver
├── prax-migrate/        # Migration engine
├── prax-cli/            # CLI tool
├── prax-armature/       # Armature integration
├── prax-axum/           # Axum integration
└── prax-actix/          # Actix-web integration
```

**Planned Crates:**
`prax-tidb`, `prax-mariadb`, `prax-redshift`, `prax-cockroachdb`, `prax-bigquery`, `prax-oracle`, `prax-cassandra`, `prax-supabase`, `prax-trino`, `prax-couchdb`, `prax-sqlanywhere`, `prax-surrealdb`

---

## 📊 Database Support Matrix

| Feature | PostgreSQL | MySQL | SQLite | MSSQL | MongoDB | DuckDB | ScyllaDB |
|---------|------------|-------|--------|-------|---------|--------|----------|
| CRUD Operations | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Transactions | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | LWT |
| Connection Pooling | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Migrations | ✅ | ✅ | ✅ | ✅ | ✅ | - | - |
| Schema Introspection | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | - |
| Embedded Mode | - | - | ✅ | - | - | ✅ | - |

*LWT = Lightweight Transactions*

### Advanced Query Features

| Feature | PostgreSQL | MySQL | SQLite | MSSQL | MongoDB |
|---------|------------|-------|--------|-------|---------|
| Views & Materialized Views | ✅ | ✅ | ✅ | ✅ | ✅ |
| Stored Procedures | ✅ | ✅ | - | ✅ | - |
| Triggers | ✅ | ✅ | ✅ | ✅ | ✅ |
| CTEs (WITH clause) | ✅ | ✅ | ✅ | ✅ | - |
| Window Functions | ✅ | ✅ | ✅ | ✅ | ✅ |
| Full-Text Search | ✅ | ✅ | ✅ | ✅ | ✅ |
| JSON Operations | ✅ | ✅ | ✅ | ✅ | ✅ |
| Upsert/Merge | ✅ | ✅ | ✅ | ✅ | ✅ |
| Row-Level Security | ✅ | - | - | ✅ | ✅ |
| Partitioning | ✅ | ✅ | - | ✅ | ✅ |

---

## ✅ Implemented Features

### Core (`prax-query/`)

| Module | Features |
|--------|----------|
| `sql.rs` | Pre-allocated buffers, static fragments, lazy generation, template caching |
| `builder.rs` | SmallVec collections, Cow strings, SmolStr identifiers, builder pooling |
| `db_optimize.rs` | Prepared statement cache, batch tuning, pipeline aggregation, query hints |
| `zero_copy.rs` | Borrowed JSON paths, reference WindowSpec, CTE slices |
| `async_optimize/` | Concurrent introspection, parallel execution, bulk pipelines |
| `mem_optimize/` | String interning, arena allocation, lazy schema parsing |
| `profiling/` | Allocation tracking, leak detection, memory snapshots |

### Multi-Tenancy (`prax-query/src/tenant/`)

| Feature | Module |
|---------|--------|
| Zero-allocation task-local context | `task_local.rs` |
| PostgreSQL RLS integration | `rls.rs` |
| LRU tenant cache with TTL | `cache.rs` |
| Sharded cache for high concurrency | `cache.rs` |
| Per-tenant connection pools | `pool.rs` |
| Statement caching | `prepared.rs` |

### Data Caching (`prax-query/src/data_cache/`)

| Feature | Module |
|---------|--------|
| In-memory LRU cache | `memory.rs` |
| Redis distributed cache | `redis.rs` |
| Tiered L1/L2 caching | `tiered.rs` |
| Pattern/tag invalidation | `invalidation.rs` |
| Cache metrics | `stats.rs` |

### DuckDB Analytics (`prax-duckdb/`)

In-process OLAP database with Parquet/CSV/JSON support, window functions, and async connection pooling.

### ScyllaDB (`prax-scylladb/`)

High-performance Cassandra-compatible driver with prepared statement caching, batch operations (logged/unlogged/counter), and lightweight transactions.

### Memory Profiling (`prax-query/src/profiling/`)

Allocation tracking, memory snapshots, leak detection, heap profiling. CI: `.github/workflows/memory-check.yml`

---

## 🚧 Planned Database Support

### TiDB (`prax-tidb/`)
MySQL-compatible distributed SQL with horizontal scaling, TiFlash HTAP, and placement rules.

### MariaDB (`prax-mariadb/`)
MySQL fork with sequences, system versioning (temporal tables), Oracle mode, ColumnStore, and Galera cluster.

### Amazon Redshift (`prax-redshift/`)
PostgreSQL-based data warehouse with distribution/sort keys, Spectrum (S3 queries), SUPER type (PartiQL), and concurrency scaling.

### CockroachDB (`prax-cockroachdb/`)
PostgreSQL-compatible distributed SQL with geo-partitioning, multi-region, CDC changefeeds, and AS OF SYSTEM TIME queries.

### Google BigQuery (`prax-bigquery/`)
Serverless data warehouse via REST/gRPC API with streaming inserts, partitioned/clustered tables, BQML, and nested STRUCT/ARRAY types.

### Oracle Database (`prax-oracle/`)
Enterprise database via OCI driver with PL/SQL, sequences, flashback queries, RAC support, and Autonomous Database.

### Apache Cassandra (`prax-cassandra/`)
CQL driver (via `scylla` crate) with tunable consistency, token-aware routing, materialized views, UDTs, and CDC.

### Supabase (`prax-supabase/`)
PostgreSQL with realtime WebSocket subscriptions, Auth/RLS integration, Storage API, Edge Functions, and pgvector.

### Trino (`prax-trino/`)
Federated SQL query engine (HTTP protocol) for data lakes—Hive, Iceberg, Delta Lake connectors.

### Apache CouchDB (`prax-couchdb/`)
Document database with HTTP API, Mango queries, MapReduce views, multi-master replication, and changes feed.

### SAP SQL Anywhere (`prax-sqlanywhere/`)
Embedded database via ODBC with MobiLink sync, spatial types, and UltraLite mobile support.

### SurrealDB (`prax-surrealdb/`)
Multi-model database (document/graph/relational) with native Rust driver, SurrealQL, live queries, and embedded mode.

---

## 📊 Benchmarks

| Benchmark | Description |
|-----------|-------------|
| `operations_bench` | Core filter and SQL builder |
| `aggregation_bench` | Aggregation and grouping |
| `pagination_bench` | Cursor and offset pagination |
| `advanced_features_bench` | Window functions, CTEs, subqueries |
| `tenant_bench` | Multi-tenancy overhead |
| `async_bench` | Concurrent execution |
| `mem_optimize_bench` | Interning, arena, lazy parsing |
| `database_bench` | Database-specific SQL |
| `throughput_bench` | Queries-per-second |
| `memory_profile_bench` | Memory profiling |

```bash
cargo bench --package prax-query
```

CI: `.github/workflows/benchmarks.yml`

---

## 📖 Quick Start

```rust
use prax::prelude::*;

#[derive(Model)]
#[prax(table = "users")]
struct User {
    #[prax(id, auto_increment)]
    id: i32,
    email: String,
    name: Option<String>,
}

async fn example(client: &PraxClient) -> Result<()> {
    let users = client
        .user()
        .find_many()
        .where(user::email::contains("@example.com"))
        .order_by(user::created_at::desc())
        .take(10)
        .exec()
        .await?;
    Ok(())
}
```

---

## 📚 References

**Implemented:**
- [Prisma](https://www.prisma.io/docs) | [tokio-postgres](https://docs.rs/tokio-postgres) | [SQLx](https://docs.rs/sqlx) | [Tiberius](https://docs.rs/tiberius) | [mongodb](https://docs.rs/mongodb)
- [DuckDB](https://duckdb.org/) | [duckdb-rs](https://docs.rs/duckdb) | [ScyllaDB](https://www.scylladb.com/) | [scylla-rs](https://docs.rs/scylla)

**Planned:**
- [TiDB](https://www.pingcap.com/tidb/) | [MariaDB](https://mariadb.org/) | [Redshift](https://aws.amazon.com/redshift/) | [CockroachDB](https://www.cockroachlabs.com/)
- [BigQuery](https://cloud.google.com/bigquery) | [Oracle](https://www.oracle.com/database/) | [Cassandra](https://cassandra.apache.org/) | [Supabase](https://supabase.com/)
- [Trino](https://trino.io/) | [CouchDB](https://couchdb.apache.org/) | [SQL Anywhere](https://www.sap.com/products/technology-platform/sql-anywhere.html) | [SurrealDB](https://surrealdb.com/)

**Other ORMs:** [SeaORM](https://www.sea-ql.org/SeaORM/) | [Diesel](https://diesel.rs/)
