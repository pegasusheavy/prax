import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-queries-json-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './queries-json.page.html',
})
export class QueriesJsonPage {
  jsonPath = `use prax::json::{JsonPath, PathSegment};

// Build JSON paths programmatically
let path = JsonPath::new("settings")
    .field("notifications")
    .field("email")
    .field("enabled");

// Generates database-specific syntax:
// PostgreSQL: settings->'notifications'->'email'->'enabled'
// MySQL: JSON_EXTRACT(settings, '$.notifications.email.enabled')
// SQLite: json_extract(settings, '$.notifications.email.enabled')
// MSSQL: JSON_VALUE(settings, '$.notifications.email.enabled')

// Array access
let first_item = JsonPath::new("items").index(0).field("name");
// PostgreSQL: items->0->'name'
// MySQL: JSON_EXTRACT(items, '$[0].name')

// Get as text (unquoted)
let email_text = JsonPath::new("profile").field("email").text();
// PostgreSQL: profile->>'email'
// MySQL: JSON_UNQUOTE(JSON_EXTRACT(profile, '$.email'))`;

  jsonFilter = `use prax::json::{JsonFilter, JsonOp};

// Filter by JSON field
let users = client
    .user()
    .find_many()
    .where(JsonFilter::path("settings.theme").equals("dark"))
    .exec()
    .await?;

// PostgreSQL: WHERE settings->>'theme' = 'dark'

// Contains (JSONB containment)
let admins = client
    .user()
    .find_many()
    .where(JsonFilter::path("roles").contains(json!(["admin"])))
    .exec()
    .await?;

// PostgreSQL: WHERE roles @> '["admin"]'

// Has key
let verified = client
    .user()
    .find_many()
    .where(JsonFilter::path("metadata").has_key("verified_at"))
    .exec()
    .await?;

// PostgreSQL: WHERE metadata ? 'verified_at'

// Has any key
let with_social = client
    .user()
    .find_many()
    .where(JsonFilter::path("social").has_any_key(["twitter", "github"]))
    .exec()
    .await?;

// PostgreSQL: WHERE social ?| array['twitter', 'github']

// JSON path match (PostgreSQL 12+)
let expensive = client
    .product()
    .find_many()
    .where(JsonFilter::path_match("$.price ? (@ > 100)"))
    .exec()
    .await?;`;

  jsonMutation = `use prax::json::JsonOp;

// Set a JSON field
client
    .user()
    .update(user::id::equals(1))
    .json_set("settings", "theme", json!("dark"))
    .exec()
    .await?;

// PostgreSQL: UPDATE users SET settings = jsonb_set(settings, '{theme}', '"dark"')

// Insert into JSON (only if key doesn't exist)
client
    .user()
    .update(user::id::equals(1))
    .json_insert("settings", "new_feature", json!(true))
    .exec()
    .await?;

// Remove a JSON key
client
    .user()
    .update(user::id::equals(1))
    .json_remove("settings", "deprecated_field")
    .exec()
    .await?;

// PostgreSQL: UPDATE users SET settings = settings - 'deprecated_field'

// Array append
client
    .user()
    .update(user::id::equals(1))
    .json_array_append("settings", "tags", json!("vip"))
    .exec()
    .await?;

// PostgreSQL: UPDATE users SET settings = jsonb_set(settings, '{tags}', settings->'tags' || '"vip"')

// Increment numeric value in JSON
client
    .product()
    .update(product::id::equals(1))
    .json_increment("stats", "views", 1)
    .exec()
    .await?;`;

  jsonAgg = `use prax::json::JsonAgg;

// Aggregate rows into JSON array
let result = client
    .raw_query(
        r#"
        SELECT
            u.id,
            u.name,
            json_agg(json_build_object('id', p.id, 'title', p.title)) as posts
        FROM users u
        LEFT JOIN posts p ON p.author_id = u.id
        GROUP BY u.id, u.name
        "#,
        []
    )
    .await?;

// Build JSON object from columns
let stats = JsonAgg::build_object([
    ("total_users", "COUNT(*)"),
    ("active_users", "COUNT(*) FILTER (WHERE active)"),
    ("avg_age", "AVG(age)"),
])
.build_postgres();

// PostgreSQL: json_build_object('total_users', COUNT(*), 'active_users', COUNT(*) FILTER (WHERE active), ...)

// Aggregate into array with ordering
let ordered = JsonAgg::array_agg("name")
    .order_by("created_at DESC")
    .filter("active = true")
    .build_postgres();

// PostgreSQL: json_agg(name ORDER BY created_at DESC) FILTER (WHERE active)`;

