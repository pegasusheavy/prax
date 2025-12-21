#![allow(dead_code, unused, clippy::type_complexity)]
//! Detailed allocation profiling for Filter::and() hot paths.
//!
//! Run with:
//! ```bash
//! cargo run --features heap-profiling --example profile_filter_allocations
//! ```

#[cfg(feature = "heap-profiling")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use prax_query::fields;
use prax_query::filter::{Filter, FilterValue, ValueList};
use std::hint::black_box;

fn main() {
    #[cfg(feature = "heap-profiling")]
    let _profiler = dhat::Profiler::new_heap();

    println!("=== Filter Allocation Analysis ===\n");

    // Analysis 1: Simple filter allocations
    analyze_simple_filters();

    // Analysis 2: AND filter scaling
    analyze_and_filter_scaling();

    // Analysis 3: IN filter allocations
    analyze_in_filter_allocations();

    // Analysis 4: String allocation patterns
    analyze_string_allocations();

    // Analysis 5: Compare static vs dynamic field names
    analyze_static_vs_dynamic();

    println!("\n=== Analysis Complete ===");
    #[cfg(feature = "heap-profiling")]
    println!("Results written to dhat-heap.json");
}

fn analyze_simple_filters() {
    println!("1. Simple Filter Allocations");
    println!("   Creating 10,000 simple equality filters...");

    // Static field name - should have minimal allocations
    for _ in 0..10_000 {
        let filter = black_box(Filter::Equals(fields::ID.into(), FilterValue::Int(42)));
        black_box(filter);
    }
    println!("   - Static field: Done");

    // Dynamic field name - will allocate String
    for i in 0..10_000 {
        let field = format!("field_{}", i % 100);
        let filter = black_box(Filter::Equals(field.into(), FilterValue::Int(42)));
        black_box(filter);
    }
    println!("   - Dynamic field: Done");

    println!();
}

fn analyze_and_filter_scaling() {
    println!("2. AND Filter Scaling Analysis");

    // Test different sizes
    for size in [2, 5, 10, 20, 50] {
        println!("   AND({} conditions):", size);

        for _ in 0..1000 {
            let filters: Vec<Filter> = (0..size)
                .map(|i| Filter::Equals(fields::ID.into(), FilterValue::Int(i as i64)))
                .collect();

            let and_filter = black_box(Filter::and(filters));
            black_box(and_filter);
        }
        println!("     1000 iterations complete");
    }

    // Compare with optimized and5()
    println!("   and5() optimized:");
    for _ in 0..1000 {
        let filter = black_box(Filter::and5(
            Filter::Equals(fields::ID.into(), FilterValue::Int(1)),
            Filter::Equals(fields::EMAIL.into(), FilterValue::Int(2)),
            Filter::Equals(fields::NAME.into(), FilterValue::Int(3)),
            Filter::Equals(fields::STATUS.into(), FilterValue::Int(4)),
            Filter::Equals(fields::ACTIVE.into(), FilterValue::Int(5)),
        ));
        black_box(filter);
    }
    println!("     1000 iterations complete");

    // Compare with and_n const generic
    println!("   and_n::<5>() const generic:");
    for _ in 0..1000 {
        let filter = black_box(Filter::and_n([
            Filter::Equals(fields::ID.into(), FilterValue::Int(1)),
            Filter::Equals(fields::EMAIL.into(), FilterValue::Int(2)),
            Filter::Equals(fields::NAME.into(), FilterValue::Int(3)),
            Filter::Equals(fields::STATUS.into(), FilterValue::Int(4)),
            Filter::Equals(fields::ACTIVE.into(), FilterValue::Int(5)),
        ]));
        black_box(filter);
    }
    println!("     1000 iterations complete");

    println!();
}

fn analyze_in_filter_allocations() {
    println!("3. IN Filter Allocation Analysis");

    // Small IN (fits in SmallVec inline)
    println!("   IN(10 values) - SmallVec inline:");
    for _ in 0..1000 {
        let values: ValueList = (0..10).map(FilterValue::Int).collect();
        let filter = black_box(Filter::In(fields::ID.into(), values));
        black_box(filter);
    }
    println!("     1000 iterations complete");

    // Medium IN (still inline with capacity 32)
    println!("   IN(30 values) - SmallVec inline:");
    for _ in 0..1000 {
        let values: ValueList = (0..30).map(FilterValue::Int).collect();
        let filter = black_box(Filter::In(fields::ID.into(), values));
        black_box(filter);
    }
    println!("     1000 iterations complete");

    // Large IN (spills to heap)
    println!("   IN(100 values) - SmallVec spills to heap:");
    for _ in 0..1000 {
        let values: ValueList = (0..100).map(FilterValue::Int).collect();
        let filter = black_box(Filter::In(fields::ID.into(), values));
        black_box(filter);
    }
    println!("     1000 iterations complete");

    // Using in_i64_slice (optimized)
    println!("   in_i64_slice(100 values) - optimized:");
    let slice: Vec<i64> = (0..100).collect();
    for _ in 0..1000 {
        let filter = black_box(Filter::in_i64_slice(fields::ID, &slice));
        black_box(filter);
    }
    println!("     1000 iterations complete");

    println!();
}

fn analyze_string_allocations() {
    println!("4. String Allocation Patterns");

    // FilterValue::String with static str
    println!("   FilterValue::String with &'static str:");
    for _ in 0..10_000 {
        let value = black_box(FilterValue::String("hello".into()));
        black_box(value);
    }
    println!("     10000 iterations complete");

    // FilterValue::String with owned String
    println!("   FilterValue::String with owned String:");
    for i in 0..10_000 {
        let s = format!("value_{}", i % 100);
        let value = black_box(FilterValue::String(s));
        black_box(value);
    }
    println!("     10000 iterations complete");

    println!();
}

fn analyze_static_vs_dynamic() {
    println!("5. Static vs Dynamic Field Names");

    // All static field names
    println!("   All static field names (1000 AND(5) filters):");
    for _ in 0..1000 {
        let filter = black_box(Filter::and([
            Filter::Equals(fields::ID.into(), FilterValue::Int(1)),
            Filter::Equals(
                fields::EMAIL.into(),
                FilterValue::String("test@example.com".into()),
            ),
            Filter::Equals(fields::NAME.into(), FilterValue::String("John".into())),
            Filter::Equals(fields::ACTIVE.into(), FilterValue::Bool(true)),
            Filter::Gt(fields::CREATED_AT.into(), FilterValue::Int(0)),
        ]));
        black_box(filter);
    }
    println!("     Complete");

    // All dynamic field names
    println!("   All dynamic field names (1000 AND(5) filters):");
    for _ in 0..1000 {
        let filter = black_box(Filter::and([
            Filter::Equals(format!("field_{}", 0).into(), FilterValue::Int(1)),
            Filter::Equals(
                format!("field_{}", 1).into(),
                FilterValue::String("test@example.com".into()),
            ),
            Filter::Equals(
                format!("field_{}", 2).into(),
                FilterValue::String("John".into()),
            ),
            Filter::Equals(format!("field_{}", 3).into(), FilterValue::Bool(true)),
            Filter::Gt(format!("field_{}", 4).into(), FilterValue::Int(0)),
        ]));
        black_box(filter);
    }
    println!("     Complete");

    println!();
}
