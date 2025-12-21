//! Cache efficiency and branch prediction analysis for Prax filters.
//!
//! This example measures cache-friendly access patterns and branch prediction
//! characteristics in filter iteration and matching.
//!
//! Run with:
//! ```bash
//! cargo run --release --example profile_cache_branch
//! ```

use prax_query::fields;
use prax_query::filter::{Filter, FilterValue, ValueList};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    println!("=== Cache Efficiency & Branch Prediction Analysis ===\n");

    // Analysis 1: Filter enum size and layout
    analyze_filter_layout();

    // Analysis 2: Sequential vs random access patterns
    analyze_access_patterns();

    // Analysis 3: Match statement branch patterns
    analyze_branch_patterns();

    // Analysis 4: Filter iteration patterns
    analyze_iteration_patterns();

    println!("\n=== Analysis Complete ===");
}

fn analyze_filter_layout() {
    println!("1. Filter Memory Layout Analysis");
    println!(
        "   Size of Filter enum: {} bytes",
        std::mem::size_of::<Filter>()
    );
    println!(
        "   Size of FilterValue: {} bytes",
        std::mem::size_of::<FilterValue>()
    );
    println!(
        "   Size of ValueList (inline): {} bytes",
        std::mem::size_of::<ValueList>()
    );
    println!(
        "   Size of FieldName (Cow<str>): {} bytes",
        std::mem::size_of::<std::borrow::Cow<'static, str>>()
    );
    println!(
        "   Size of Box<Filter>: {} bytes",
        std::mem::size_of::<Box<Filter>>()
    );
    println!(
        "   Size of Box<[Filter]>: {} bytes",
        std::mem::size_of::<Box<[Filter]>>()
    );
    println!(
        "   Size of Vec<Filter>: {} bytes",
        std::mem::size_of::<Vec<Filter>>()
    );

    // Alignment
    println!(
        "   Alignment of Filter: {} bytes",
        std::mem::align_of::<Filter>()
    );

    // Cache line analysis (typical 64-byte cache lines)
    let filters_per_cache_line = 64 / std::mem::size_of::<Filter>();
    println!(
        "   Filters per cache line (64B): ~{}",
        filters_per_cache_line
    );

    println!();
}

fn analyze_access_patterns() {
    println!("2. Memory Access Pattern Analysis");

    const COUNT: usize = 100_000;

    // Create filters in a contiguous array (cache-friendly)
    let filters: Vec<Filter> = (0..COUNT)
        .map(|i| Filter::Equals(fields::ID.into(), FilterValue::Int(i as i64)))
        .collect();

    // Sequential access (cache-friendly)
    let start = Instant::now();
    let mut sum = 0i64;
    for filter in &filters {
        if let Filter::Equals(_, FilterValue::Int(v)) = filter {
            sum += v;
        }
    }
    let sequential_time = start.elapsed();
    black_box(sum);
    println!(
        "   Sequential access ({} filters): {:?}",
        COUNT, sequential_time
    );

    // Random access (cache-unfriendly)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let indices: Vec<usize> = (0..COUNT)
        .map(|i| {
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            (hasher.finish() as usize) % COUNT
        })
        .collect();

    let start = Instant::now();
    let mut sum = 0i64;
    for &idx in &indices {
        if let Filter::Equals(_, FilterValue::Int(v)) = &filters[idx] {
            sum += v;
        }
    }
    let random_time = start.elapsed();
    black_box(sum);
    println!("   Random access ({} filters): {:?}", COUNT, random_time);

    let ratio = random_time.as_nanos() as f64 / sequential_time.as_nanos() as f64;
    println!("   Random/Sequential ratio: {:.2}x slower", ratio);

    println!();
}

