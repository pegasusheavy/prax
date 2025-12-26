import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-schema-attributes-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './schema-attributes.page.html',
})
export class SchemaAttributesPage {
  // Field Attributes
  fieldAttrsBasic = `model User {
    id        Int      @id              // Primary key
    count     Int      @auto            // Auto-increment
    email     String   @unique          // Unique constraint
    active    Boolean  @default(true)   // Default value
    createdAt DateTime @default(now())  // Function default
    updatedAt DateTime @updatedAt       // Auto-update timestamp
    firstName String   @map("first_name") // Column name mapping
    data      Json?    @ignore          // Ignore in client
}`;

  fieldAttrsAdvanced = `model Post {
    // UUID with auto-generation
    id        String   @id @default(uuid())

    // CUID alternatives
    publicId  String   @default(cuid())
    shortId   String   @default(nanoid())
    sortId    String   @default(ulid())

    // Database-specific column types
    content   String   @db.Text
    price     Decimal  @db.Decimal(10, 2)
    metadata  Json     @db.JsonB

    // Relation with custom name
    author    User     @relation("PostAuthor")
}`;

  // Model Attributes
  modelAttrs = `model User {
    id       Int    @id @auto
    email    String
    tenantId Int
    role     String

    // Map to different table name
    @@map("users")

    // Single-field index
    @@index([email])

    // Composite index with name
    @@index([tenantId, role], name: "tenant_role_idx")

    // Composite unique constraint
    @@unique([email, tenantId])

    // Composite primary key
    @@id([tenantId, id])

    // Full-text search index (PostgreSQL)
    @@index([name, bio], type: GIN)
}`;

  // Relation Attributes
  relationAttrs = `model Post {
    id       Int  @id @auto
    author   User @relation(
        fields: [authorId],      // Foreign key field(s)
        references: [id],        // Referenced field(s)
        name: "UserPosts",       // Relation name (for disambiguation)
        onDelete: Cascade,       // Delete behavior
        onUpdate: Cascade        // Update behavior
    )
    authorId Int
}

model Comment {
    id       Int  @id @auto
    // Self-relation example
    parent   Comment?  @relation("CommentReplies", fields: [parentId], references: [id])
    parentId Int?
    replies  Comment[] @relation("CommentReplies")
}`;

  // String Validators
  stringValidators = `model User {
    // Email format validation
    email       String  @validate.email

    // URL format validation
    website     String? @validate.url

    // ID format validators
    externalId  String  @validate.uuid
    trackingId  String  @validate.cuid
    shortCode   String  @validate.nanoid
    sortableId  String  @validate.ulid

    // Length constraints
    username    String  @validate.minLength(3) @validate.maxLength(30)
    bio         String? @validate.length(10, 500)

    // Pattern matching
    phone       String  @validate.regex("^\\+[1-9]\\d{1,14}$")
    slug        String  @validate.slug

    // Content validators
    firstName   String  @validate.alpha
    code        String  @validate.alphanumeric
    lowerName   String  @validate.lowercase
    upperCode   String  @validate.uppercase

    // String content
    searchTerm  String  @validate.startsWith("search:")
    filePath    String  @validate.endsWith(".json")
    keywords    String  @validate.contains("prax")

    // Whitespace handling
    cleanInput  String  @validate.trim
    noSpaces    String  @validate.noWhitespace

    // Network validators
    ipAddress   String  @validate.ip
    ipv4Only    String  @validate.ipv4
    ipv6Only    String  @validate.ipv6

    // Format validators
    cardNumber  String  @validate.creditCard
    phoneNum    String  @validate.phone
    hexColor    String  @validate.hex
    encoded     String  @validate.base64
    jsonStr     String  @validate.json
}`;

