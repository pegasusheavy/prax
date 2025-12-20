import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-queries-raw-sql-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './queries-raw-sql.page.html',
})
export class QueriesRawSqlPage {
  queryCode = `use prax::raw_sql;

// Query with type-safe parameters
let users: Vec<User> = client
    .raw_sql_query::<User>(
        raw_sql!("SELECT * FROM users WHERE active = {} AND age > {}", true, 18)
    )
    .await?;

// First result only
let user: Option<User> = client
    .raw_sql_first::<User>(
        raw_sql!("SELECT * FROM users WHERE id = {}", user_id)
    )
    .await?;

// Scalar value
let count: i64 = client
    .raw_sql_scalar(
        raw_sql!("SELECT COUNT(*) FROM users WHERE role = {}", "admin")
    )
    .await?;`;

  executeCode = `// Execute without returning results
let affected = client
    .raw_sql_execute(
        raw_sql!("UPDATE users SET last_seen = NOW() WHERE id = {}", user_id)
    )
    .await?;

println!("Updated {} rows", affected);

// Batch execute
client
    .raw_sql_batch(vec![
        raw_sql!("INSERT INTO logs (message) VALUES ({})", "Started"),
        raw_sql!("UPDATE counters SET value = value + 1"),
    ])
    .await?;`;

  builderCode = `use prax::Sql;

// Build SQL dynamically
let mut sql = Sql::new("SELECT * FROM users WHERE 1=1");

if let Some(name) = filter_name {
    sql = sql.push(" AND name LIKE ").bind(format!("%{}%", name));
}

if let Some(role) = filter_role {
    sql = sql.push(" AND role = ").bind(role);
}

sql = sql.push(" ORDER BY created_at DESC LIMIT ").bind(limit);

let users: Vec<User> = client.raw_sql_query(sql).await?;`;
}