fn analyze_branch_patterns() {
    println!("3. Branch Prediction Pattern Analysis");

    const COUNT: usize = 100_000;

    // Uniform distribution (predictable pattern)
    let uniform_filters: Vec<Filter> = (0..COUNT)
        .map(|_| Filter::Equals(fields::ID.into(), FilterValue::Int(1)))
        .collect();

    let start = Instant::now();
    let mut count = 0usize;
    for filter in &uniform_filters {
        count += match filter {
            Filter::Equals(..) => 1,
            Filter::NotEquals(..) => 2,
            Filter::Gt(..) => 3,
            Filter::Gte(..) => 4,
            Filter::Lt(..) => 5,
            Filter::Lte(..) => 6,
            Filter::Contains(..) => 7,
            Filter::StartsWith(..) => 8,
            Filter::EndsWith(..) => 9,
            Filter::In(..) => 10,
            Filter::NotIn(..) => 11,
            Filter::IsNull(..) => 12,
            Filter::IsNotNull(..) => 13,
            Filter::And(..) => 14,
            Filter::Or(..) => 15,
            Filter::Not(..) => 16,
            Filter::None => 17,
            _ => 0,
        };
    }
    let uniform_time = start.elapsed();
    black_box(count);
    println!("   Uniform (all Equals): {:?}", uniform_time);

    // Mixed distribution (harder to predict)
    let mixed_filters: Vec<Filter> = (0..COUNT)
        .map(|i| match i % 8 {
            0 => Filter::Equals(fields::ID.into(), FilterValue::Int(i as i64)),
            1 => Filter::NotEquals(fields::ID.into(), FilterValue::Int(i as i64)),
            2 => Filter::Gt(fields::ID.into(), FilterValue::Int(i as i64)),
            3 => Filter::Lt(fields::ID.into(), FilterValue::Int(i as i64)),
            4 => Filter::IsNull(fields::ID.into()),
            5 => Filter::IsNotNull(fields::ID.into()),
            6 => Filter::Contains(fields::NAME.into(), FilterValue::String("test".into())),
            _ => Filter::None,
        })
        .collect();

    let start = Instant::now();
    let mut count = 0usize;
    for filter in &mixed_filters {
        count += match filter {
            Filter::Equals(..) => 1,
            Filter::NotEquals(..) => 2,
            Filter::Gt(..) => 3,
            Filter::Gte(..) => 4,
            Filter::Lt(..) => 5,
            Filter::Lte(..) => 6,
            Filter::Contains(..) => 7,
            Filter::StartsWith(..) => 8,
            Filter::EndsWith(..) => 9,
            Filter::In(..) => 10,
            Filter::NotIn(..) => 11,
            Filter::IsNull(..) => 12,
            Filter::IsNotNull(..) => 13,
            Filter::And(..) => 14,
            Filter::Or(..) => 15,
            Filter::Not(..) => 16,
            Filter::None => 17,
            _ => 0,
        };
    }
    let mixed_time = start.elapsed();
    black_box(count);
    println!("   Mixed (8 variants rotating): {:?}", mixed_time);

    let ratio = mixed_time.as_nanos() as f64 / uniform_time.as_nanos() as f64;
    println!("   Mixed/Uniform ratio: {:.2}x slower", ratio);

    // Random distribution (worst case for branch prediction)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let random_filters: Vec<Filter> = (0..COUNT)
        .map(|i| {
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            match (hasher.finish() as usize) % 8 {
                0 => Filter::Equals(fields::ID.into(), FilterValue::Int(i as i64)),
                1 => Filter::NotEquals(fields::ID.into(), FilterValue::Int(i as i64)),
                2 => Filter::Gt(fields::ID.into(), FilterValue::Int(i as i64)),
                3 => Filter::Lt(fields::ID.into(), FilterValue::Int(i as i64)),
                4 => Filter::IsNull(fields::ID.into()),
                5 => Filter::IsNotNull(fields::ID.into()),
                6 => Filter::Contains(fields::NAME.into(), FilterValue::String("test".into())),
                _ => Filter::None,
            }
        })
        .collect();

    let start = Instant::now();
    let mut count = 0usize;
    for filter in &random_filters {
        count += match filter {
            Filter::Equals(..) => 1,
            Filter::NotEquals(..) => 2,
            Filter::Gt(..) => 3,
            Filter::Gte(..) => 4,
            Filter::Lt(..) => 5,
            Filter::Lte(..) => 6,
            Filter::Contains(..) => 7,
            Filter::StartsWith(..) => 8,
            Filter::EndsWith(..) => 9,
            Filter::In(..) => 10,
            Filter::NotIn(..) => 11,
            Filter::IsNull(..) => 12,
            Filter::IsNotNull(..) => 13,
            Filter::And(..) => 14,
            Filter::Or(..) => 15,
            Filter::Not(..) => 16,
            Filter::None => 17,
            _ => 0,
        };
    }
    let random_time = start.elapsed();
    black_box(count);
    println!("   Random (hashed selection): {:?}", random_time);

    let ratio = random_time.as_nanos() as f64 / uniform_time.as_nanos() as f64;
    println!("   Random/Uniform ratio: {:.2}x slower", ratio);

    println!();
}

