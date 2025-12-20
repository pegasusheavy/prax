import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-queries-filtering-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './queries-filtering.page.html',
})
export class QueriesFilteringPage {
  basicFilters = `// Equality
user::id::equals(1)
user::name::not_equals("Admin")

// Comparison
user::age::gt(18)
user::age::gte(21)
user::age::lt(65)
user::age::lte(100)

// Null checks
user::deletedAt::is_null()
user::deletedAt::is_not_null()

// List membership
user::role::in_(vec![Role::Admin, Role::Moderator])
user::id::not_in(vec![1, 2, 3])`;

  stringFilters = `// Contains
user::name::contains("john")
user::email::contains("@example.com")

// Starts/ends with
user::name::starts_with("Dr.")
user::email::ends_with(".edu")

// Case-insensitive (mode)
user::name::contains("JOHN").mode(QueryMode::Insensitive)`;

  combineFilters = `// AND (implicit)
client.user().find_many()
    .where(user::active::equals(true))
    .where(user::age::gte(18))
    .exec().await?;

// AND (explicit)
.where(user::AND(vec![
    user::active::equals(true),
    user::age::gte(18),
]))

// OR
.where(user::OR(vec![
    user::role::equals(Role::Admin),
    user::role::equals(Role::Moderator),
]))

// NOT
.where(user::NOT(vec![
    user::email::contains("spam"),
]))`;

  relationFilters = `// Filter by related records
// Posts where author is active
post::author::is(user::active::equals(true))

// Users with at least one published post
user::posts::some(post::published::equals(true))

// Users where ALL posts are published
user::posts::every(post::published::equals(true))

// Users with no posts
user::posts::none()`;

  macroFilters = `use prax_query::{filter, and_filter, or_filter, not_filter};

// Simple filter with compile-time construction
let active_filter = filter!(user::active == true);

// Compound AND filter (~33ns for 5 conditions)
let complex = and_filter!(
    filter!(user::active == true),
    filter!(user::age >= 18),
    filter!(user::role == Role::Admin),
);

// OR filter
let role_filter = or_filter!(
    filter!(user::role == Role::Admin),
    filter!(user::role == Role::Moderator),
);

// NOT filter
let not_spam = not_filter!(
    filter!(user::email.contains("spam"))
);

// Nested combinations
let query_filter = and_filter!(
    filter!(user::active == true),
    or_filter!(
        filter!(user::role == Role::Admin),
        filter!(user::posts::some(post::published == true)),
    ),
);`;

  typedFilters = `use prax_query::typed_filter::{self, DirectSql};
use prax_query::sql::DatabaseType;

// Create typed filters (compile-time type composition)
let filter = typed_filter::and(
    typed_filter::eq("active", true),
    typed_filter::gt("age", 18),
);

// Generate SQL with zero allocations
let mut sql = String::with_capacity(64);
let mut param_idx = 1;
filter.write_sql(&mut sql, &mut param_idx, DatabaseType::PostgreSQL);
// Result: "active = $1 AND age > $2"

// Typed filters can be composed at compile time
let complex = typed_filter::and(
    typed_filter::eq("status", "active"),
    typed_filter::or(
        typed_filter::gt("priority", 5),
        typed_filter::is_not_null("due_date"),
    ),
);

// Performance: ~1.7ns for simple eq, ~4ns for AND(2), ~17ns for AND(5)`;
}
