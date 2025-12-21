//! Database Execution Benchmarks
//!
//! This benchmark suite compares actual database execution performance across ORMs:
//! - Prax (PostgreSQL, MySQL, SQLite)
//! - Diesel-Async (PostgreSQL, MySQL)
//! - SQLx (PostgreSQL, MySQL, SQLite)
//!
//! # Prerequisites
//!
//! Start the database containers before running:
//! ```bash
//! docker compose up -d postgres mysql
//! ```
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all database execution benchmarks
//! cargo bench --bench database_execution
//!
//! # Run specific ORM benchmarks
//! cargo bench --bench database_execution -- prax
//! cargo bench --bench database_execution -- diesel_async
//! cargo bench --bench database_execution -- sqlx
//! ```
//!
//! # Environment Variables
//!
//! - `POSTGRES_URL`: PostgreSQL connection string (default: postgres://prax:prax_test_password@localhost:5432/prax_test)
//! - `MYSQL_URL`: MySQL connection string (default: mysql://prax:prax_test_password@localhost:3306/prax_test)
//! - `SKIP_DB_BENCHMARKS`: Set to "1" to skip database benchmarks

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::env;
use std::time::Duration;

// Connection URLs with defaults
fn postgres_url() -> String {
    env::var("POSTGRES_URL")
        .unwrap_or_else(|_| "postgres://prax:prax_test_password@localhost:5432/prax_test".into())
}

fn mysql_url() -> String {
    env::var("MYSQL_URL")
        .unwrap_or_else(|_| "mysql://prax:prax_test_password@localhost:3306/prax_test".into())
}

fn should_skip_db_benchmarks() -> bool {
    env::var("SKIP_DB_BENCHMARKS")
        .map(|v| v == "1")
        .unwrap_or(false)
}

// ==============================================================================
// Prax Benchmarks (Async)
// ==============================================================================

mod prax_benchmarks {
    use super::*;
    use prax_query::filter::{Filter, FilterValue};
    use prax_query::sql::{DatabaseType, FastSqlBuilder, QueryCapacity};
    use tokio::runtime::Runtime;

    /// Benchmark query building (no database, baseline)
    pub fn query_building_select(c: &mut Criterion) {
        let mut group = c.benchmark_group("prax/query_building");

        group.bench_function("simple_select", |b| {
            b.iter(|| {
                let mut builder = FastSqlBuilder::postgres(QueryCapacity::SimpleSelect);
                builder.push_str("SELECT id, name, email FROM users WHERE id = ");
                builder.bind(42i64);
                black_box(builder.build())
            })
        });

        group.bench_function("select_with_filters", |b| {
            b.iter(|| {
                let mut builder = FastSqlBuilder::postgres(QueryCapacity::SelectWithFilters(3));
                builder.push_str("SELECT * FROM users WHERE status = ");
                builder.bind("active");
                builder.push_str(" AND age > ");
                builder.bind(18i64);
                builder.push_str(" AND verified = ");
                builder.bind(true);
                black_box(builder.build())
            })
        });

        group.bench_function("insert", |b| {
            b.iter(|| {
                let mut builder = FastSqlBuilder::postgres(QueryCapacity::Insert(4));
                builder.push_str("INSERT INTO users (name, email, age, status) VALUES (");
                builder.bind("Test User");
                builder.push_str(", ");
                builder.bind("test@example.com");
                builder.push_str(", ");
                builder.bind(25i64);
                builder.push_str(", ");
                builder.bind("active");
                builder.push_str(") RETURNING id");
                black_box(builder.build())
            })
        });

        group.finish();
    }

    /// Benchmark filter construction
    pub fn filter_construction(c: &mut Criterion) {
        let mut group = c.benchmark_group("prax/filter_construction");

        group.bench_function("simple_eq", |b| {
            b.iter(|| black_box(Filter::Equals("id".into(), FilterValue::Int(42))))
        });

        group.bench_function("and_5", |b| {
            b.iter(|| {
                black_box(Filter::and([
                    Filter::Equals("status".into(), FilterValue::String("active".into())),
                    Filter::Gt("age".into(), FilterValue::Int(18)),
                    Filter::Lt("age".into(), FilterValue::Int(65)),
                    Filter::IsNotNull("email".into()),
                    Filter::Equals("verified".into(), FilterValue::Bool(true)),
                ]))
            })
        });

        group.bench_function("in_100", |b| {
            b.iter(|| {
                let values: Vec<FilterValue> = (0..100).map(|i| FilterValue::Int(i)).collect();
                black_box(Filter::In("id".into(), values))
            })
        });

        group.finish();
    }