fn analyze_iteration_patterns() {
    println!("4. Filter Iteration Pattern Analysis");

    // Nested AND filter iteration
    let nested_and = Filter::and([
        Filter::and([
            Filter::Equals(fields::ID.into(), FilterValue::Int(1)),
            Filter::Equals(fields::EMAIL.into(), FilterValue::String("test".into())),
        ]),
        Filter::and([
            Filter::Gt(fields::CREATED_AT.into(), FilterValue::Int(0)),
            Filter::IsNotNull(fields::UPDATED_AT.into()),
        ]),
        Filter::or([
            Filter::Equals(fields::STATUS.into(), FilterValue::String("active".into())),
            Filter::Equals(fields::STATUS.into(), FilterValue::String("pending".into())),
        ]),
    ]);

    // Count total filter nodes (simulates SQL generation traversal)
    fn count_filter_nodes(filter: &Filter) -> usize {
        match filter {
            Filter::And(filters) | Filter::Or(filters) => {
                1 + filters.iter().map(count_filter_nodes).sum::<usize>()
            }
            Filter::Not(inner) => 1 + count_filter_nodes(inner),
            Filter::None => 0,
            _ => 1,
        }
    }

    const ITERATIONS: usize = 100_000;

    let start = Instant::now();
    let mut total = 0usize;
    for _ in 0..ITERATIONS {
        total += count_filter_nodes(&nested_and);
    }
    let nested_time = start.elapsed();
    black_box(total);
    println!(
        "   Nested AND/OR traversal ({} iter): {:?}",
        ITERATIONS, nested_time
    );
    println!("   Nodes per filter: {}", count_filter_nodes(&nested_and));
    println!(
        "   Time per traversal: {:.1}ns",
        nested_time.as_nanos() as f64 / ITERATIONS as f64
    );

    // Flat AND filter iteration
    let flat_and = Filter::and([
        Filter::Equals(fields::ID.into(), FilterValue::Int(1)),
        Filter::Equals(fields::EMAIL.into(), FilterValue::String("test".into())),
        Filter::Gt(fields::CREATED_AT.into(), FilterValue::Int(0)),
        Filter::IsNotNull(fields::UPDATED_AT.into()),
        Filter::Equals(fields::STATUS.into(), FilterValue::String("active".into())),
        Filter::Equals(fields::ACTIVE.into(), FilterValue::Bool(true)),
    ]);

    let start = Instant::now();
    let mut total = 0usize;
    for _ in 0..ITERATIONS {
        total += count_filter_nodes(&flat_and);
    }
    let flat_time = start.elapsed();
    black_box(total);
    println!(
        "   Flat AND traversal ({} iter): {:?}",
        ITERATIONS, flat_time
    );
    println!("   Nodes per filter: {}", count_filter_nodes(&flat_and));
    println!(
        "   Time per traversal: {:.1}ns",
        flat_time.as_nanos() as f64 / ITERATIONS as f64
    );

    println!();
}
