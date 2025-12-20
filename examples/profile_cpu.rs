//! CPU profiling example for Prax ORM using pprof.
//!
//! Run with:
//! ```bash
//! cargo run --profile profiling --features cpu-profiling --example profile_cpu
//! ```
//!
//! This will generate `profile/cpu_flamegraph.svg`.

#[cfg(feature = "cpu-profiling")]
fn main() {
    use pprof::ProfilerGuard;
    use std::fs::File;

    // Ensure profile directory exists
    std::fs::create_dir_all("profile").expect("Failed to create profile directory");

    println!("Starting CPU profiling...\n");

    // Start profiler
    let guard = ProfilerGuard::new(100).expect("Failed to start profiler");

    // Run workload
    run_cpu_intensive_workload();

    // Generate reports
    match guard.report().build() {
        Ok(report) => {
            // Generate flamegraph SVG
            let file = File::create("profile/cpu_flamegraph.svg")
                .expect("Failed to create flamegraph file");
            report.flamegraph(file).expect("Failed to write flamegraph");
            println!("\nFlamegraph written to profile/cpu_flamegraph.svg");

            // Note: For pprof protobuf output, use the `protobuf-codec` feature
            // and the protobuf crate. The flamegraph SVG is sufficient for most use cases.
            println!("\nFor interactive analysis, open profile/cpu_flamegraph.svg in a browser");
        }
        Err(e) => eprintln!("Failed to generate report: {}", e),
    }
}

#[cfg(not(feature = "cpu-profiling"))]
fn main() {
    eprintln!("CPU profiling requires --features cpu-profiling");
    eprintln!("Run with: cargo run --profile profiling --features cpu-profiling --example profile_cpu");
    std::process::exit(1);
}

fn run_cpu_intensive_workload() {
    println!("Running CPU-intensive workload...");

    // Schema parsing (CPU-intensive due to parsing)
    cpu_intensive_parsing();

    // Query building (CPU-intensive due to string operations)
    cpu_intensive_query_building();

    // Filter construction (CPU-intensive due to allocations)
    cpu_intensive_filters();

    // String processing (common bottleneck)
    cpu_intensive_strings();

    println!("Workload complete!");
}

fn cpu_intensive_parsing() {
    use prax_schema::parser::parse_schema;

    let schema = generate_large_schema(50);

    for _ in 0..20 {
        let _ = parse_schema(&schema);
    }
    println!("  Completed schema parsing");
}

fn cpu_intensive_query_building() {
    use prax_query::raw::Sql;
    use prax_query::sql::DatabaseType;

    for _ in 0..1000 {
        let mut sql = Sql::new("INSERT INTO users (")
            .with_db_type(DatabaseType::PostgreSQL);

        // Build a query with many columns
        for i in 0..20 {
            if i > 0 {
                sql = sql.push(", ");
            }
            sql = sql.push(format!("col{}", i));
        }

        sql = sql.push(") VALUES (");

        for i in 0..20 {
            if i > 0 {
                sql = sql.push(", ");
            }
            sql = sql.bind(format!("value{}", i));
        }

        sql = sql.push(") RETURNING *");
        let _ = sql.build();
    }
    println!("  Completed query building");
}

fn cpu_intensive_filters() {
    use prax_query::filter::{Filter, FilterValue};

    for _ in 0..1000 {
        // Build deeply nested filters
        let filter = Filter::and([
            Filter::or([
                Filter::and([
                    Filter::Equals("a".into(), FilterValue::Int(1)),
                    Filter::Equals("b".into(), FilterValue::Int(2)),
                ]),
                Filter::and([
                    Filter::Equals("c".into(), FilterValue::Int(3)),
                    Filter::Equals("d".into(), FilterValue::Int(4)),
                ]),
            ]),
            Filter::Not(Box::new(Filter::Equals(
                "deleted".into(),
                FilterValue::Bool(true),
            ))),
            Filter::In(
                "status".into(),
                (0..10).map(|i| FilterValue::String(format!("status_{}", i))).collect(),
            ),
        ]);

        // Clone to simulate usage
        let _ = filter.clone();
    }
    println!("  Completed filter construction");
}

fn cpu_intensive_strings() {
    // Simulate string-heavy operations common in ORMs
    for _ in 0..1000 {
        let mut result = String::with_capacity(1000);
        for i in 0..50 {
            result.push_str(&format!("field_{} = ${}, ", i, i + 1));
        }
        std::hint::black_box(result);
    }
    println!("  Completed string processing");
}

fn generate_large_schema(model_count: usize) -> String {
    let mut schema = String::new();

    schema.push_str(r#"
enum Status {
    ACTIVE
    INACTIVE
    PENDING
}

"#);

    for i in 0..model_count {
        schema.push_str(&format!(
            r#"
model Model{i} {{
    id          Int      @id @auto
    name        String
    description String?
    status      Status   @default(ACTIVE)
    value       Float?
    count       Int      @default(0)
    active      Boolean  @default(true)
    createdAt   DateTime @default(now())
    updatedAt   DateTime @updatedAt

    @@index([name])
}}
"#
        ));
    }

    schema
}

