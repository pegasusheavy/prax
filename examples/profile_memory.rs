#![allow(dead_code, unused, clippy::type_complexity)]
//! Memory profiling example for Prax ORM.
//!
//! Run with:
//! ```bash
//! cargo run --features heap-profiling --example profile_memory
//! ```
//!
//! This will generate `dhat-heap.json` which can be viewed at:
//! https://nnethercote.github.io/dh_view/dh_view.html

#[cfg(feature = "heap-profiling")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use prax_schema::parser::parse_schema;

fn main() {
    #[cfg(feature = "heap-profiling")]
    let _profiler = dhat::Profiler::new_heap();

    println!("Starting memory profiling...\n");

    // Profile schema parsing
    profile_schema_parsing();

    // Profile query building
    profile_query_building();

    // Print memory stats if available
    print_memory_stats();

    println!("\nMemory profiling complete!");
    #[cfg(feature = "heap-profiling")]
    println!("Results written to dhat-heap.json");
    #[cfg(not(feature = "heap-profiling"))]
    println!("Run with --features heap-profiling for detailed heap analysis");
}

fn profile_schema_parsing() {
    println!("Profiling schema parsing...");

    let schema = r#"
        model User {
            id        Int      @id @auto
            email     String   @unique
            name      String?
            posts     Post[]
            profile   Profile?
            createdAt DateTime @default(now())
        }

        model Post {
            id        Int      @id @auto
            title     String
            content   String?
            published Boolean  @default(false)
            author    User     @relation(fields: [authorId], references: [id])
            authorId  Int
            createdAt DateTime @default(now())
        }

        model Profile {
            id     Int    @id @auto
            bio    String?
            user   User   @relation(fields: [userId], references: [id])
            userId Int    @unique
        }

        enum Role {
            USER
            ADMIN
            MODERATOR
        }
    "#;

    // Parse multiple times to get meaningful data
    for i in 0..100 {
        let result = parse_schema(schema);
        if i == 0 {
            match result {
                Ok(ast) => println!(
                    "  Parsed {} models, {} enums",
                    ast.models.len(),
                    ast.enums.len()
                ),
                Err(e) => println!("  Parse error: {}", e),
            }
        }
    }

    println!("  Completed 100 parse iterations");
}

fn profile_query_building() {
    println!("\nProfiling query building...");

    use prax_query::filter::{Filter, FilterValue};
    use prax_query::raw::Sql;

    // Build many filters
    let mut filters = Vec::new();
    for i in 0..1000 {
        let filter = Filter::and([
            Filter::Equals("id".into(), FilterValue::Int(i)),
            Filter::Equals("name".into(), FilterValue::String(format!("User {}", i))),
            Filter::or([
                Filter::Equals("status".into(), FilterValue::String("active".into())),
                Filter::Equals("status".into(), FilterValue::String("pending".to_string())),
            ]),
        ]);
        filters.push(filter);
    }
    println!("  Built {} complex filters", filters.len());

    // Build many SQL queries
    let mut queries = Vec::new();
    for i in 0..1000 {
        let sql = Sql::new("SELECT * FROM users WHERE ")
            .push("id = ")
            .bind(i)
            .push(" AND name = ")
            .bind(format!("User {}", i))
            .push(" AND (status = ")
            .bind("active")
            .push(" OR status = ")
            .bind("pending")
            .push(")");
        queries.push(sql.build());
    }
    println!("  Built {} SQL queries", queries.len());
}

fn print_memory_stats() {
    #[cfg(feature = "memory-stats")]
    {
        use memory_stats::memory_stats;

        if let Some(usage) = memory_stats() {
            println!("\nMemory Statistics:");
            println!("  Physical memory: {} KB", usage.physical_mem / 1024);
            println!("  Virtual memory:  {} KB", usage.virtual_mem / 1024);
        }
    }
}
