# Fuzzing Guide for Prax

This directory contains fuzz tests for the Prax ORM using [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) and libFuzzer.

## Prerequisites

```bash
# Install cargo-fuzz (requires nightly)
cargo install cargo-fuzz

# Install AFL alternative (optional)
cargo install afl
```

## Available Fuzz Targets

| Target | Description |
|--------|-------------|
| `fuzz_schema_parser` | Raw byte fuzzing of schema parser |
| `fuzz_schema_structured` | Structured fuzzing with valid-ish schemas |
| `fuzz_query_builder` | SQL query builder operations |
| `fuzz_filter_construction` | Filter tree construction |
| `fuzz_sql_builder` | SQL builder utilities |
| `fuzz_config_parser` | TOML config parser |

## Quick Start

```bash
# Enter the fuzz directory
cd fuzz

# Run a fuzz target
cargo +nightly fuzz run fuzz_schema_parser

# Run with a time limit (e.g., 60 seconds)
cargo +nightly fuzz run fuzz_schema_parser -- -max_total_time=60

# Run with a specific corpus
cargo +nightly fuzz run fuzz_schema_parser corpus/schema_parser/

# Run all fuzz targets sequentially
for target in fuzz_schema_parser fuzz_schema_structured fuzz_query_builder fuzz_filter_construction fuzz_sql_builder fuzz_config_parser; do
    cargo +nightly fuzz run $target -- -max_total_time=30
done
```

## Fuzz Target Details

### Schema Parser (`fuzz_schema_parser`)

Tests the Prax schema parser with arbitrary byte sequences. Good for finding:
- Parser crashes on malformed input
- Unicode handling issues
- Buffer overflow vulnerabilities

```bash
cargo +nightly fuzz run fuzz_schema_parser
```

### Structured Schema Fuzzer (`fuzz_schema_structured`)

Generates semi-valid schema structures using the `arbitrary` crate. More effective at exploring parser logic:
- Field type combinations
- Attribute parsing
- Relation definitions
- Enum handling

```bash
cargo +nightly fuzz run fuzz_schema_structured
```

### Query Builder (`fuzz_query_builder`)

Tests the SQL query builder with random operations:
- Push/bind sequences
- Database type variations
- Parameter interpolation

```bash
cargo +nightly fuzz run fuzz_query_builder
```

### Filter Construction (`fuzz_filter_construction`)

Tests filter tree construction:
- Deep nesting
- Complex AND/OR combinations
- Various filter types
- Value type handling

```bash
cargo +nightly fuzz run fuzz_filter_construction
```

### SQL Builder (`fuzz_sql_builder`)

Tests low-level SQL building utilities:
- Identifier escaping
- Placeholder generation
- SQL string construction

```bash
cargo +nightly fuzz run fuzz_sql_builder
```

### Config Parser (`fuzz_config_parser`)

Tests TOML configuration parsing:
- Invalid TOML syntax
- Unexpected values
- Missing fields

```bash
cargo +nightly fuzz run fuzz_config_parser
```

## Working with Crashes

### Finding Crash Location

```bash
# The fuzzer will output the crash file location
# Reproduce with:
cargo +nightly fuzz run fuzz_schema_parser artifacts/fuzz_schema_parser/crash-*

# Get a backtrace
RUST_BACKTRACE=1 cargo +nightly fuzz run fuzz_schema_parser artifacts/fuzz_schema_parser/crash-*
```

### Minimizing Crashes

```bash
# Minimize a crash input
cargo +nightly fuzz tmin fuzz_schema_parser artifacts/fuzz_schema_parser/crash-*
```

### Corpus Management

```bash
# Create initial corpus with valid inputs
mkdir -p corpus/fuzz_schema_parser
echo 'model User { id Int @id }' > corpus/fuzz_schema_parser/valid_simple
echo 'enum Role { ADMIN USER }' > corpus/fuzz_schema_parser/valid_enum

# Merge corpus (deduplicate)
cargo +nightly fuzz cmin fuzz_schema_parser
```

## Advanced Options

### Parallel Fuzzing

```bash
# Run with multiple jobs
cargo +nightly fuzz run fuzz_schema_parser -- -jobs=4 -workers=4
```

### Memory Limits

```bash
# Limit memory usage (in MB)
cargo +nightly fuzz run fuzz_schema_parser -- -rss_limit_mb=2048
```

### Timeout Handling

```bash
# Set per-input timeout (in seconds)
cargo +nightly fuzz run fuzz_schema_parser -- -timeout=10
```

### Dictionary-Based Fuzzing

Create a dictionary file for better fuzzing:

```bash
# Create dictionary for schema parser
cat > dict/schema.dict << 'EOF'
"model"
"enum"
"view"
"@id"
"@auto"
"@unique"
"@default"
"@relation"
"@map"
"@index"
"Int"
"String"
"Boolean"
"DateTime"
"Float"
"Json"
"fields"
"references"
"onDelete"
"onUpdate"
"Cascade"
"SetNull"
"Restrict"
EOF

# Run with dictionary
cargo +nightly fuzz run fuzz_schema_parser -- -dict=dict/schema.dict
```

## Coverage-Guided Development

### Generate Coverage Report

```bash
# Build with coverage instrumentation
RUSTFLAGS="-C instrument-coverage" cargo +nightly fuzz coverage fuzz_schema_parser

# Generate HTML report (requires llvm-cov)
llvm-cov show target/*/release/fuzz_schema_parser \
    -instr-profile=coverage/fuzz_schema_parser/coverage.profdata \
    -format=html > coverage.html
```

## CI Integration

Add to `.github/workflows/fuzz.yml`:

```yaml
name: Fuzz Testing
on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly on Sunday
  workflow_dispatch:

jobs:
  fuzz:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - fuzz_schema_parser
          - fuzz_schema_structured
          - fuzz_query_builder
          - fuzz_filter_construction
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz
      - name: Run fuzzer
        run: |
          cd fuzz
          cargo +nightly fuzz run ${{ matrix.target }} -- -max_total_time=300
      - name: Upload crashes
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: fuzz-crashes-${{ matrix.target }}
          path: fuzz/artifacts/${{ matrix.target }}/
```

## OSS-Fuzz Integration

For continuous fuzzing via Google's OSS-Fuzz, create `fuzz/oss-fuzz/build.sh`:

```bash
#!/bin/bash
cd $SRC/prax/fuzz
cargo +nightly fuzz build
cp target/*/release/fuzz_* $OUT/
```

## Troubleshooting

### "error: could not compile" with sanitizer

```bash
# Install required LLVM components
rustup component add llvm-tools-preview --toolchain nightly
```

### "LLVM ERROR: unsupported ASAN platform"

```bash
# Use a supported platform or disable sanitizers
cargo +nightly fuzz run fuzz_schema_parser -- -use_value_profile=0
```

### Slow fuzzing

```bash
# Disable debug info for faster execution
RUSTFLAGS="-C debuginfo=0" cargo +nightly fuzz run fuzz_schema_parser
```

## Best Practices

1. **Start with structured fuzzing** - Use `fuzz_schema_structured` first as it explores more code paths
2. **Build a good corpus** - Add valid inputs to guide the fuzzer
3. **Use dictionaries** - Keyword dictionaries help for text-based formats
4. **Run regularly** - Fuzzing is most effective over long periods
5. **Fix crashes immediately** - Don't let crashes accumulate
6. **Add regression tests** - Convert crash inputs into unit tests

