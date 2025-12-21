//! ORM Comparison Profiling Example
//!
//! This example profiles Prax against other Rust ORMs for query building
//! and filter construction performance. Use with profiling tools like:
//!
//! ```bash
//! # CPU profiling with flamegraph
//! cargo run --example profile_orm_comparison --release --features cpu-profiling
//!
//! # Memory profiling with dhat
//! cargo run --example profile_orm_comparison --release --features heap-profiling
//!
//! # Quick benchmark run
//! cargo run --example profile_orm_comparison --release
//! ```
//!
//! # Results Interpretation
//!
//! The output shows timing comparisons for:
//! - Query building (SQL string construction)
//! - Filter construction (condition building)
//! - Scaling behavior (varying sizes)

use std::time::{Duration, Instant};

const ITERATIONS: u32 = 100_000;

fn main() {
    println!("🔬 Prax ORM Comparison Profiler\n");
    println!("Running {} iterations per benchmark\n", ITERATIONS);

    println!("═══════════════════════════════════════════════════════════════");
    println!("  QUERY BUILDING PERFORMANCE");
    println!("═══════════════════════════════════════════════════════════════\n");

    profile_query_building();

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("  FILTER CONSTRUCTION PERFORMANCE");
    println!("═══════════════════════════════════════════════════════════════\n");

    profile_filter_construction();

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("  SCALING ANALYSIS");
    println!("═══════════════════════════════════════════════════════════════\n");

    profile_scaling();

    println!("\n✅ Profiling complete!");
}

fn profile_query_building() {
    use prax_query::raw::Sql;
    use prax_query::sql::DatabaseType;

    // Prax simple SELECT
    let prax_simple = measure(ITERATIONS, || {
        let sql = Sql::new("SELECT id, name, email FROM users WHERE id = ").bind(42i64);
        std::hint::black_box(sql.build())
    });

    // Prax complex SELECT
    let prax_complex = measure(ITERATIONS, || {
        let sql = Sql::new("SELECT * FROM users WHERE ")
            .push("status = ")
            .bind("active")
            .push(" AND age > ")
            .bind(18i64)
            .push(" AND created_at > ")
            .bind("2024-01-01")
            .push(" ORDER BY created_at DESC LIMIT ")
            .bind(10i64);
        std::hint::black_box(sql.build())
    });

    // Prax PostgreSQL
    let prax_postgres = measure(ITERATIONS, || {
        let sql = Sql::new("SELECT * FROM users WHERE id = ")
            .with_db_type(DatabaseType::PostgreSQL)
            .bind(42i64);
        std::hint::black_box(sql.build())
    });

    // Prax MySQL
    let prax_mysql = measure(ITERATIONS, || {
        let sql = Sql::new("SELECT * FROM users WHERE id = ")
            .with_db_type(DatabaseType::MySQL)
            .bind(42i64);
        std::hint::black_box(sql.build())
    });

    // Prax SQLite
    let prax_sqlite = measure(ITERATIONS, || {
        let sql = Sql::new("SELECT * FROM users WHERE id = ")
            .with_db_type(DatabaseType::SQLite)
            .bind(42i64);
        std::hint::black_box(sql.build())
    });

    println!("Query Building Results:");
    println!(
        "  Prax simple SELECT:  {:>8.2}ns",
        prax_simple.as_nanos() as f64 / ITERATIONS as f64
    );
    println!(
        "  Prax complex SELECT: {:>8.2}ns",
        prax_complex.as_nanos() as f64 / ITERATIONS as f64
    );
    println!(
        "  Prax PostgreSQL:     {:>8.2}ns",
        prax_postgres.as_nanos() as f64 / ITERATIONS as f64
    );
    println!(
        "  Prax MySQL:          {:>8.2}ns",
        prax_mysql.as_nanos() as f64 / ITERATIONS as f64
    );
    println!(
        "  Prax SQLite:         {:>8.2}ns",
        prax_sqlite.as_nanos() as f64 / ITERATIONS as f64
    );
}

