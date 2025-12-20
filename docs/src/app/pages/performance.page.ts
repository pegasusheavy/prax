import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-performance-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './performance.page.html',
})
export class PerformancePage {
  directSqlExample = `use prax_query::typed_filter::{self, DirectSql};

// Create typed filters
let filter = typed_filter::and(
    typed_filter::eq("active", true),
    typed_filter::gt("age", 18),
);

// Generate SQL with zero allocations
let mut sql = String::with_capacity(64);
let mut param_idx = 1;
filter.write_sql(&mut sql, &mut param_idx, DatabaseType::PostgreSQL);
// sql = "active = $1 AND age > $2"`;

  placeholderExample = `// Pre-computed placeholders for PostgreSQL
// Lookup table avoids format!() calls
pub static POSTGRES_PLACEHOLDERS: &[&str] = &[
    "$1", "$2", "$3", "$4", "$5", // ... up to $256
];

// Zero-cost placeholder access
let placeholder = POSTGRES_PLACEHOLDERS[idx - 1]; // ~0ns`;

  cacheExample = `use prax_query::cache::{global_template_cache, register_global_template};

// Pre-register common queries at startup
register_global_template(
    "users_find_by_id",
    "SELECT * FROM users WHERE id = $1"
);

// Fast lookup during request handling (~34ns)
if let Some(template) = global_template_cache().get("users_find_by_id") {
    // Use cached SQL string
}`;

  macroExample = `use prax_query::{filter, and_filter};

// Compile-time filter construction
let filter = filter!(user::active == true);

// Compound filters with zero runtime overhead
let complex = and_filter!(
    filter!(user::age > 18),
    filter!(user::role == Role::Admin),
    filter!(user::email.contains("@example.com"))
);`;
}

