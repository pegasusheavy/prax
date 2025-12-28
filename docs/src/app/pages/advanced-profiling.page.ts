import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-advanced-profiling-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './advanced-profiling.page.html',
})
export class AdvancedProfilingPage {
  quickCheckCode = `use prax_query::profiling::{with_profiling, LeakReport};

let (result, report) = with_profiling(|| {
    // Your code here
    perform_database_operations()
});

if report.has_leaks() {
    eprintln!("⚠️  Potential leaks detected:\\n{}", report);
}`;

  snapshotCode = `use prax_query::profiling::{MemoryProfiler, MemorySnapshot};

let profiler = MemoryProfiler::new();

// Take initial snapshot
let before = profiler.snapshot();

// ... perform operations ...
let data = client.user().find_many().exec().await?;

// Take final snapshot and diff
let after = profiler.snapshot();
let diff = after.diff(&before);

println!("Memory delta: {} bytes", diff.bytes_delta);
if diff.has_leaks() {
    println!("{}", diff.report());
}`;

  trackingCode = `use prax_query::profiling::AllocationTracker;

// Start tracking
AllocationTracker::start();

// ... code to profile ...
let users = client.user().find_many().exec().await?;

// Get stats
let stats = AllocationTracker::stop();

println!("Total allocations: {}", stats.total_allocations);
println!("Total bytes: {}", stats.total_bytes);
println!("Peak bytes: {}", stats.peak_bytes);
println!("Current bytes: {}", stats.current_bytes);`;

  reportCode = `let profiler = MemoryProfiler::new();
let report = profiler.report();

// Overall stats
println!("Current: {} bytes", report.allocation_stats.current_bytes);
println!("Peak: {} bytes", report.allocation_stats.peak_bytes);

// String pool stats
println!("Interned strings: {}", report.pool_stats.string_pool.count);
println!("String pool bytes: {}", report.pool_stats.string_pool.bytes);

// Arena stats
println!("Arena allocations: {}", report.pool_stats.arena.count);`;

  ciCode = `# .github/workflows/memory-check.yml
name: Memory Leak Check
on: [pull_request]

jobs:
  valgrind:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Valgrind
        run: sudo apt-get install -y valgrind
      - name: Build with debug
        run: cargo build --features profiling
      - name: Run Valgrind
        run: |
          valgrind --leak-check=full \\
                   --show-leak-kinds=all \\
                   --error-exitcode=1 \\
                   ./target/debug/my_test`;
}