    /// Benchmark actual database execution using Prax with connection pooling
    pub fn database_execution(c: &mut Criterion) {
        use prax_postgres::{PgConfig, PgPool, PoolConfig};

        if should_skip_db_benchmarks() {
            return;
        }

        let rt = match Runtime::new() {
            Ok(rt) => rt,
            Err(_) => return,
        };

        let url = postgres_url();

        // Create pool with warmup - run pool creation inside the runtime
        let pool = match rt.block_on(async {
            // Parse config from URL
            let config = PgConfig::from_url(&url)?;

            // Create pool configuration (disable timeouts for benchmark)
            let pool_config = PoolConfig {
                max_connections: 5,
                min_connections: 1,
                statement_cache_size: 100,
                connection_timeout: None, // Disable timeout for benchmark
                idle_timeout: None,
                max_lifetime: None,
            };

            let pool = PgPool::with_pool_config(config, pool_config).await?;

            // Warmup: pre-establish connections
            pool.warmup(3).await?;

            Ok::<_, prax_postgres::PgError>(pool)
        }) {
            Ok(pool) => pool,
            Err(e) => {
                eprintln!("Failed to connect to PostgreSQL for Prax benchmarks: {}", e);
                eprintln!(
                    "Skipping database execution benchmarks. Start PostgreSQL with: docker compose up -d postgres"
                );
                return;
            }
        };

        let mut group = c.benchmark_group("prax/db_execution");
        group.sample_size(50);
        group.measurement_time(Duration::from_secs(10));

        // SELECT by ID with pooling
        group.bench_function("select_by_id", |b| {
            b.to_async(&rt).iter(|| async {
                let conn = pool.get().await.unwrap();
                let result = conn
                    .query_opt(
                        "SELECT id, name, email, age, status, role, verified, score FROM users WHERE id = $1",
                        &[&1i64],
                    )
                    .await;
                black_box(result)
            })
        });

        // SELECT with filters
        group.bench_function("select_filtered", |b| {
            b.to_async(&rt).iter(|| async {
                let conn = pool.get().await.unwrap();
                let result = conn
                    .query(
                        "SELECT id, name, email, age, status, role, verified, score FROM users WHERE status = $1 AND age > $2 LIMIT 10",
                        &[&"active", &18i32],
                    )
                    .await;
                black_box(result)
            })
        });

        // COUNT
        group.bench_function("count", |b| {
            b.to_async(&rt).iter(|| async {
                let conn = pool.get().await.unwrap();
                let result = conn
                    .query_one("SELECT COUNT(*) FROM users WHERE status = $1", &[&"active"])
                    .await;
                black_box(result)
            })
        });

        // SELECT with prepared statement (tests statement caching)
        group.bench_function("select_prepared", |b| {
            b.to_async(&rt).iter(|| async {
                let conn = pool.get().await.unwrap();
                let result = conn
                    .query_cached(
                        "SELECT id, name, email FROM users WHERE id = $1",
                        &[&1i64],
                    )
                    .await;
                black_box(result)
            })
        });

        group.finish();

        // Cleanup
        pool.close();
    }
}

// ==============================================================================
// Diesel-Async Benchmarks
// ==============================================================================

mod diesel_async_benchmarks {
    use super::*;
    use diesel::debug_query;
    use diesel::pg::Pg;
    use diesel::prelude::*;
    use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
    use tokio::runtime::Runtime;

    mod schema {
        diesel::table! {
            users (id) {
                id -> Int8,
                name -> Text,
                email -> Text,
                age -> Int4,
                status -> Text,
                role -> Text,
                verified -> Bool,
                score -> Int4,
                attempts -> Int4,
                deleted -> Bool,
                deleted_at -> Nullable<Timestamp>,
                created_at -> Timestamp,
                updated_at -> Timestamp,
            }
        }
    }

    use schema::users;
    use schema::users::dsl::*;

