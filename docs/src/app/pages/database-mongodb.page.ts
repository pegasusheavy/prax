import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-database-mongodb-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './database-mongodb.page.html',
})
export class DatabaseMongodbPage {
  connectionExample = `// prax.toml configuration
[database]
provider = "mongodb"
url = "mongodb://localhost:27017/mydb"

# Replica set configuration
# url = "mongodb://primary:27017,secondary1:27017,secondary2:27017/mydb?replicaSet=rs0"

# MongoDB Atlas
# url = "mongodb+srv://user:password@cluster.mongodb.net/mydb"

[database.options]
app_name = "my-app"
max_pool_size = 10
min_pool_size = 2
connect_timeout = "10s"
server_selection_timeout = "30s"`;

  schemaExample = `// MongoDB schema definition
generator client {
  provider = "prax-mongodb"
  output   = "./generated"
}

datasource db {
  provider = "mongodb"
  url      = env("DATABASE_URL")
}

model User {
  id        String   @id @default(auto()) @map("_id") @db.ObjectId
  email     String   @unique
  name      String?
  profile   Profile? // Embedded document
  tags      String[] // Array field
  metadata  Json?    // Flexible JSON/BSON
  createdAt DateTime @default(now())

  @@map("users")
}

// Embedded document type
type Profile {
  bio       String?
  avatar    String?
  social    SocialLinks?
}

type SocialLinks {
  twitter   String?
  github    String?
  linkedin  String?
}`;

  aggregationViewExample = `use prax_mongodb::view::{AggregationView, stages, accumulators};

// Define an aggregation view
let user_stats = AggregationView::new("user_stats")
    .source("users")
    .pipeline([
        stages::lookup("posts", "id", "author_id", "user_posts"),
        stages::unwind("$user_posts", true),
        stages::group(
            "$_id",
            [
                ("email", accumulators::first("$email")),
                ("name", accumulators::first("$name")),
                ("post_count", accumulators::sum(1)),
                ("total_likes", accumulators::sum("$user_posts.likes")),
            ]
        ),
        stages::sort([("total_likes", -1)]),
    ]);

// Materialize with $merge
let materialized = user_stats
    .materialize("user_stats_cache")
    .on_match(MergeAction::Replace)
    .on_not_matched(MergeAction::Insert);`;

  changeStreamExample = `use prax::trigger::{ChangeStreamBuilder, ChangeType, ChangeStreamOptions};

// Watch for changes on a collection
let stream = ChangeStreamBuilder::new("users")
    .watch_events([ChangeType::Insert, ChangeType::Update, ChangeType::Delete])
    .filter(doc! {
        "fullDocument.status": "active"
    })
    .full_document(FullDocumentType::UpdateLookup)
    .build();

// Process changes
while let Some(change) = stream.next().await {
    match change.operation_type {
        ChangeType::Insert => {
            println!("New user: {:?}", change.full_document);
        }
        ChangeType::Update => {
            println!("Updated fields: {:?}", change.update_description);
        }
        ChangeType::Delete => {
            println!("Deleted: {:?}", change.document_key);
        }
    }
}`;

  shardingExample = `use prax::partition::mongodb::{ShardKey, ZoneShardingBuilder};

// Define shard key for a collection
let shard_key = ShardKey::builder()
    .hashed("tenant_id")  // Hashed sharding for even distribution
    .range("created_at")   // Range for time-series queries
    .build();

// Enable sharding
let command = shard_key.enable_sharding_command("orders");

// Zone sharding for geographic distribution
let zones = ZoneShardingBuilder::new("orders")
    .add_zone("US", "tenant_id", "us_", "us_~")
    .add_zone("EU", "tenant_id", "eu_", "eu_~")
    .add_zone("APAC", "tenant_id", "apac_", "apac_~")
    .build();`;

  atlasSearchExample = `use prax::search::mongodb::{AtlasSearchQuery, AtlasSearchIndexBuilder};

// Create Atlas Search index
let index = AtlasSearchIndexBuilder::new("default")
    .collection("products")
    .dynamic_mapping(true)
    .field("name", "string", [("analyzer", "lucene.standard")])
    .field("description", "string", [("analyzer", "lucene.english")])
    .field("price", "number")
    .field("location", "geo")
    .build();

// Full-text search with Atlas Search
let results = AtlasSearchQuery::new("wireless headphones")
    .index("default")
    .path(["name", "description"])
    .fuzzy(1, 3)  // maxEdits, prefixLength
    .highlight(["name", "description"])
    .score_boost("name", 2.0)
    .filter(doc! { "price": { "$lt": 200 } })
    .limit(20)
    .exec(&client)
    .await?;

// Access highlights
for result in results {
    println!("Score: {}", result.score);
    for highlight in result.highlights {
        println!("Match in {}: {}", highlight.path, highlight.texts.join("..."));
    }
}`;

