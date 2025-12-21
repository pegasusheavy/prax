#![allow(dead_code, unused, clippy::type_complexity)]
//! Tracing-based profiling example for Prax ORM.
//!
//! Run with:
//! ```bash
//! # Generate flame graph data
//! cargo run --features profiling --example profile_tracing -- --flame
//!
//! # Generate Chrome trace
//! cargo run --features profiling --example profile_tracing -- --chrome
//! ```

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("--chrome");

    match mode {
        "--flame" => run_with_flame_profiling(),
        "--chrome" => run_with_chrome_profiling(),
        _ => {
            eprintln!("Usage: profile_tracing [--flame|--chrome]");
            eprintln!("  --flame  Generate flame graph data (profile/tracing.folded)");
            eprintln!("  --chrome Generate Chrome trace (profile/trace.json)");
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "profiling")]
fn run_with_flame_profiling() {
    use tracing_flame::FlameLayer;
    use tracing_subscriber::prelude::*;

    // Ensure profile directory exists
    std::fs::create_dir_all("profile").expect("Failed to create profile directory");

    let (flame_layer, guard) =
        FlameLayer::with_file("profile/tracing.folded").expect("Failed to create flame layer");

    tracing_subscriber::registry().with(flame_layer).init();

    println!("Running with flame profiling...");
    run_workload();

    drop(guard);
    println!("\nFlame graph data written to profile/tracing.folded");
    println!(
        "Generate SVG with: cat profile/tracing.folded | inferno-flamegraph > profile/tracing.svg"
    );
}

#[cfg(not(feature = "profiling"))]
fn run_with_flame_profiling() {
    eprintln!("Flame profiling requires --features profiling");
    std::process::exit(1);
}

#[cfg(feature = "profiling")]
fn run_with_chrome_profiling() {
    use tracing_chrome::ChromeLayerBuilder;
    use tracing_subscriber::prelude::*;

    // Ensure profile directory exists
    std::fs::create_dir_all("profile").expect("Failed to create profile directory");

    let (chrome_layer, guard) = ChromeLayerBuilder::new().file("profile/trace.json").build();

    tracing_subscriber::registry().with(chrome_layer).init();

    println!("Running with Chrome trace profiling...");
    run_workload();

    drop(guard);
    println!("\nChrome trace written to profile/trace.json");
    println!("View in Chrome: chrome://tracing");
}

#[cfg(not(feature = "profiling"))]
fn run_with_chrome_profiling() {
    eprintln!("Chrome profiling requires --features profiling");
    std::process::exit(1);
}

#[tracing::instrument]
fn run_workload() {
    println!("Starting workload...\n");

    // Schema parsing workload
    profile_schema_parsing();

    // Query building workload
    profile_query_building();

    // Filter construction workload
    profile_filters();

    println!("\nWorkload complete!");
}

#[tracing::instrument]
fn profile_schema_parsing() {
    let schema = r#"
        model User {
            id        Int      @id @auto
            email     String   @unique
            name      String?
            posts     Post[]
            createdAt DateTime @default(now())
        }

        model Post {
            id        Int      @id @auto
            title     String
            content   String?
            author    User     @relation(fields: [authorId], references: [id])
            authorId  Int
        }
    "#;

    for _ in 0..50 {
        let _ = parse_schema_work(schema);
    }
    println!("  Completed 50 schema parse iterations");
}

#[tracing::instrument(skip(schema))]
fn parse_schema_work(schema: &str) -> bool {
    prax_schema::parser::parse_schema(schema).is_ok()
}

#[tracing::instrument]
fn profile_query_building() {
    for i in 0..100 {
        let _ = build_query_work(i);
    }
    println!("  Completed 100 query build iterations");
}

#[tracing::instrument]
fn build_query_work(i: i32) -> (String, Vec<prax_query::filter::FilterValue>) {
    prax_query::raw::Sql::new("SELECT u.id, u.name, u.email, p.title ")
        .push("FROM users u ")
        .push("LEFT JOIN posts p ON p.user_id = u.id ")
        .push("WHERE u.active = ")
        .bind(true)
        .push(" AND u.id > ")
        .bind(i)
        .push(" ORDER BY u.created_at DESC ")
        .push("LIMIT ")
        .bind(100)
        .build()
}

#[tracing::instrument]
fn profile_filters() {
    for i in 0..100 {
        let _ = build_filter_work(i);
    }
    println!("  Completed 100 filter build iterations");
}

#[tracing::instrument]
fn build_filter_work(i: i64) -> prax_query::filter::Filter {
    use prax_query::filter::{Filter, FilterValue};

    Filter::and([
        Filter::Equals("id".into(), FilterValue::Int(i)),
        Filter::or([
            Filter::Equals("status".into(), FilterValue::String("active".into())),
            Filter::Equals("status".into(), FilterValue::String("pending".to_string())),
        ]),
        Filter::Not(Box::new(Filter::Equals(
            "deleted".into(),
            FilterValue::Bool(true),
        ))),
        Filter::Gt("created_at".into(), FilterValue::Int(1000)),
    ])
}
