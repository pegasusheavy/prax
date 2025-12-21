//! Detailed size analysis for Filter types.
//!
//! Run with:
//! ```bash
//! cargo run --release --example profile_size_analysis
//! ```

use prax_query::filter::{FieldName, Filter, FilterValue, ValueList};
use prax_query::memory::CompactFilter;
use prax_query::smallvec::SmallVec;
use std::mem::size_of;

fn main() {
    println!("=== Type Size Analysis ===\n");

    // Core types
    println!("1. Core Type Sizes:");
    println!("   Filter enum:        {} bytes", size_of::<Filter>());
    println!("   FilterValue enum:   {} bytes", size_of::<FilterValue>());
    println!("   ValueList:          {} bytes", size_of::<ValueList>());
    println!("   FieldName (Cow):    {} bytes", size_of::<FieldName>());
    println!(
        "   CompactFilter:      {} bytes",
        size_of::<CompactFilter>()
    );
    println!();

    // Breakdown
    println!("2. Type Breakdown:");
    println!("   Box<Filter>:        {} bytes", size_of::<Box<Filter>>());
    println!(
        "   Box<[Filter]>:      {} bytes",
        size_of::<Box<[Filter]>>()
    );
    println!("   Vec<Filter>:        {} bytes", size_of::<Vec<Filter>>());
    println!("   String:             {} bytes", size_of::<String>());
    println!(
        "   Cow<str>:           {} bytes",
        size_of::<std::borrow::Cow<'static, str>>()
    );
    println!(
        "   Arc<str>:           {} bytes",
        size_of::<std::sync::Arc<str>>()
    );
    println!();

    // SmallVec sizes
    println!("3. SmallVec Analysis:");
    println!(
        "   SmallVec<[FilterValue; 8]>:  {} bytes",
        size_of::<SmallVec<[FilterValue; 8]>>()
    );
    println!(
        "   SmallVec<[FilterValue; 16]>: {} bytes",
        size_of::<SmallVec<[FilterValue; 16]>>()
    );
    println!(
        "   SmallVec<[FilterValue; 32]>: {} bytes",
        size_of::<SmallVec<[FilterValue; 32]>>()
    );
    println!(
        "   Vec<FilterValue>:            {} bytes",
        size_of::<Vec<FilterValue>>()
    );
    println!();

    // Per-variant analysis (approximate)
    println!("4. Filter Variant Approximate Sizes:");
    println!(
        "   Equals/NotEquals:   ~{} bytes (FieldName + FilterValue)",
        size_of::<FieldName>() + size_of::<FilterValue>()
    );
    println!(
        "   In/NotIn:           ~{} bytes (FieldName + ValueList)",
        size_of::<FieldName>() + size_of::<ValueList>()
    );
    println!(
        "   IsNull/IsNotNull:   ~{} bytes (FieldName only)",
        size_of::<FieldName>()
    );
    println!(
        "   And/Or:             ~{} bytes (Box<[Filter]>)",
        size_of::<Box<[Filter]>>()
    );
    println!(
        "   Not:                ~{} bytes (Box<Filter>)",
        size_of::<Box<Filter>>()
    );
    println!();

    // Cache analysis
    println!("5. Cache Line Analysis (64 bytes):");
    println!("   Filters per cache line: {}", 64 / size_of::<Filter>());
    println!(
        "   FilterValues per cache line: {}",
        64 / size_of::<FilterValue>()
    );
    println!(
        "   CompactFilters per cache line: {}",
        64 / size_of::<CompactFilter>()
    );
    println!();

    // Potential savings
    let current_filter = size_of::<Filter>();
    let with_vec = size_of::<FieldName>() + size_of::<Vec<FilterValue>>() + 8; // discriminant + padding
    let with_smallvec_8 = size_of::<FieldName>() + size_of::<SmallVec<[FilterValue; 8]>>() + 8;

    println!("6. Potential Optimizations:");
    println!("   Current Filter size:        {} bytes", current_filter);
    println!(
        "   With Vec<FilterValue>:      ~{} bytes ({:.0}% smaller)",
        with_vec,
        100.0 - (with_vec as f64 / current_filter as f64 * 100.0)
    );
    println!(
        "   With SmallVec<[FV; 8]>:     ~{} bytes ({:.0}% smaller)",
        with_smallvec_8,
        100.0 - (with_smallvec_8 as f64 / current_filter as f64 * 100.0)
    );
    println!();

    // Recommendation
    println!("7. Recommendation:");
    if current_filter > 256 {
        println!(
            "   ⚠️  Filter is {} bytes - consider reducing inline capacity",
            current_filter
        );
        println!(
            "   💡 Reducing SmallVec capacity from 32 to 8 would save ~{}% memory",
            ((current_filter - with_smallvec_8) as f64 / current_filter as f64 * 100.0) as i32
        );
    } else {
        println!(
            "   ✅ Filter size is acceptable at {} bytes",
            current_filter
        );
    }
}