  jsonIndex = `use prax::json::{JsonIndex, JsonIndexBuilder};

// GIN index for JSONB containment queries (PostgreSQL)
let gin_index = JsonIndexBuilder::new("user_settings_idx")
    .table("users")
    .column("settings")
    .using("GIN")
    .ops_class("jsonb_path_ops")  // Optimized for @> queries
    .build();

// CREATE INDEX user_settings_idx ON users USING GIN (settings jsonb_path_ops)

// Expression index for specific JSON path
let email_index = JsonIndexBuilder::new("user_email_idx")
    .table("users")
    .expression("(profile->>'email')")
    .build();

// CREATE INDEX user_email_idx ON users ((profile->>'email'))

// MySQL generated column + index
let mysql_index = JsonIndexBuilder::new("user_theme_idx")
    .table("users")
    .generated_column("theme", "settings->>'$.theme'", "VARCHAR(50)")
    .build();

// ALTER TABLE users ADD COLUMN theme VARCHAR(50) GENERATED ALWAYS AS (settings->>'$.theme') STORED;
// CREATE INDEX user_theme_idx ON users (theme);`;

  mongoDocument = `use prax::json::mongodb::{UpdateOp, ArrayOp, UpdateBuilder};

// MongoDB document operations
let update = UpdateBuilder::new()
    // Set fields
    .set("profile.bio", "Software Engineer")
    .set("updatedAt", Bson::DateTime(now()))
    // Increment
    .inc("stats.loginCount", 1)
    // Unset (remove field)
    .unset("tempField")
    // Rename field
    .rename("oldName", "newName")
    // Min/Max (only update if new value is less/greater)
    .min("stats.minScore", 50)
    .max("stats.maxScore", 100)
    // Multiply
    .mul("balance", 1.1)  // Increase by 10%
    .build();

// Array operations
let array_update = UpdateBuilder::new()
    // Push to array
    .push("tags", "premium")
    // Push multiple with sort and slice
    .push_each("scores", [95, 87, 92])
        .sort(-1)           // Sort descending
        .slice(10)          // Keep top 10
    // Pull (remove from array)
    .pull("tags", "trial")
    // Pull matching condition
    .pull_all("notifications", [
        doc! { "read": true, "age": { "$gt": 30 } }
    ])
    // Add to set (only if not exists)
    .add_to_set("roles", "member")
    // Pop first or last
    .pop("queue", -1)  // Remove first element
    .build();

// Positional updates (update matched array element)
client.orders().update_one(
    doc! { "_id": order_id, "items.productId": product_id },
    doc! { "$set": { "items.$.quantity": 5 } }  // Update matched item
).await?;

// Array filters for nested arrays
client.orders().update_one(
    doc! { "_id": order_id },
    doc! { "$set": { "items.$[elem].shipped": true } },
    UpdateOptions::builder()
        .array_filters([doc! { "elem.status": "ready" }])
        .build()
).await?;`;

  nestedDocuments = `// Schema with embedded documents
model User {
  id       String   @id @default(auto()) @map("_id") @db.ObjectId
  email    String   @unique
  profile  Profile  // Embedded document
  settings Json     // Flexible JSON
}

type Profile {
  firstName String
  lastName  String
  avatar    String?
  social    Social?
}

type Social {
  twitter  String?
  github   String?
  linkedin String?
}

// Query nested fields
let users = client
    .user()
    .find_many()
    .where(user::profile::is(
        profile::firstName::contains("John")
    ))
    .exec()
    .await?;

// MongoDB dot notation
// { "profile.firstName": { "$regex": "John" } }

// Update nested fields
client
    .user()
    .update(user::id::equals(user_id))
    .data(user::Update {
        profile: Some(user::profile::update(profile::Update {
            avatar: Some("https://example.com/avatar.jpg".into()),
            ..Default::default()
        })),
        ..Default::default()
    })
    .exec()
    .await?;`;
}
