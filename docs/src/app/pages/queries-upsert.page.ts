import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-queries-upsert-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './queries-upsert.page.html',
})
export class QueriesUpsertPage {
  basicUpsert = `use prax::upsert::{Upsert, ConflictTarget, ConflictAction};

// PostgreSQL: ON CONFLICT DO UPDATE
let upsert = Upsert::new("users")
    .columns(["email", "name", "updated_at"])
    .values(["alice@example.com", "Alice", "2024-01-15"])
    .on_conflict(
        ConflictTarget::columns(["email"]),
        ConflictAction::do_update(["name", "updated_at"])
    )
    .build_postgres();

// INSERT INTO users (email, name, updated_at)
// VALUES ($1, $2, $3)
// ON CONFLICT (email)
// DO UPDATE SET name = EXCLUDED.name, updated_at = EXCLUDED.updated_at`;

  doNothing = `use prax::upsert::{Upsert, ConflictTarget, ConflictAction};

// INSERT or ignore on conflict
let insert_ignore = Upsert::new("users")
    .columns(["email", "name"])
    .values(["bob@example.com", "Bob"])
    .on_conflict(
        ConflictTarget::columns(["email"]),
        ConflictAction::do_nothing()
    )
    .build_postgres();

// INSERT INTO users (email, name) VALUES ($1, $2) ON CONFLICT (email) DO NOTHING

// MySQL equivalent: INSERT IGNORE
let mysql = Upsert::new("users")
    .columns(["email", "name"])
    .values(["bob@example.com", "Bob"])
    .ignore_on_conflict()
    .build_mysql();

// INSERT IGNORE INTO users (email, name) VALUES (?, ?)`;

  conflictTargets = `use prax::upsert::{Upsert, ConflictTarget, ConflictAction};

// Conflict on specific columns
let by_columns = ConflictTarget::columns(["tenant_id", "email"]);

// Conflict on constraint name
let by_constraint = ConflictTarget::constraint("users_email_unique");

// Conflict on partial index (PostgreSQL)
let by_partial = ConflictTarget::columns(["email"])
    .where_clause("active = true");

// Conflict on expression
let by_expression = ConflictTarget::expression("LOWER(email)");`;

  conditionalUpdate = `use prax::upsert::{Upsert, ConflictTarget, ConflictAction, UpdateSpec};

// Conditional update with WHERE
let upsert = Upsert::new("products")
    .columns(["sku", "price", "stock", "updated_at"])
    .values(["SKU-123", "29.99", "100", "2024-01-15"])
    .on_conflict(
        ConflictTarget::columns(["sku"]),
        ConflictAction::do_update_where(
            ["price", "stock", "updated_at"],
            "products.updated_at < EXCLUDED.updated_at"  // Only update if newer
        )
    )
    .build_postgres();

// Custom update expressions
let custom = Upsert::new("counters")
    .columns(["key", "value"])
    .values(["page_views", "1"])
    .on_conflict(
        ConflictTarget::columns(["key"]),
        ConflictAction::do_update_set([
            ("value", "counters.value + EXCLUDED.value"),  // Increment
            ("updated_at", "NOW()"),
        ])
    )
    .build_postgres();`;

  mysqlUpsert = `use prax::upsert::{Upsert, ConflictAction};

// MySQL: ON DUPLICATE KEY UPDATE
let upsert = Upsert::new("users")
    .columns(["email", "name", "login_count"])
    .values(["alice@example.com", "Alice", "1"])
    .on_duplicate_key_update(["name", "login_count"])
    .build_mysql();

// INSERT INTO users (email, name, login_count)
// VALUES (?, ?, ?)
// ON DUPLICATE KEY UPDATE
//   name = VALUES(name),
//   login_count = login_count + VALUES(login_count)

// With custom expressions
let custom = Upsert::new("counters")
    .columns(["key", "value"])
    .values(["visits", "1"])
    .on_duplicate_key_set([
        ("value", "value + 1"),
        ("updated_at", "NOW()"),
    ])
    .build_mysql();`;

