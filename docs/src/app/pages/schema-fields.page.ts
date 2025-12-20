import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-schema-fields-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './schema-fields.page.html',
})
export class SchemaFieldsPage {
  basicTypes = `model Example {
    // Integer types
    id         Int       @id @auto          // 32-bit integer
    bigId      BigInt                       // 64-bit integer
    smallNum   Int       @db.SmallInt       // 16-bit (PostgreSQL)

    // Floating point
    price      Float                        // 64-bit float
    rating     Decimal   @db.Decimal(3, 2)  // Exact decimal

    // String types
    name       String                       // Variable length text
    bio        String    @db.Text           // Long text
    code       String    @db.Char(6)        // Fixed length

    // Boolean
    active     Boolean   @default(true)

    // Date/Time
    createdAt  DateTime  @default(now())
    birthDate  DateTime  @db.Date           // Date only
    loginTime  DateTime  @db.Time           // Time only

    // Binary
    avatar     Bytes                        // Binary data

    // JSON
    metadata   Json                         // JSON/JSONB
}`;

  modifiers = `model Example {
    // Required (default) - must have a value
    required   String

    // Optional - can be NULL
    optional   String?

    // Array/List - multiple values
    tags       String[]

    // Optional Array - entire array can be NULL
    optTags    String[]?
}

// Type modifier combinations
model Post {
    // Required string
    title      String

    // Optional string (nullable)
    subtitle   String?

    // Required array of strings
    tags       String[]

    // Optional array (array itself can be null)
    aliases    String[]?

    // Array with optional elements (PostgreSQL)
    // Each element can be null
    maybeItems String?[]
}`;

  attributes = `model User {
    // Primary key
    id           Int       @id

    // Auto-increment (usually with @id)
    sequence     Int       @auto

    // Unique constraint
    email        String    @unique

    // Default literal value
    role         String    @default("user")
    active       Boolean   @default(true)
    count        Int       @default(0)
    score        Float     @default(0.0)

    // Default function
    id           String    @default(uuid())
    publicId     String    @default(cuid())
    shortId      String    @default(nanoid())
    sortableId   String    @default(ulid())
    createdAt    DateTime  @default(now())

    // Auto-update timestamp
    updatedAt    DateTime  @updatedAt

    // Database-generated default
    trackingId   String    @default(dbgenerated("gen_random_uuid()"))

    // Column name mapping
    firstName    String    @map("first_name")

    // Ignore in client (still in DB)
    internalNote String?   @ignore

    // Database-specific type
    content      String    @db.Text
    amount       Decimal   @db.Decimal(10, 2)
}`;

  primaryKeys = `// Auto-increment integer (most common)
model User {
    id Int @id @auto
}

// UUID primary key
model Session {
    id String @id @default(uuid())
}

// CUID for distributed systems
model Event {
    id String @id @default(cuid())
}

// ULID for sortable IDs
model LogEntry {
    id String @id @default(ulid())
}

// NanoID for short URLs
model ShortLink {
    id String @id @default(nanoid())
}

// Composite primary key
model Membership {
    userId Int
    orgId  Int
    @@id([userId, orgId])
}

// Natural key (be careful with changes)
model Country {
    code String @id  // "US", "GB", etc.
    name String
}`;

  defaultValues = `model Record {
    // Literal defaults
    name      String   @default("")
    count     Int      @default(0)
    active    Boolean  @default(true)
    rating    Float    @default(0.0)
    tags      String[] @default([])
    config    Json     @default("{}")

    // Built-in functions
    id        String   @default(uuid())      // UUID v4
    publicId  String   @default(cuid())      // CUID
    shortId   String   @default(nanoid())    // NanoID (21 chars)
    tinyId    String   @default(nanoid(8))   // NanoID (custom length)
    sortId    String   @default(ulid())      // ULID
    createdAt DateTime @default(now())       // Current timestamp

    // Auto-increment
    sequence  Int      @default(autoincrement())

    // Enum default
    status    Status   @default(DRAFT)

    // Database-generated (PostgreSQL examples)
    trackId   String   @default(dbgenerated("gen_random_uuid()"))
    serial    Int      @default(dbgenerated("nextval('my_seq')"))
}`;