  // Numeric Validators
  numericValidators = `model Product {
    // Range constraints
    price       Decimal @validate.min(0)
    discount    Int     @validate.max(100)
    quantity    Int     @validate.range(0, 10000)

    // Sign validators
    rating      Float   @validate.positive
    adjustment  Int     @validate.negative
    balance     Decimal @validate.nonNegative
    debt        Decimal @validate.nonPositive

    // Type validators
    wholeNumber Float   @validate.integer
    percentage  Float   @validate.multipleOf(0.01)
    safeNumber  Float   @validate.finite
}`;

  // Array Validators
  arrayValidators = `model Post {
    // Array length constraints
    tags        String[] @validate.minItems(1)
    categories  String[] @validate.maxItems(5)
    keywords    String[] @validate.items(1, 10)

    // Array content validators
    uniqueTags  String[] @validate.unique
    requiredArr String[] @validate.nonEmpty
}`;

  // Date Validators
  dateValidators = `model Event {
    // Relative date validators
    birthDate   DateTime @validate.past
    eventDate   DateTime @validate.future
    lastLogin   DateTime @validate.pastOrPresent
    nextReview  DateTime @validate.futureOrPresent

    // Absolute date constraints
    startDate   DateTime @validate.after("2024-01-01")
    endDate     DateTime @validate.before("2025-12-31")
}`;

  // General Validators
  generalValidators = `model User {
    // Required even if type is optional
    middleName  String? @validate.required

    // Non-empty (works with strings, arrays, etc.)
    nickname    String  @validate.notEmpty

    // Enum-like constraint
    status      String  @validate.oneOf("active", "inactive", "pending")
    role        String  @validate.oneOf("admin", "user", "guest")

    // Custom validator function
    password    String  @validate.custom("strongPassword")
}`;

  // ID Generation - UUID
  uuidExamples = `model User {
    // UUID v4 as primary key (recommended for distributed systems)
    id        String   @id @default(uuid())

    // UUID with native database type (PostgreSQL)
    id        String   @id @default(uuid()) @db.Uuid
}

model Session {
    // UUID for non-primary key fields
    id        Int      @id @auto
    token     String   @unique @default(uuid())
}

// Example generated values:
// "550e8400-e29b-41d4-a716-446655440000"
// "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
// "f47ac10b-58cc-4372-a567-0e02b2c3d479"`;

  // ID Generation - CUID
  cuidExamples = `model Post {
    // CUID as primary key (collision-resistant, sortable)
    id        String   @id @default(cuid())

    // CUID for public-facing IDs
    slug      String   @unique @default(cuid())
}

model ApiKey {
    // CUID2 - more secure, shorter (next generation)
    id        Int      @id @auto
    key       String   @unique @default(cuid2())
}

// CUID example values (25 chars, starts with 'c'):
// "cjld2cjxh0000qzrmn831i7rn"
// "cjld2cyuq0000t3rmniod1foy"

// CUID2 example values (24 chars, random start):
// "tz4a98xxat96iws9zmbrgj3a"
// "pfh0haxfpzowht3oi213cqos"`;

  // ID Generation - NanoID
  nanoidExamples = `model ShortUrl {
    // NanoID - URL-friendly, customizable length
    id        String   @id @default(nanoid())     // 21 chars default
    code      String   @unique @default(nanoid(8)) // 8 chars custom
}

model InviteCode {
    id        Int      @id @auto
    // Short codes for sharing
    code      String   @unique @default(nanoid(6))
}

// NanoID example values (21 chars default):
// "V1StGXR8_Z5jdHi6B-myT"
// "FwcE6X9N4f7kLpQrS2hJm"

// NanoID(8) example values:
// "5fX8pK2m"
// "R9qW3vNx"`;

  // ID Generation - ULID
  ulidExamples = `model Event {
    // ULID - sortable by creation time
    id        String   @id @default(ulid())
}

model LogEntry {
    // ULID preserves insertion order
    id        String   @id @default(ulid())
    level     String
    message   String
    timestamp DateTime @default(now())
}

// ULID example values (26 chars, time-sortable):
// "01ARZ3NDEKTSV4RRFFQ69G5FAV"
// "01BX5ZZKBKACTAV9WEVGEMMVRY"
// Structure: TTTTTTTTTTSSSSSSSSSSSSSSSS
//            |---------|--------------|
//            timestamp    randomness`;

