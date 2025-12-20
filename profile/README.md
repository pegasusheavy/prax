# Profiling Guide for Prax

This guide covers how to profile Prax ORM for performance analysis and optimization.

## Quick Start

```bash
# CPU profiling with flamegraph
cargo build --profile profiling --features cpu-profiling
cargo run --profile profiling --features cpu-profiling --example profile_queries

# Memory/heap profiling
cargo run --features heap-profiling --example profile_memory

# Tracing-based profiling (flame graph or chrome trace)
cargo run --features profiling --example profile_tracing
```

## Available Profiling Modes

### 1. CPU Profiling with pprof

Best for: Finding hot code paths, CPU-intensive operations.

```bash
# Build with profiling symbols
cargo build --profile profiling --features cpu-profiling

# Generate flamegraph
cargo run --profile profiling --features cpu-profiling -- \
    && flamegraph -o profile/flamegraph.svg target/profiling/prax
```

Output: `profile/flamegraph.svg` - Interactive SVG flamegraph

### 2. Heap Profiling with dhat

Best for: Finding memory allocations, memory leaks, allocation hotspots.

```bash
# Run with heap profiling
cargo run --features heap-profiling --example profile_memory

# Output will be written to dhat-heap.json
# View with: https://nnethercote.github.io/dh_view/dh_view.html
```

Output: `dhat-heap.json` - Upload to dhat viewer

### 3. Tracing Profiling

Best for: Understanding async execution flow, timing spans.

#### Flame Graph Output
```bash
cargo run --features profiling --example profile_tracing -- --flame
# Output: profile/tracing.folded (load in flamegraph tools)
```

#### Chrome Trace Output
```bash
cargo run --features profiling --example profile_tracing -- --chrome
# Output: profile/trace.json (load in chrome://tracing)
```

### 4. Benchmark Profiling

Profile specific benchmarks:

```bash
# Profile a benchmark with flamegraph
cargo bench --bench query_builder -- --profile-time=10

# With pprof integration
cargo bench --features cpu-profiling --bench query_builder
```

## Profile Configurations

The project includes several Cargo profiles:

| Profile | Use Case | Debug Symbols | Optimizations |
|---------|----------|---------------|---------------|
| `dev` | Development | Full | None |
| `dev-opt` | Fast testing | Full | Level 2 |
| `release` | Production | None | Full + LTO |
| `profiling` | Profiling | Full | Full + LTO |
| `bench` | Benchmarks | None | Full + LTO |

## Profiling Async Code

### Using tokio-console

For real-time async profiling:

```bash
# Install tokio-console
cargo install tokio-console

# Run with console subscriber
RUSTFLAGS="--cfg tokio_unstable" cargo run --features profiling
```

### Tracing Spans

The codebase uses `tracing` for instrumentation. Key spans:

- `query_many` - Multi-row queries
- `query_one` - Single-row queries
- `execute_insert` - Insert operations
- `execute_update` - Update operations
- `execute_delete` - Delete operations
- `raw_sql_*` - Raw SQL operations

## Memory Profiling Tips

### Tracking Allocations

```rust
// Enable in main.rs or test
#[cfg(feature = "heap-profiling")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "heap-profiling")]
    let _profiler = dhat::Profiler::new_heap();

    // Your code here
}
```

### Checking Memory Usage

```rust
use memory_stats::memory_stats;

if let Some(usage) = memory_stats() {
    println!("Physical memory: {} bytes", usage.physical_mem);
    println!("Virtual memory: {} bytes", usage.virtual_mem);
}
```

## Profiling Schema Parsing

```bash
# Profile schema parsing specifically
cargo bench -p prax-schema --bench schema_parsing --profile profiling
```

## Profiling Database Operations

```bash
# Profile SQLite operations (no external DB needed)
cargo bench -p prax-sqlite --bench sqlite_operations --profile profiling
```

## Common Bottlenecks to Look For

1. **String allocations** - Look for excessive `.to_string()` or `format!()` calls
2. **JSON serialization** - Serialization/deserialization overhead
3. **Connection acquisition** - Pool contention
4. **Query building** - Filter construction overhead
5. **Result mapping** - Row-to-struct conversion

## Interpreting Results

### Flamegraph Reading

- **Wide bars** = more CPU time
- **Tall stacks** = deep call chains
- Look for:
  - Unexpected wide bars in hot paths
  - Repeated patterns suggesting optimization opportunities
  - `clone()` or allocation functions taking significant time

### dhat Heap Analysis

Focus on:
- **Total bytes allocated** - Overall memory pressure
- **Max bytes live** - Peak memory usage
- **Allocation sites** - Where allocations originate
- **Block frequencies** - How often allocations happen

## Continuous Profiling

For CI integration, consider:

```yaml
# .github/workflows/profile.yml
name: Profile
on:
  pull_request:
    paths:
      - 'prax-*/src/**'
jobs:
  profile:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run benchmarks
        run: cargo bench --profile profiling
      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: profile-results
          path: target/criterion
```

## Tools Required

```bash
# Install profiling tools
cargo install flamegraph
cargo install cargo-instruments  # macOS only
cargo install tokio-console
cargo install samply  # Modern alternative to perf

# Linux: ensure perf is available
sudo apt-get install linux-tools-common linux-tools-generic
```

## Troubleshooting

### "Permission denied" for perf events
```bash
# Allow perf for current user (Linux)
echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid
```

### Missing debug symbols
```bash
# Ensure you're using the profiling profile
cargo build --profile profiling
```

### Slow compilation with profiling
```bash
# Use incremental compilation for faster iteration
CARGO_INCREMENTAL=1 cargo build --profile profiling
```