  vectorSearchExample = `use prax::extension::mongodb::{VectorSearch, VectorIndex};

// Create vector search index
let index = VectorIndex::new("embedding_index")
    .collection("products")
    .field("embedding", 1536)  // OpenAI embedding dimension
    .similarity("cosine")
    .num_candidates(100)
    .build();

// Vector similarity search
let query_embedding = get_embedding("wireless bluetooth headphones").await?;
let similar = VectorSearch::new("embedding_index")
    .vector(query_embedding)
    .path("embedding")
    .limit(10)
    .filter(doc! { "category": "electronics" })
    .exec(&client)
    .await?;`;

  documentOpsExample = `use prax::json::mongodb::{UpdateOp, ArrayOp, UpdateBuilder};

// Atomic document updates
let update = UpdateBuilder::new()
    .set("profile.bio", "Software Engineer")
    .set("updatedAt", Bson::DateTime(now()))
    .inc("loginCount", 1)
    .push("tags", "verified")
    .add_to_set("roles", "admin")
    .unset("tempField")
    .build();

client.users().update_one(
    doc! { "_id": user_id },
    update
).await?;

// Array operations
let array_update = UpdateBuilder::new()
    .push_each("scores", [85, 90, 95])
    .pull("tags", "unverified")
    .pop("notifications", -1)  // Remove first element
    .build();

// Positional updates
let positional = UpdateBuilder::new()
    .set("items.$.quantity", 5)  // Update matched array element
    .build();

client.orders().update_one(
    doc! { "_id": order_id, "items.productId": product_id },
    positional
).await?;`;

  readPreferenceExample = `use prax::replication::mongodb::{MongoReadPreference, ReadConcern, WriteConcern};

// Configure read preference
let read_pref = MongoReadPreference::secondary_preferred()
    .max_staleness(Duration::from_secs(90))
    .tag_set([("region", "us-east-1"), ("type", "analytics")])
    .hedged(true);  // Send to multiple replicas, use first response

// Read concern levels
let concern = ReadConcern::Majority;  // Read from majority-committed data
// Options: Local, Available, Majority, Linearizable, Snapshot

// Write concern
let write_concern = WriteConcern::Majority
    .journal(true)
    .timeout(Duration::from_secs(5));

// Apply to operations
let users = client
    .users()
    .with_read_preference(read_pref)
    .with_read_concern(ReadConcern::Majority)
    .find_many()
    .exec()
    .await?;`;

  fieldEncryptionExample = `use prax::security::mongodb::{FieldEncryption, KmsProvider};

// Configure Client-Side Field Level Encryption (CSFLE)
let encryption = FieldEncryption::new()
    .kms_provider(KmsProvider::Aws {
        access_key_id: env!("AWS_ACCESS_KEY_ID"),
        secret_access_key: env!("AWS_SECRET_ACCESS_KEY"),
        region: "us-east-1",
    })
    .key_vault("encryption.__keyVault")
    .schema_map("users", doc! {
        "bsonType": "object",
        "encryptMetadata": {
            "keyId": "/keyAltName"
        },
        "properties": {
            "ssn": {
                "encrypt": {
                    "bsonType": "string",
                    "algorithm": "AEAD_AES_256_CBC_HMAC_SHA_512-Deterministic"
                }
            },
            "medicalRecords": {
                "encrypt": {
                    "bsonType": "array",
                    "algorithm": "AEAD_AES_256_CBC_HMAC_SHA_512-Random"
                }
            }
        }
    });

// Client with encryption enabled
let client = PraxClient::mongodb()
    .url(db_url)
    .encryption(encryption)
    .connect()
    .await?;`;

  atlasTriggerExample = `use prax_migrate::procedure::{AtlasTrigger, AtlasTriggerType, AtlasOperation};

// Database trigger (Atlas only)
let trigger = AtlasTrigger::new("user_signup_handler")
    .trigger_type(AtlasTriggerType::Database)
    .collection("users")
    .operations([AtlasOperation::Insert])
    .full_document(true)
    .function_name("onUserSignup");

// Scheduled trigger
let scheduled = AtlasTrigger::new("daily_report")
    .trigger_type(AtlasTriggerType::Scheduled)
    .schedule("0 0 * * *")  // Cron: Daily at midnight
    .function_name("generateDailyReport");

// Authentication trigger
let auth_trigger = AtlasTrigger::new("on_user_create")
    .trigger_type(AtlasTriggerType::Authentication)
    .operation_type("CREATE")
    .function_name("initializeUserProfile");`;
}




