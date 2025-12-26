import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-database-postgresql-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './database-postgresql.page.html',
})
export class DatabasePostgresqlPage {
  configCode = `[database]
provider = "postgresql"
url = "postgres://user:password@localhost:5432/mydb"

# Connection pool settings
[database.pool]
max_connections = 20
min_connections = 5
connect_timeout = 30
idle_timeout = 600`;

  connectionCode = `# Standard format
postgres://user:password@host:port/database

# With SSL
postgres://user:password@host:port/database?sslmode=require

# With schema
postgres://user:password@host:port/database?schema=myschema

# Environment variable
DATABASE_URL=postgres://...`;

  poolCode = `use prax_postgres::{PostgresPool, PostgresConfig};

// Create pool with custom config
let config = PostgresConfig::from_url("postgres://localhost/mydb")?
    .max_connections(20)
    .min_connections(5)
    .connect_timeout(Duration::from_secs(30));

let pool = PostgresPool::new(config).await?;

// Use the pool
let client = PraxClient::with_pool(pool).await?;`;

  typesCode = `model Document {
    id       Int      @id @auto
    data     Json     // JSONB
    metadata Json?

    // Array types
    tags     String[]
    scores   Int[]

    // UUID
    uuid     String   @default(uuid())

    // Full-text search
    @@index([data], type: GIN)
}`;

  // ============================================================
  // EXTENSIONS
  // ============================================================

  extensionsBasic = `// Enable PostgreSQL extensions in your datasource block
// Note: Database URL is configured in prax.toml, not in the schema
datasource db {
    provider   = "postgresql"
    extensions = [pg_trgm, vector, uuid-ossp]
}`;

  extensionsList = `// Common PostgreSQL extensions
datasource db {
    provider   = "postgresql"
    extensions = [
        pg_trgm,      // Trigram similarity for fuzzy text search
        vector,       // pgvector for AI/ML embeddings
        uuid-ossp,    // UUID generation functions
        pgcrypto,     // Cryptographic functions
        postgis,      // Geographic objects and spatial queries
        hstore,       // Key-value store
        ltree,        // Hierarchical tree-like data
        citext,       // Case-insensitive text
        cube,         // Multi-dimensional cubes
        tablefunc,    // Cross-tabulation and pivot tables
        fuzzystrmatch // Fuzzy string matching
    ]
}

// Database URL is configured in prax.toml:
// [database]
// provider = "postgresql"
// url = "postgres://user:pass@localhost:5432/mydb"
// # or use environment variable
// url = "\${DATABASE_URL}"`;

  extensionsMigration = `-- Generated migration for extensions
-- Up migration
CREATE EXTENSION IF NOT EXISTS "pg_trgm";
CREATE EXTENSION IF NOT EXISTS "vector";
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Down migration (rollback)
DROP EXTENSION IF EXISTS "uuid-ossp" CASCADE;
DROP EXTENSION IF EXISTS "vector" CASCADE;
DROP EXTENSION IF EXISTS "pg_trgm" CASCADE;`;

  // ============================================================
  // VECTOR TYPES
  // ============================================================

  vectorTypes = `// Vector types for AI/ML embeddings (requires pgvector extension)
datasource db {
    provider   = "postgresql"
    extensions = [vector]
}

model Document {
    id        Int           @id @auto
    title     String
    content   String

    // Dense vector - most common for embeddings
    // Dimension matches your embedding model output
    embedding Vector(1536)  // OpenAI text-embedding-ada-002
}

model ImageFeatures {
    id        Int            @id @auto
    imageUrl  String

    // Different embedding dimensions for different models
    clip      Vector(512)    // CLIP ViT-B/32
    resnet    Vector(2048)   // ResNet-50 features
}

model EfficientEmbeddings {
    id        Int              @id @auto

    // Half-precision vector - 50% storage savings
    halfVec   HalfVector(768)  // BERT-base dimension

    // Sparse vector - for sparse embeddings (SPLADE, BM25)
    sparse    SparseVector(30000)

    // Binary vector - for quantized/hashed embeddings
    binary    Bit(256)
}`;

  vectorTypesTable = [
    { type: 'Vector(dim)', rust: 'Vec<f32>', storage: '4 bytes × dim', use: 'Dense embeddings (OpenAI, Cohere, etc.)' },
    { type: 'HalfVector(dim)', rust: 'Vec<f32>', storage: '2 bytes × dim', use: '50% smaller, slight precision loss' },
    { type: 'SparseVector(dim)', rust: 'Vec<(u32, f32)>', storage: 'Variable', use: 'Sparse embeddings (SPLADE, learned sparse)' },
    { type: 'Bit(dim)', rust: 'Vec<u8>', storage: '⌈dim/8⌉ bytes', use: 'Binary quantization, LSH' },
  ];

  // ============================================================
  // VECTOR INDEXES
  // ============================================================

  vectorIndexHnsw = `// HNSW Index - Hierarchical Navigable Small World
// Best for: Most use cases, excellent recall
model Document {
    id        Int          @id @auto
    embedding Vector(1536)

    // Basic HNSW index with cosine distance
    @@index([embedding], type: Hnsw, ops: Cosine)
}

model HighQualitySearch {
    id        Int          @id @auto
    embedding Vector(768)

    // HNSW with tuned parameters for better recall
    @@index([embedding], type: Hnsw, ops: Cosine, m: 32, ef_construction: 128)
    // m: max connections per layer (higher = better recall, more memory)
    // ef_construction: build-time quality (higher = better recall, slower build)
}`;