  timestamps = `model Record {
    // Creation timestamp - set once on insert
    createdAt DateTime @default(now())

    // Update timestamp - auto-updates on every change
    updatedAt DateTime @updatedAt

    // Soft delete timestamp - null means not deleted
    deletedAt DateTime?

    // Custom timestamp fields
    publishedAt DateTime?              // When published
    expiresAt   DateTime?              // Expiration date
    lastLoginAt DateTime?              // Last activity
}

// Database-specific timestamp types
model TimestampExample {
    // With timezone (PostgreSQL)
    createdAt   DateTime @db.Timestamptz

    // Without timezone
    localTime   DateTime @db.Timestamp

    // Date only (no time)
    birthDate   DateTime @db.Date

    // Time only (no date)
    startTime   DateTime @db.Time
}`;

  jsonFields = `model Settings {
    id       Int   @id @auto
    // JSON field (JSONB in PostgreSQL for indexing)
    config   Json  @db.JsonB

    // JSON with default
    options  Json  @default("{}")
    metadata Json  @default("{\\"version\\": 1}")
}

// Using JSON in queries
let users = client
    .user()
    .find_many()
    .where(user::settings::path(["theme"])::equals("dark"))
    .exec()
    .await?;

// JSON structure (not enforced by DB, document in comments)
/// Settings JSON structure:
/// {
///   "theme": "light" | "dark",
///   "notifications": {
///     "email": boolean,
///     "push": boolean
///   },
///   "language": string
/// }
model UserPreferences {
    id       Int  @id @auto
    settings Json @default("{}")
}`;

  arrayFields = `model Post {
    id       Int      @id @auto
    // String array
    tags     String[]
    // Integer array
    scores   Int[]
    // Enum array
    flags    Flag[]

    // Index for array contains queries (PostgreSQL)
    @@index([tags], type: GIN)
}

// Array operations in queries
let posts = client
    .post()
    .find_many()
    .where(post::tags::has("rust"))           // Contains element
    .where(post::tags::has_some(["rust", "orm"])) // Contains any
    .where(post::tags::has_every(["featured"]))   // Contains all
    .where(post::tags::is_empty(false))       // Not empty
    .exec()
    .await?;`;

  decimalFields = `// Decimal for exact precision (money, financial data)
model Product {
    id       Int     @id @auto
    // Default precision varies by database
    price    Decimal

    // Explicit precision: DECIMAL(10, 2)
    // 10 total digits, 2 after decimal
    // Range: -99999999.99 to 99999999.99
    cost     Decimal @db.Decimal(10, 2)

    // High precision for currency calculations
    amount   Decimal @db.Decimal(19, 4)

    // Tax rate (percentage)
    taxRate  Decimal @db.Decimal(5, 4)  // 0.0000 to 9.9999
}

// Decimal vs Float
model Comparison {
    // Use Decimal for:
    // - Money/currency
    // - Financial calculations
    // - When exact precision matters
    price    Decimal @db.Decimal(10, 2)

    // Use Float for:
    // - Scientific calculations
    // - When small rounding errors are acceptable
    // - Performance-critical calculations
    rating   Float
}`;

  bytesFields = `model File {
    id       Int    @id @auto
    name     String
    // Binary data (BYTEA in PostgreSQL, BLOB in MySQL)
    content  Bytes

    // File metadata
    mimeType String
    size     Int
}

model Image {
    id        Int    @id @auto
    // Store small images directly
    thumbnail Bytes?
    // Store image hash for deduplication
    hash      Bytes  @unique
}

// Note: For large files, consider storing in object storage
// (S3, GCS) and keeping only the URL/path in the database`;

  databaseTypes = `model DatabaseSpecific {
    id Int @id @auto

    // PostgreSQL-specific types
    uuid     String   @db.Uuid            // Native UUID
    jsonb    Json     @db.JsonB           // Binary JSON (indexable)
    xml      String   @db.Xml             // XML data
    inet     String   @db.Inet            // IP address
    cidr     String   @db.Cidr            // Network address
    macaddr  String   @db.MacAddr         // MAC address
    money    Decimal  @db.Money           // Currency
    tsVector String   @db.TsVector        // Full-text search vector

    // MySQL-specific types
    tinyText    String @db.TinyText       // 255 bytes
    mediumText  String @db.MediumText     // 16MB
    longText    String @db.LongText       // 4GB
    tinyInt     Int    @db.TinyInt        // -128 to 127
    mediumInt   Int    @db.MediumInt      // -8M to 8M
    year        Int    @db.Year           // 1901 to 2155
    bit         Int    @db.Bit(8)         // Bit field
    binary      Bytes  @db.Binary(16)     // Fixed binary
    varbinary   Bytes  @db.VarBinary(255) // Variable binary

    // SQLite types (limited type system)
    // All map to: INTEGER, REAL, TEXT, BLOB
    // Prax handles the conversion automatically
}`;
}