fn profile_filter_construction() {
    use prax_query::filter::{Filter, FilterValue};

    // Simple filter
    let simple = measure(ITERATIONS, || {
        std::hint::black_box(Filter::Equals("id".into(), FilterValue::Int(42)))
    });

    // AND filter (5 conditions)
    let and_5 = measure(ITERATIONS, || {
        std::hint::black_box(Filter::and([
            Filter::Equals("status".into(), FilterValue::String("active".into())),
            Filter::Gt("age".into(), FilterValue::Int(18)),
            Filter::Lt("age".into(), FilterValue::Int(65)),
            Filter::IsNotNull("email".into()),
            Filter::Equals("verified".into(), FilterValue::Bool(true)),
        ]))
    });

    // AND filter (10 conditions)
    let and_10 = measure(ITERATIONS, || {
        std::hint::black_box(Filter::and([
            Filter::Equals("status".into(), FilterValue::String("active".into())),
            Filter::Gt("age".into(), FilterValue::Int(18)),
            Filter::Lt("age".into(), FilterValue::Int(65)),
            Filter::IsNotNull("email".into()),
            Filter::Equals("verified".into(), FilterValue::Bool(true)),
            Filter::Contains("name".into(), FilterValue::String("John".into())),
            Filter::Gte("score".into(), FilterValue::Int(100)),
            Filter::Lte("attempts".into(), FilterValue::Int(3)),
            Filter::NotEquals("role".into(), FilterValue::String("banned".into())),
            Filter::IsNull("deleted_at".into()),
        ]))
    });

    // OR filter (5 conditions)
    let or_5 = measure(ITERATIONS, || {
        std::hint::black_box(Filter::or([
            Filter::Equals("role".into(), FilterValue::String("admin".into())),
            Filter::Equals("role".into(), FilterValue::String("moderator".into())),
            Filter::Equals("role".into(), FilterValue::String("editor".into())),
            Filter::Equals("role".into(), FilterValue::String("author".into())),
            Filter::Equals("role".into(), FilterValue::String("contributor".into())),
        ]))
    });

    // IN filter (100 values)
    let in_100 = measure(ITERATIONS, || {
        let values: Vec<FilterValue> = (0..100).map(|i| FilterValue::Int(i)).collect();
        std::hint::black_box(Filter::In("id".into(), values.into()))
    });

    // Complex nested filter
    let complex = measure(ITERATIONS, || {
        std::hint::black_box(Filter::and([
            Filter::or([
                Filter::and([
                    Filter::Equals("status".into(), FilterValue::String("active".into())),
                    Filter::Gt("score".into(), FilterValue::Int(100)),
                ]),
                Filter::and([
                    Filter::Equals("role".into(), FilterValue::String("admin".into())),
                    Filter::Equals("verified".into(), FilterValue::Bool(true)),
                ]),
            ]),
            Filter::Not(Box::new(Filter::Equals(
                "deleted".into(),
                FilterValue::Bool(true),
            ))),
            Filter::IsNotNull("email".into()),
        ]))
    });

    println!("Filter Construction Results:");
    println!(
        "  Simple filter:       {:>8.2}ns",
        simple.as_nanos() as f64 / ITERATIONS as f64
    );
    println!(
        "  AND (5 conditions):  {:>8.2}ns",
        and_5.as_nanos() as f64 / ITERATIONS as f64
    );
    println!(
        "  AND (10 conditions): {:>8.2}ns",
        and_10.as_nanos() as f64 / ITERATIONS as f64
    );
    println!(
        "  OR (5 conditions):   {:>8.2}ns",
        or_5.as_nanos() as f64 / ITERATIONS as f64
    );
    println!(
        "  IN (100 values):     {:>8.2}ns",
        in_100.as_nanos() as f64 / ITERATIONS as f64
    );
    println!(
        "  Complex nested:      {:>8.2}ns",
        complex.as_nanos() as f64 / ITERATIONS as f64
    );
}

fn profile_scaling() {
    use prax_query::filter::{Filter, FilterValue};

    println!("AND Filter Scaling:");
    for size in [1, 5, 10, 25, 50, 100] {
        let duration = measure(ITERATIONS / 10, || {
            let filters: Vec<_> = (0..size)
                .map(|i| Filter::Equals(format!("field_{}", i).into(), FilterValue::Int(i as i64)))
                .collect();
            std::hint::black_box(Filter::and(filters))
        });
        let avg_ns = duration.as_nanos() as f64 / (ITERATIONS / 10) as f64;
        let per_condition = avg_ns / size as f64;
        println!(
            "  {} conditions: {:>10.2}ns total, {:>6.2}ns/condition",
            size, avg_ns, per_condition
        );
    }

    println!("\nIN Filter Scaling:");
    for size in [1, 5, 10, 25, 50, 100, 500, 1000] {
        let duration = measure(ITERATIONS / 10, || {
            let values: Vec<FilterValue> = (0..size).map(|i| FilterValue::Int(i as i64)).collect();
            std::hint::black_box(Filter::In("id".into(), values.into()))
        });
        let avg_ns = duration.as_nanos() as f64 / (ITERATIONS / 10) as f64;
        let per_value = avg_ns / size as f64;
        println!(
            "  {} values: {:>10.2}ns total, {:>6.2}ns/value",
            size, avg_ns, per_value
        );
    }
}

fn measure<F, R>(iterations: u32, f: F) -> Duration
where
    F: Fn() -> R,
{
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = f();
    }
    start.elapsed()
}