  // ID Generation Comparison
  idComparison = `// Choose the right ID type for your use case:

model Example {
    // UUID v4 - Universal, widely supported
    // ✓ Standard format (RFC 4122)
    // ✓ Native database support (PostgreSQL UUID type)
    // ✗ Not sortable by creation time
    // ✗ Longer (36 chars with dashes)
    uuidId      String @default(uuid())

    // CUID - Collision-resistant, horizontal scaling
    // ✓ Sortable (roughly by creation time)
    // ✓ URL-safe characters
    // ✓ Designed for distributed systems
    // ✗ Longer than NanoID (25 chars)
    cuidId      String @default(cuid())

    // CUID2 - Next generation CUID
    // ✓ More secure (unpredictable)
    // ✓ Shorter than CUID (24 chars)
    // ✓ Better entropy distribution
    // ✗ Not sortable by time
    cuid2Id     String @default(cuid2())

    // NanoID - Compact, customizable
    // ✓ URL-safe (A-Za-z0-9_-)
    // ✓ Customizable length
    // ✓ Smaller than UUID (21 chars default)
    // ✗ Not sortable by time
    nanoidId    String @default(nanoid())

    // ULID - Time-sortable
    // ✓ Lexicographically sortable
    // ✓ Encodes timestamp (first 10 chars)
    // ✓ Compatible with UUID format
    // ✗ Timestamp can leak creation time
    ulidId      String @default(ulid())

    // Auto-increment - Simple sequential
    // ✓ Smallest storage (integer)
    // ✓ Natural ordering
    // ✗ Reveals record count
    // ✗ Not suitable for distributed systems
    autoId      Int    @auto
}`;

  // Default Value Functions
  defaultFunctions = `model Record {
    // Timestamp functions
    createdAt   DateTime @default(now())

    // UUID generation (v4)
    id          String   @default(uuid())

    // CUID generation (collision-resistant)
    publicId    String   @default(cuid())

    // CUID2 (next generation, more secure)
    trackingId  String   @default(cuid2())

    // NanoID (URL-friendly, customizable)
    shortId     String   @default(nanoid())
    customId    String   @default(nanoid(10))  // Custom length

    // ULID (sortable)
    sortableId  String   @default(ulid())

    // Auto-increment (integers)
    sequence    Int      @default(autoincrement())

    // Database sequence
    orderNum    Int      @default(dbgenerated("nextval('order_seq')"))

    // Static defaults
    active      Boolean  @default(true)
    role        String   @default("user")
    count       Int      @default(0)
    tags        String[] @default([])
}`;

  // Database-specific Attributes
  dbSpecific = `model Document {
    // PostgreSQL types
    id          Int      @id @auto
    data        Json     @db.JsonB           // JSONB for indexing
    content     String   @db.Text            // TEXT type
    amount      Decimal  @db.Decimal(19, 4)  // DECIMAL(19,4)
    small       Int      @db.SmallInt        // SMALLINT
    big         BigInt   @db.BigInt          // BIGINT
    uuid        String   @db.Uuid            // UUID type
    xml         String   @db.Xml             // XML type
    inet        String   @db.Inet            // INET type
    cidr        String   @db.Cidr            // CIDR type
    macaddr     String   @db.MacAddr         // MACADDR type

    // MySQL types
    tinyText    String   @db.TinyText
    mediumText  String   @db.MediumText
    longText    String   @db.LongText
    tinyInt     Int      @db.TinyInt
    mediumInt   Int      @db.MediumInt
    year        Int      @db.Year

    // Column charset (MySQL)
    name        String   @db.VarChar(255) @db.Charset("utf8mb4")
}`;