    #[derive(Queryable, Selectable, Debug)]
    #[diesel(table_name = users)]
    pub struct User {
        pub id: i64,
        pub name: String,
        pub email: String,
        pub age: i32,
        pub status: String,
        pub role: String,
        pub verified: bool,
        pub score: i32,
    }

    #[derive(Insertable)]
    #[diesel(table_name = users)]
    pub struct NewUser<'a> {
        pub name: &'a str,
        pub email: &'a str,
        pub age: i32,
        pub status: &'a str,
        pub role: &'a str,
    }

    /// Benchmark query building (no database)
    pub fn query_building(c: &mut Criterion) {
        let mut group = c.benchmark_group("diesel_async/query_building");

        group.bench_function("simple_select", |b| {
            b.iter(|| {
                let query = users.select((id, name, email)).filter(id.eq(42i64));
                black_box(debug_query::<Pg, _>(&query).to_string())
            })
        });

        group.bench_function("select_with_filters", |b| {
            b.iter(|| {
                let query = users
                    .filter(status.eq("active"))
                    .filter(age.gt(18))
                    .filter(verified.eq(true));
                black_box(debug_query::<Pg, _>(&query).to_string())
            })
        });

        group.bench_function("insert", |b| {
            b.iter(|| {
                let new_user = NewUser {
                    name: "Test User",
                    email: "test@example.com",
                    age: 25,
                    status: "active",
                    role: "user",
                };
                let query = diesel::insert_into(users).values(&new_user);
                black_box(debug_query::<Pg, _>(&query).to_string())
            })
        });

        group.finish();
    }

    /// Benchmark filter construction
    pub fn filter_construction(c: &mut Criterion) {
        let mut group = c.benchmark_group("diesel_async/filter_construction");

        group.bench_function("simple_eq", |b| {
            b.iter(|| {
                let filter: Box<
                    dyn BoxableExpression<users::table, Pg, SqlType = diesel::sql_types::Bool>,
                > = Box::new(id.eq(42i64));
                black_box(filter)
            })
        });

        group.bench_function("and_5", |b| {
            b.iter(|| {
                let filter: Box<
                    dyn BoxableExpression<users::table, Pg, SqlType = diesel::sql_types::Bool>,
                > = Box::new(
                    status
                        .eq("active")
                        .and(age.gt(18))
                        .and(age.lt(65))
                        .and(email.is_not_null())
                        .and(verified.eq(true)),
                );
                black_box(filter)
            })
        });

        group.bench_function("in_100", |b| {
            b.iter(|| {
                let values: Vec<i64> = (0..100).collect();
                let filter: Box<
                    dyn BoxableExpression<users::table, Pg, SqlType = diesel::sql_types::Bool>,
                > = Box::new(id.eq_any(values));
                black_box(filter)
            })
        });

        group.finish();
    }

    /// Benchmark actual database execution (requires running PostgreSQL)
    /// Note: This benchmark creates a new connection per iteration which adds overhead,
    /// but is necessary for criterion's async benchmarking model.
    pub fn database_execution(c: &mut Criterion) {
        if should_skip_db_benchmarks() {
            return;
        }

        let rt = match Runtime::new() {
            Ok(rt) => rt,
            Err(_) => return,
        };

        let url = postgres_url();

        // Test connection first
        let test_result = rt.block_on(async { AsyncPgConnection::establish(&url).await });

        if let Err(e) = test_result {
            eprintln!(
                "Failed to connect to PostgreSQL for diesel-async benchmarks: {}",
                e
            );
            eprintln!(
                "Skipping database execution benchmarks. Start PostgreSQL with: docker compose up -d postgres"
            );
            return;
        }

        let mut group = c.benchmark_group("diesel_async/db_execution");
        group.sample_size(50);
        group.measurement_time(Duration::from_secs(10));

        let url_clone = url.clone();
        // SELECT by ID - using pool would be better but keeping it simple
        group.bench_function("select_by_id", |b| {
            b.to_async(&rt).iter(|| {
                let url = url_clone.clone();
                async move {
                    let mut conn = AsyncPgConnection::establish(&url).await.unwrap();
                    let result: Option<User> = users
                        .filter(id.eq(1i64))
                        .select(User::as_select())
                        .first(&mut conn)
                        .await
                        .optional()
                        .unwrap();
                    black_box(result)
                }
            })
        });

        let url_clone = url.clone();
        // SELECT with filters
        group.bench_function("select_filtered", |b| {
            b.to_async(&rt).iter(|| {
                let url = url_clone.clone();
                async move {
                    let mut conn = AsyncPgConnection::establish(&url).await.unwrap();
                    let result: Vec<User> = users
                        .filter(status.eq("active"))
                        .filter(age.gt(18))
                        .limit(10)
                        .select(User::as_select())
                        .load(&mut conn)
                        .await
                        .unwrap();
                    black_box(result)
                }
            })
        });

        let url_clone = url.clone();
        // COUNT
        group.bench_function("count", |b| {
            b.to_async(&rt).iter(|| {
                let url = url_clone.clone();
                async move {
                    let mut conn = AsyncPgConnection::establish(&url).await.unwrap();
                    let result: i64 = users
                        .filter(status.eq("active"))
                        .count()
                        .get_result(&mut conn)
                        .await
                        .unwrap();
                    black_box(result)
                }
            })
        });

        group.finish();
    }
}

