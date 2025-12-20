# Prax ORM Benchmark Suite

Comprehensive benchmarking suite for the Prax ORM, measuring performance across schema parsing, query building, database operations, and more.

## Quick Start

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suites
cargo bench -p prax-schema --bench schema_parsing
cargo bench -p prax-schema --bench ast_operations
cargo bench -p prax-schema --bench validation_bench
cargo bench -p prax-query --bench operations_bench
cargo bench -p prax-query --bench aggregation_bench
cargo bench -p prax-query --bench pagination_bench
cargo bench -p prax-sqlite --bench sqlite_operations
```

## Benchmark Categories

### Schema Parsing (`prax-schema`)

Benchmarks for parsing Prax schema files into AST representations.

| Benchmark | Description |
|-----------|-------------|
| `schema_parsing` | Parser performance for various schema sizes and complexities |
| `ast_operations` | AST node creation, traversal, and manipulation |
| `validation_bench` | Validation rule parsing and field validation operations |

```bash
# Run schema benchmarks
cargo bench -p prax-schema
```

**Key metrics measured:**
- Schema parsing throughput (schemas/sec)
- AST node creation latency
- Field/model traversal performance
- Validation rule parsing speed

### Query Operations (`prax-query`)

Benchmarks for the type-safe query builder.

| Benchmark | Description |
|-----------|-------------|
| `operations_bench` | CRUD operations, find queries, SQL generation |
| `aggregation_bench` | Count, sum, avg, min, max, groupBy operations |
| `pagination_bench` | Skip/take, cursor-based pagination, ordering |

```bash
# Run query benchmarks
cargo bench -p prax-query
```

**Key metrics measured:**
- Query building latency
- SQL generation throughput
- Filter construction performance
- Complex query composition time

### Database Drivers

Driver-specific benchmarks for real database operations.

```bash
# SQLite benchmarks
cargo bench -p prax-sqlite

# PostgreSQL benchmarks (requires database)
cargo bench -p prax-postgres

# MySQL benchmarks (requires database)
cargo bench -p prax-mysql
```

## Benchmark Structure

Each benchmark file follows a consistent structure:

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("operation_category");

    group.bench_function("operation_name", |b| {
        b.iter(|| {
            // Operation to benchmark
        })
    });

    group.finish();
}

criterion_group!(benches, bench_operation);
criterion_main!(benches);
```

## Running Specific Tests

### Filter by Benchmark Name

```bash
# Run only benchmarks containing "find"
cargo bench -- find

# Run only aggregation-related benchmarks
cargo bench -- aggregate
```

### Generate HTML Reports

Criterion automatically generates HTML reports in `target/criterion/`:

```bash
# Run benchmarks and open report
cargo bench
open target/criterion/report/index.html
```

### Save Baseline for Comparison

```bash
# Save current performance as baseline
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main
```

## Benchmark Categories in Detail

### 1. Schema Parsing Benchmarks

**File:** `prax-schema/benches/schema_parsing.rs`

Tests parser performance with:
- Simple schemas (1-5 models)
- Medium schemas (10-20 models)
- Large schemas (50+ models)
- Complex schemas (relations, attributes, views)

### 2. AST Operations Benchmarks

**File:** `prax-schema/benches/ast_operations.rs`

Tests AST manipulation:
- Model creation (empty, with fields, with attributes)
- Schema building (various sizes)
- Enum operations
- Field type handling
- Attribute operations
- View creation
- Schema traversal
- Clone operations
- Relation building

### 3. Validation Benchmarks

**File:** `prax-schema/benches/validation_bench.rs`

Tests validation system:
- Validation rule creation
- Validation type checks
- Field validation operations
- Enhanced documentation parsing
- Field metadata handling
- Visibility checks
- Batch validation operations
- Field + validation integration

### 4. Query Operations Benchmarks

**File:** `prax-query/benches/operations_bench.rs`

Tests query building:
- Find operations (findMany, findUnique, findFirst)
- Create operations (single, bulk)
- Update operations (single, many, complex filters)
- Delete operations
- Upsert operations
- Count operations
- SQL builder utilities

### 5. Aggregation Benchmarks

**File:** `prax-query/benches/aggregation_bench.rs`

Tests aggregation queries:
- Aggregate field creation
- SQL generation for aggregates
- Aggregate operations (count, sum, avg, min, max)
- Group by operations
- Having conditions
- Real-world aggregate scenarios

### 6. Pagination Benchmarks

**File:** `prax-query/benches/pagination_bench.rs`

Tests pagination:
- Order by creation
- Sort operations
- Cursor pagination
- Offset pagination (skip/take)
- Complex pagination
- Real-world pagination scenarios

## Performance Targets

| Operation | Target Latency |
|-----------|----------------|
| Simple schema parse | < 100µs |
| Model creation | < 1µs |
| Simple query build | < 10µs |
| Complex query build | < 50µs |
| Filter creation | < 1µs |
| SQL generation | < 20µs |

## CI Integration

Add to your CI workflow:

```yaml
benchmark:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Run benchmarks
      run: cargo bench -- --noplot

    - name: Store benchmark result
      uses: benchmark-action/github-action-benchmark@v1
      with:
        tool: 'cargo'
        output-file-path: target/criterion/*/new/estimates.json
```

## Writing New Benchmarks

1. Add `criterion` to `[dev-dependencies]` in the crate's `Cargo.toml`:

```toml
[dev-dependencies]
criterion = { workspace = true }
```

2. Add `[[bench]]` section:

```toml
[[bench]]
name = "my_benchmark"
harness = false
```

3. Create benchmark file in `benches/my_benchmark.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_something(c: &mut Criterion) {
    c.bench_function("operation_name", |b| {
        b.iter(|| {
            black_box(operation_to_benchmark())
        })
    });
}

criterion_group!(benches, bench_something);
criterion_main!(benches);
```

## Troubleshooting

### Benchmarks Won't Compile

Ensure all benchmark dependencies are in `[dev-dependencies]`:

```bash
cargo check --benches
```

### High Variance Results

- Close other applications
- Use `--measurement-time 10` for longer measurements
- Run on a quiet system

### Missing HTML Reports

Ensure you're not using `--noplot`:

```bash
cargo bench  # With plots
cargo bench -- --noplot  # Without plots (CI)
```

## License

Apache 2.0 / MIT