  // Documentation Attributes
  docAttrs = `model User {
    /// @hidden - Exclude from public API
    internalId  String

    /// @internal - Admin-only visibility
    debugInfo   Json?

    /// @sensitive - Mask in logs
    ssn         String?

    /// @readonly - Cannot be set via API
    createdAt   DateTime @default(now())

    /// @writeonly - Not returned in responses
    password    String

    /// @deprecated Use 'email' instead
    /// @since 1.0.0
    oldEmail    String?

    /// @example "john@example.com"
    /// @label "Email Address"
    /// @placeholder "Enter your email"
    email       String

    /// @group "Personal Info"
    /// @order 1
    firstName   String

    /// @alias "userName"
    /// @json "user_name"
    username    String
}`;

  // Index Types
  indexTypes = `model SearchDocument {
    id       Int    @id @auto
    title    String
    content  String
    tags     String[]
    location Json

    // B-Tree index (default)
    @@index([title])

    // Hash index (equality only)
    @@index([id], type: Hash)

    // GIN index (arrays, JSONB, full-text)
    @@index([tags], type: GIN)
    @@index([content], type: GIN)  // Full-text search

    // GiST index (geometric, full-text)
    @@index([location], type: GiST)

    // BRIN index (large sorted tables)
    @@index([createdAt], type: BRIN)

    // Partial index with condition
    @@index([email], where: "active = true")

    // Unique index with nulls handling
    @@unique([email], nulls: NotDistinct)
}`;

  // Vector Index Types
  vectorIndexTypes = `// Vector indexes for AI/ML similarity search (requires pgvector)
// Database URL is configured in prax.toml, not in the schema
datasource db {
    provider   = "postgresql"
    extensions = [vector]
}

model Document {
    id        Int          @id @auto
    title     String
    content   String
    embedding Vector(1536)  // OpenAI embedding dimension

    // HNSW index - best recall, recommended for most use cases
    @@index([embedding], type: Hnsw, ops: Cosine)
}

model ImageSearch {
    id       Int        @id @auto
    features Vector(512)

    // IVFFlat index - faster builds for large datasets
    @@index([features], type: IvfFlat, ops: L2, lists: 100)
}

model AdvancedSearch {
    id        Int          @id @auto
    embedding Vector(768)

    // HNSW with custom parameters
    @@index([embedding], type: Hnsw, ops: Cosine, m: 32, ef_construction: 128)

    // Inner product for max similarity
    @@index([embedding], type: Hnsw, ops: InnerProduct, name: "ip_idx")
}

// Index Type Options:
// - type: Hnsw | IvfFlat
// - ops: Cosine | L2 | InnerProduct
// - m: HNSW connections (default 16)
// - ef_construction: HNSW build quality (default 64)
// - lists: IVFFlat clusters (default 100)`;

  // Index types reference table
  indexTypesTable = [
    { type: 'BTree', use: 'Default, range queries, sorting', pg: '✅', mysql: '✅', sqlite: '✅' },
    { type: 'Hash', use: 'Equality comparisons only', pg: '✅', mysql: '✅', sqlite: '❌' },
    { type: 'GIN', use: 'Arrays, JSONB, full-text', pg: '✅', mysql: '❌', sqlite: '❌' },
    { type: 'GiST', use: 'Geometric, full-text, ranges', pg: '✅', mysql: '❌', sqlite: '❌' },
    { type: 'BRIN', use: 'Large sorted tables', pg: '✅', mysql: '❌', sqlite: '❌' },
    { type: 'Hnsw', use: 'Vector similarity (best recall)', pg: '✅*', mysql: '❌', sqlite: '❌' },
    { type: 'IvfFlat', use: 'Vector similarity (fast build)', pg: '✅*', mysql: '❌', sqlite: '❌' },
  ];

  vectorOpsTable = [
    { op: 'Cosine', desc: 'Cosine distance (1 - similarity)', best: 'Text embeddings (normalized)' },
    { op: 'L2', desc: 'Euclidean distance', best: 'Image features (unnormalized)' },
    { op: 'InnerProduct', desc: 'Negative inner product', best: 'Max inner product search' },
  ];
}