  vectorIndexIvfflat = `// IVFFlat Index - Inverted File with Flat quantization
// Best for: Large datasets, faster index builds
model LargeDataset {
    id        Int          @id @auto
    embedding Vector(1536)

    // IVFFlat with 100 lists (good for ~100k-1M vectors)
    @@index([embedding], type: IvfFlat, ops: L2, lists: 100)
    // lists: number of clusters (sqrt(num_vectors) is a good starting point)
}

model VeryLargeDataset {
    id        Int          @id @auto
    embedding Vector(768)

    // More lists for larger datasets (10M+ vectors)
    @@index([embedding], type: IvfFlat, ops: Cosine, lists: 1000)
}`;

  vectorOpsTable = [
    { op: 'Cosine', pgOps: 'vector_cosine_ops', operator: '<=>', best: 'Text embeddings, normalized vectors' },
    { op: 'L2', pgOps: 'vector_l2_ops', operator: '<->', best: 'Image features, unnormalized vectors' },
    { op: 'InnerProduct', pgOps: 'vector_ip_ops', operator: '<#>', best: 'Max inner product search (MIPS)' },
  ];

  vectorIndexComparison = [
    { aspect: 'Build Speed', hnsw: 'Slower', ivfflat: 'Faster' },
    { aspect: 'Query Speed', hnsw: 'Very Fast', ivfflat: 'Fast' },
    { aspect: 'Recall', hnsw: 'Excellent (99%+)', ivfflat: 'Good (95%+)' },
    { aspect: 'Memory', hnsw: 'Higher', ivfflat: 'Lower' },
    { aspect: 'Best For', hnsw: 'Quality-critical apps', ivfflat: 'Large datasets, cost-sensitive' },
  ];

  vectorQueries = `use prax::generated::{document, Document};

// Find similar documents by embedding
let query_embedding: Vec<f32> = get_embedding("search query").await?;

// Cosine similarity search (lower distance = more similar)
let similar = client
    .document()
    .find_many()
    .order_by_vector_distance(
        document::embedding::cosine_distance(query_embedding.clone()),
        "ASC"
    )
    .take(10)
    .exec()
    .await?;

// L2 (Euclidean) distance search
let nearest = client
    .document()
    .find_many()
    .order_by_vector_distance(
        document::embedding::l2_distance(query_embedding.clone()),
        "ASC"
    )
    .take(5)
    .exec()
    .await?;

// Inner product search (higher = more similar)
let max_similarity = client
    .document()
    .find_many()
    .order_by_vector_distance(
        document::embedding::inner_product(query_embedding),
        "DESC"  // Note: DESC for inner product
    )
    .take(10)
    .exec()
    .await?;`;

  vectorBestPractices = `// ✅ Best Practices for Vector Search

// 1. Choose the right index type
model SmallDataset {       // < 100k vectors
    embedding Vector(1536)
    @@index([embedding], type: Hnsw, ops: Cosine)  // HNSW for best recall
}

model LargeDataset {       // > 1M vectors
    embedding Vector(1536)
    @@index([embedding], type: IvfFlat, ops: Cosine, lists: 1000)  // IVFFlat for efficiency
}

// 2. Match distance metric to your embeddings
model TextEmbeddings {
    embedding Vector(1536)  // OpenAI embeddings are normalized
    @@index([embedding], type: Hnsw, ops: Cosine)  // Use Cosine for normalized
}

model ImageFeatures {
    features Vector(2048)   // ResNet features are NOT normalized
    @@index([features], type: Hnsw, ops: L2)  // Use L2 for unnormalized
}

// 3. Tune HNSW parameters based on your needs
model HighRecall {
    embedding Vector(768)
    // Higher m and ef_construction = better recall, more resources
    @@index([embedding], type: Hnsw, ops: Cosine, m: 48, ef_construction: 200)
}

model BalancedPerformance {
    embedding Vector(768)
    // Default-ish values for balanced performance
    @@index([embedding], type: Hnsw, ops: Cosine, m: 16, ef_construction: 64)
}

// 4. Use HalfVector for storage efficiency (slight precision loss)
model StorageOptimized {
    embedding HalfVector(1536)  // Half the storage of Vector
    @@index([embedding], type: Hnsw, ops: Cosine)
}

// 5. Consider hybrid search (vector + keyword)
model HybridSearch {
    id        Int          @id @auto
    title     String
    content   String
    embedding Vector(1536)

    // Vector index for semantic search
    @@index([embedding], type: Hnsw, ops: Cosine)

    // GIN index for full-text keyword search
    @@index([title, content], type: GIN)
}`;

  vectorMigrationExample = `-- Generated SQL for vector indexes

-- HNSW index with cosine distance
CREATE INDEX "idx_document_embedding" ON "documents"
  USING hnsw ("embedding" vector_cosine_ops)
  WITH (m = 16, ef_construction = 64);

-- IVFFlat index with L2 distance
CREATE INDEX "idx_image_features" ON "images"
  USING ivfflat ("features" vector_l2_ops)
  WITH (lists = 100);

-- HNSW with inner product (for MIPS)
CREATE INDEX "idx_product_embedding" ON "products"
  USING hnsw ("embedding" vector_ip_ops)
  WITH (m = 32, ef_construction = 128);

-- Set probes for IVFFlat queries (runtime setting)
SET ivfflat.probes = 10;  -- Higher = better recall, slower

-- Set ef_search for HNSW queries (runtime setting)
SET hnsw.ef_search = 100;  -- Higher = better recall, slower`;
}