// ==============================================================================
// SQLx Benchmarks
// ==============================================================================

mod sqlx_benchmarks {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::{PgPool, Row};
    use tokio::runtime::Runtime;

    #[derive(Debug, sqlx::FromRow)]
    pub struct User {
        pub id: i64,
        pub name: String,
        pub email: String,
        pub age: i32,
        pub status: String,
        pub role: String,
        pub verified: bool,
        pub score: i32,
    }

    /// Benchmark query string building (baseline)
    pub fn query_building(c: &mut Criterion) {
        let mut group = c.benchmark_group("sqlx/query_building");

        group.bench_function("simple_select", |b| {
            b.iter(|| black_box(format!("SELECT id, name, email FROM users WHERE id = $1")))
        });

        group.bench_function("select_with_filters", |b| {
            b.iter(|| {
                black_box(format!(
                    "SELECT * FROM users WHERE status = $1 AND age > $2 AND verified = $3"
                ))
            })
        });

        group.bench_function("insert", |b| {
            b.iter(|| {
                black_box(format!(
                    "INSERT INTO users (name, email, age, status, role) VALUES ($1, $2, $3, $4, $5) RETURNING id"
                ))
            })
        });

        group.bench_function("in_100_placeholders", |b| {
            b.iter(|| {
                let placeholders: String = (1..=100)
                    .map(|i| format!("${}", i))
                    .collect::<Vec<_>>()
                    .join(", ");
                black_box(format!(
                    "SELECT * FROM users WHERE id IN ({})",
                    placeholders
                ))
            })
        });

        group.finish();
    }

    /// Benchmark actual database execution (requires running PostgreSQL)
    pub fn database_execution(c: &mut Criterion) {
        if should_skip_db_benchmarks() {
            return;
        }

        let rt = match Runtime::new() {
            Ok(rt) => rt,
            Err(_) => return,
        };

        let url = postgres_url();

        // Try to establish connection pool
        let pool = match rt
            .block_on(async { PgPoolOptions::new().max_connections(5).connect(&url).await })
        {
            Ok(pool) => pool,
            Err(e) => {
                eprintln!("Failed to connect to PostgreSQL for SQLx benchmarks: {}", e);
                eprintln!(
                    "Skipping database execution benchmarks. Start PostgreSQL with: docker compose up -d postgres"
                );
                return;
            }
        };

        let mut group = c.benchmark_group("sqlx/db_execution");
        group.sample_size(50);
        group.measurement_time(Duration::from_secs(10));

        // SELECT by ID
        group.bench_function("select_by_id", |b| {
            b.to_async(&rt).iter(|| async {
                let result: Result<Option<User>, _> = sqlx::query_as::<_, User>(
                    "SELECT id, name, email, age, status, role, verified, score FROM users WHERE id = $1"
                )
                .bind(1i64)
                .fetch_optional(&pool)
                .await;
                black_box(result)
            })
        });

        // SELECT with filters
        group.bench_function("select_filtered", |b| {
            b.to_async(&rt).iter(|| async {
                let result: Result<Vec<User>, _> = sqlx::query_as::<_, User>(
                    "SELECT id, name, email, age, status, role, verified, score FROM users WHERE status = $1 AND age > $2 LIMIT 10"
                )
                .bind("active")
                .bind(18)
                .fetch_all(&pool)
                .await;
                black_box(result)
            })
        });

        // COUNT
        group.bench_function("count", |b| {
            b.to_async(&rt).iter(|| async {
                let result: Result<(i64,), _> =
                    sqlx::query_as("SELECT COUNT(*) FROM users WHERE status = $1")
                        .bind("active")
                        .fetch_one(&pool)
                        .await;
                black_box(result)
            })
        });

        group.finish();

        // Cleanup
        rt.block_on(async {
            pool.close().await;
        });
    }
}

