import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-queries-cte-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './queries-cte.page.html',
})
export class QueriesCtePage {
  basicCte = `use prax::cte::{Cte, WithClause};

// Basic CTE (Common Table Expression)
let cte = Cte::new("active_users")
    .columns(["id", "email", "name"])
    .as_query("SELECT id, email, name FROM users WHERE active = true");

let query = WithClause::new()
    .cte(cte)
    .main_query("SELECT * FROM active_users WHERE email LIKE '%@example.com'")
    .build();

// WITH active_users (id, email, name) AS (
//   SELECT id, email, name FROM users WHERE active = true
// )
// SELECT * FROM active_users WHERE email LIKE '%@example.com'`;

  multipleCtes = `use prax::cte::{Cte, WithClause};

// Multiple CTEs
let users_cte = Cte::new("active_users")
    .as_query("SELECT * FROM users WHERE active = true");

let orders_cte = Cte::new("recent_orders")
    .as_query("SELECT * FROM orders WHERE created_at > NOW() - INTERVAL '30 days'");

let stats_cte = Cte::new("user_stats")
    .as_query(r#"
        SELECT
            u.id as user_id,
            COUNT(o.id) as order_count,
            SUM(o.total) as total_spent
        FROM active_users u
        LEFT JOIN recent_orders o ON o.user_id = u.id
        GROUP BY u.id
    "#);

let query = WithClause::new()
    .cte(users_cte)
    .cte(orders_cte)
    .cte(stats_cte)
    .main_query("SELECT * FROM user_stats WHERE total_spent > 1000")
    .build();`;

  recursiveCte = `use prax::cte::{Cte, WithClause, SearchClause, SearchMethod, CycleClause};

// Recursive CTE for hierarchical data
let org_tree = Cte::recursive("org_hierarchy")
    .columns(["id", "name", "manager_id", "depth", "path"])
    .initial_query(r#"
        SELECT id, name, manager_id, 0 as depth, ARRAY[name] as path
        FROM employees
        WHERE manager_id IS NULL
    "#)
    .recursive_query(r#"
        SELECT e.id, e.name, e.manager_id, oh.depth + 1, oh.path || e.name
        FROM employees e
        JOIN org_hierarchy oh ON e.manager_id = oh.id
    "#);

let query = WithClause::recursive()
    .cte(org_tree)
    .main_query("SELECT * FROM org_hierarchy ORDER BY path")
    .build();

// WITH RECURSIVE org_hierarchy (id, name, manager_id, depth, path) AS (
//   SELECT id, name, manager_id, 0, ARRAY[name] FROM employees WHERE manager_id IS NULL
//   UNION ALL
//   SELECT e.id, e.name, e.manager_id, oh.depth + 1, oh.path || e.name
//   FROM employees e JOIN org_hierarchy oh ON e.manager_id = oh.id
// )
// SELECT * FROM org_hierarchy ORDER BY path

// With SEARCH clause (PostgreSQL 14+)
let breadth_first = Cte::recursive("tree")
    .search(SearchClause::new(SearchMethod::BreadthFirst, ["id"]).set_column("ordercol"))
    .cycle(CycleClause::new(["id"]).mark_column("is_cycle").path_column("cycle_path"));`;

  materializedCte = `use prax::cte::{Cte, WithClause, Materialized};

// Materialized CTE (PostgreSQL 12+)
let expensive_cte = Cte::new("expensive_calculation")
    .materialized(Materialized::Yes)
    .as_query(r#"
        SELECT product_id, complex_aggregation(data) as result
        FROM large_table
        GROUP BY product_id
    "#);

// WITH expensive_calculation AS MATERIALIZED (...)

// NOT MATERIALIZED (force inline)
let simple_cte = Cte::new("filtered_data")
    .materialized(Materialized::No)
    .as_query("SELECT * FROM data WHERE active = true");

// WITH filtered_data AS NOT MATERIALIZED (...)

// Let PostgreSQL decide (default)
let auto_cte = Cte::new("auto_decide")
    .as_query("SELECT * FROM data");`;

  ctePatterns = `use prax::cte::patterns;

// Tree traversal pattern
let tree = patterns::tree_traversal(
    "categories",           // table
    "id",                   // id column
    "parent_id",            // parent column
    Some("Electronics"),    // root name (optional)
);

// Graph shortest path
let path = patterns::graph_path(
    "connections",
    "from_node",
    "to_node",
    "node_a",
    "node_z",
    Some(10),  // max depth
);

// Paginated with total count
let paginated = patterns::paginated(
    "SELECT * FROM products WHERE category = $1",
    20,   // page size
    3,    // page number
);
// Returns both rows and total_count in a single query

// Running total
let running = patterns::running_total(
    "transactions",
    "amount",
    "created_at",
    Some("account_id"),  // partition by
);`;

  mongoLookup = `use prax::cte::mongodb::{Lookup, GraphLookup, UnionWith};

// MongoDB $lookup (like CTE + JOIN)
let pipeline = vec![
    Lookup::new()
        .from("orders")
        .local_field("_id")
        .foreign_field("user_id")
        .as_field("user_orders")
        .to_stage(),
];

// $lookup with pipeline (subquery)
let pipeline = vec![
    Lookup::new()
        .from("orders")
        .let_vars([("userId", "$_id")])
        .pipeline([
            doc! { "$match": { "$expr": { "$eq": ["$user_id", "$$userId"] } } },
            doc! { "$sort": { "created_at": -1 } },
            doc! { "$limit": 5 },
        ])
        .as_field("recent_orders")
        .to_stage(),
];

// $graphLookup for recursive relationships
let org_chart = GraphLookup::new()
    .from("employees")
    .start_with("$manager_id")
    .connect_from_field("manager_id")
    .connect_to_field("_id")
    .as_field("reporting_chain")
    .max_depth(10)
    .depth_field("level")
    .to_stage();

// $unionWith (UNION equivalent)
let combined = UnionWith::new("archive_orders")
    .pipeline([
        doc! { "$match": { "status": "completed" } },
        doc! { "$project": { "id": 1, "total": 1, "created_at": 1 } },
    ])
    .to_stage();`;

  windowFunctions = `use prax::window::{WindowFunction, WindowSpec, row_number, rank, lag, sum};

// ROW_NUMBER
let numbered = row_number()
    .over(WindowSpec::new()
        .partition_by(["department"])
        .order_by([("salary", "DESC")])
    )
    .alias("row_num");

// RANK with ties
let ranked = rank()
    .over(WindowSpec::new()
        .partition_by(["category"])
        .order_by([("score", "DESC")])
    )
    .alias("rank");

// LAG/LEAD for comparing with previous/next rows
let prev_value = lag("price", 1, Some("0"))
    .over(WindowSpec::new()
        .partition_by(["product_id"])
        .order_by([("date", "ASC")])
    )
    .alias("prev_price");

// Running sum
let running_total = sum("amount")
    .over(WindowSpec::new()
        .partition_by(["account_id"])
        .order_by([("date", "ASC")])
        .frame("ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW")
    )
    .alias("running_total");

// Multiple window functions in one query
let query = client
    .raw_query(
        r#"
        SELECT
            *,
            ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) as dept_rank,
            AVG(salary) OVER (PARTITION BY dept) as dept_avg,
            salary - LAG(salary) OVER (ORDER BY hire_date) as salary_change
        FROM employees
        "#,
        []
    )
    .await?;`;
}