  mssqlMerge = `use prax::upsert::{Upsert, ConflictTarget, ConflictAction};

// MSSQL: MERGE statement
let merge = Upsert::new("users")
    .columns(["email", "name", "updated_at"])
    .values(["alice@example.com", "Alice", "2024-01-15"])
    .on_conflict(
        ConflictTarget::columns(["email"]),
        ConflictAction::do_update(["name", "updated_at"])
    )
    .build_mssql();

// MERGE INTO users AS target
// USING (VALUES (@P1, @P2, @P3)) AS source (email, name, updated_at)
// ON target.email = source.email
// WHEN MATCHED THEN
//   UPDATE SET name = source.name, updated_at = source.updated_at
// WHEN NOT MATCHED THEN
//   INSERT (email, name, updated_at)
//   VALUES (source.email, source.name, source.updated_at);

// MERGE with DELETE
let merge_delete = Upsert::new("inventory")
    .columns(["sku", "quantity"])
    .values(["SKU-123", "0"])
    .on_conflict(
        ConflictTarget::columns(["sku"]),
        ConflictAction::merge()
            .when_matched_update(["quantity"])
            .when_matched_delete("quantity = 0")  // Delete if zero
    )
    .build_mssql();`;

  mongoUpsert = `use prax::upsert::mongodb::{MongoUpsertBuilder, BulkUpsert};

// MongoDB native upsert
let result = client
    .users()
    .update_one(
        doc! { "email": "alice@example.com" },
        doc! {
            "$set": { "name": "Alice", "updatedAt": now },
            "$setOnInsert": { "createdAt": now },  // Only on insert
            "$inc": { "loginCount": 1 },
        },
    )
    .upsert(true)
    .await?;

if result.upserted_id.is_some() {
    println!("Created new user");
} else {
    println!("Updated existing user");
}

// Bulk upsert with bulkWrite
let bulk = BulkUpsert::new()
    .upsert(
        doc! { "email": "alice@example.com" },
        doc! { "$set": { "name": "Alice" } }
    )
    .upsert(
        doc! { "email": "bob@example.com" },
        doc! { "$set": { "name": "Bob" } }
    )
    .upsert(
        doc! { "email": "carol@example.com" },
        doc! { "$set": { "name": "Carol" } }
    );

let result = client.users().bulk_write(bulk.operations()).await?;
println!("Inserted: {}, Modified: {}", result.inserted_count, result.modified_count);

// Replace semantics (full document replacement)
client
    .users()
    .replace_one(
        doc! { "email": "alice@example.com" },
        User { email: "alice@example.com".into(), name: "Alice".into(), ... },
    )
    .upsert(true)
    .await?;`;

  bulkUpsert = `use prax::upsert::{Upsert, BulkUpsert};

// Bulk upsert multiple rows
let bulk = BulkUpsert::new("products")
    .columns(["sku", "name", "price"])
    .values([
        ["SKU-001", "Widget A", "9.99"],
        ["SKU-002", "Widget B", "19.99"],
        ["SKU-003", "Widget C", "29.99"],
    ])
    .on_conflict(
        ConflictTarget::columns(["sku"]),
        ConflictAction::do_update(["name", "price"])
    )
    .build_postgres();

// PostgreSQL:
// INSERT INTO products (sku, name, price) VALUES
//   ($1, $2, $3),
//   ($4, $5, $6),
//   ($7, $8, $9)
// ON CONFLICT (sku) DO UPDATE SET
//   name = EXCLUDED.name,
//   price = EXCLUDED.price

// With RETURNING
let inserted = BulkUpsert::new("products")
    .columns(["sku", "name", "price"])
    .values(products)
    .on_conflict(
        ConflictTarget::columns(["sku"]),
        ConflictAction::do_update(["name", "price"])
    )
    .returning(["id", "sku", "created_at"])
    .exec(&client)
    .await?;`;

  praxUpsert = `// Using Prax's fluent API
let user = client
    .user()
    .upsert()
    .where(user::email::equals("alice@example.com"))
    .create(user::Create {
        email: "alice@example.com".into(),
        name: "Alice".into(),
        ..Default::default()
    })
    .update(user::Update {
        name: Some("Alice Updated".into()),
        ..Default::default()
    })
    .exec()
    .await?;

// create_many with skipDuplicates
let users = client
    .user()
    .create_many([
        user::Create { email: "a@example.com".into(), ... },
        user::Create { email: "b@example.com".into(), ... },
        user::Create { email: "c@example.com".into(), ... },
    ])
    .skip_duplicates()  // Equivalent to ON CONFLICT DO NOTHING
    .exec()
    .await?;`;
}