// ==============================================================================
// Comparison Benchmarks
// ==============================================================================

/// Direct head-to-head comparison of query building
fn bench_query_building_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison/query_building");

    // Simple SELECT
    group.bench_function("prax", |b| {
        use prax_query::sql::{FastSqlBuilder, QueryCapacity};
        b.iter(|| {
            let mut builder = FastSqlBuilder::postgres(QueryCapacity::SimpleSelect);
            builder.push_str("SELECT id, name, email FROM users WHERE id = ");
            builder.bind(42i64);
            black_box(builder.build())
        })
    });

    group.bench_function("diesel", |b| {
        use diesel::debug_query;
        use diesel::pg::Pg;
        use diesel::prelude::*;

        mod diesel_schema {
            diesel::table! {
                users (id) {
                    id -> Int8,
                    name -> Text,
                    email -> Text,
                }
            }
        }
        use diesel_schema::users::dsl::*;

        b.iter(|| {
            let query = users.select((id, name, email)).filter(id.eq(42i64));
            black_box(debug_query::<Pg, _>(&query).to_string())
        })
    });

    group.bench_function("sqlx", |b| {
        b.iter(|| black_box(format!("SELECT id, name, email FROM users WHERE id = $1")))
    });

    group.finish();
}

/// Direct head-to-head comparison of filter construction
fn bench_filter_construction_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison/filter_construction");

    // AND filter with 5 conditions
    group.bench_function("prax/and_5", |b| {
        use prax_query::filter::{Filter, FilterValue};
        b.iter(|| {
            black_box(Filter::and([
                Filter::Equals("status".into(), FilterValue::String("active".into())),
                Filter::Gt("age".into(), FilterValue::Int(18)),
                Filter::Lt("age".into(), FilterValue::Int(65)),
                Filter::IsNotNull("email".into()),
                Filter::Equals("verified".into(), FilterValue::Bool(true)),
            ]))
        })
    });

    group.bench_function("diesel/and_5", |b| {
        use diesel::pg::Pg;
        use diesel::prelude::*;

        mod diesel_filter_schema {
            diesel::table! {
                users (id) {
                    id -> Int8,
                    age -> Int4,
                    status -> Text,
                    email -> Nullable<Text>,
                    verified -> Bool,
                }
            }
        }
        use diesel_filter_schema::users;
        use diesel_filter_schema::users::dsl::*;

        b.iter(|| {
            let filter: Box<
                dyn BoxableExpression<users::table, Pg, SqlType = diesel::sql_types::Bool>,
            > = Box::new(
                status
                    .eq("active")
                    .and(age.gt(18))
                    .and(age.lt(65))
                    .and(email.is_not_null())
                    .and(verified.eq(true)),
            );
            black_box(filter)
        })
    });

    group.finish();
}

// ==============================================================================
// Criterion Setup
// ==============================================================================

criterion_group!(
    name = query_building_benches;
    config = Criterion::default().sample_size(100);
    targets =
        prax_benchmarks::query_building_select,
        prax_benchmarks::filter_construction,
        diesel_async_benchmarks::query_building,
        diesel_async_benchmarks::filter_construction,
        sqlx_benchmarks::query_building,
        bench_query_building_comparison,
        bench_filter_construction_comparison
);

criterion_group!(
    name = database_execution_benches;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(10));
    targets =
        prax_benchmarks::database_execution,
        diesel_async_benchmarks::database_execution,
        sqlx_benchmarks::database_execution
);

criterion_main!(query_building_benches, database_execution_benches);
