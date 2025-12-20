import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-schema-models-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './schema-models.page.html',
})
export class SchemaModelsPage {
  basicModel = `// A model represents a database table
model User {
    id    Int    @id @auto   // Primary key with auto-increment
    email String @unique     // Unique constraint
    name  String?            // Optional (nullable) field
}`;

  modelAnatomy = `model ModelName {
    // ┌─ Field name (camelCase convention)
    // │      ┌─ Field type (scalar, enum, or relation)
    // │      │        ┌─ Type modifier (? = optional, [] = array)
    // │      │        │  ┌─ Attributes (start with @)
    // │      │        │  │
    fieldName FieldType?  @attribute(args)

    // Model-level attributes start with @@
    @@modelAttribute([fields])
}`;

  fullModel = `/// User account in the system
/// Stores authentication and profile information
model User {
    // Primary key
    id           Int       @id @auto

    // Unique identifiers
    email        String    @unique
    username     String    @unique @validate.minLength(3)

    // Profile information
    name         String?
    bio          String?   @db.Text
    avatarUrl    String?   @map("avatar_url")

    // Authentication
    passwordHash String    @map("password_hash") @writeonly

    // Status and role
    role         Role      @default(USER)
    status       Status    @default(ACTIVE)
    emailVerified Boolean  @default(false) @map("email_verified")

    // Relations
    posts        Post[]
    comments     Comment[]
    profile      Profile?
    sessions     Session[]

    // Timestamps
    createdAt    DateTime  @default(now()) @map("created_at")
    updatedAt    DateTime  @updatedAt @map("updated_at")
    deletedAt    DateTime? @map("deleted_at")

    // Model attributes
    @@map("users")                          // Table name
    @@index([email])                        // Performance index
    @@index([createdAt])                    // For sorting
    @@unique([username, deletedAt])         // Soft-delete aware unique
}`;

  compositeKeys = `// Composite primary key
model PostTag {
    post   Post @relation(fields: [postId], references: [id])
    postId Int
    tag    Tag  @relation(fields: [tagId], references: [id])
    tagId  Int

    @@id([postId, tagId])  // Composite primary key
}

// Composite primary key with additional fields
model Membership {
    user      User   @relation(fields: [userId], references: [id])
    userId    Int
    org       Org    @relation(fields: [orgId], references: [id])
    orgId     Int
    role      String @default("member")
    joinedAt  DateTime @default(now())

    @@id([userId, orgId])  // User can only be in org once
}

// Multi-tenant composite key
model TenantUser {
    tenantId  Int
    id        Int       @default(autoincrement())
    email     String
    name      String?

    @@id([tenantId, id])              // Tenant-scoped ID
    @@unique([tenantId, email])       // Email unique per tenant
    @@map("tenant_users")
}`;

  indexes = `model Product {
    id          Int      @id @auto
    name        String
    sku         String   @unique
    price       Decimal
    category    String
    subcategory String
    brand       String?
    inStock     Boolean  @default(true)
    createdAt   DateTime @default(now())

    // Single-field indexes
    @@index([name])                           // Basic index
    @@index([createdAt])                      // For sorting

    // Composite indexes (for multi-column queries)
    @@index([category, subcategory])          // Category filtering
    @@index([category, price])                // Category + price range
    @@index([brand, inStock])                 // Brand with stock filter

    // Named index
    @@index([name, category], name: "product_search_idx")

    // Unique composite constraint
    @@unique([category, sku])

    // Hash index (equality queries only, PostgreSQL)
    @@index([sku], type: Hash)

    // GIN index for full-text search (PostgreSQL)
    @@index([name, description], type: GIN)

    // Partial/filtered index (PostgreSQL)
    @@index([price], where: "in_stock = true", name: "active_products_price")
}`;

  softDelete = `// Soft delete pattern
model Document {
    id        Int       @id @auto
    title     String
    content   String?
    createdAt DateTime  @default(now())
    updatedAt DateTime  @updatedAt
    deletedAt DateTime?                    // Soft delete marker

    // Index for efficient queries excluding deleted
    @@index([deletedAt])

    // Unique constraint that allows duplicates if deleted
    @@unique([title, deletedAt])
}

// Multi-tenant with soft delete
model TenantDocument {
    id        Int       @id @auto
    tenantId  Int
    title     String
    deletedAt DateTime?

    // Unique title per tenant (active documents only)
    @@unique([tenantId, title, deletedAt])
    @@index([tenantId, deletedAt])
}`;

  multiTenant = `// Row-level multi-tenancy
model TenantAwareModel {
    id        Int       @id @auto
    tenantId  Int                          // Tenant discriminator
    name      String

    @@index([tenantId])                    // Fast tenant filtering
    @@unique([tenantId, name])             // Unique per tenant
}

// Schema-based multi-tenancy
model SchemaAwareModel {
    id    Int    @id @auto
    name  String

    @@schema("tenant_<TENANT_ID>")        // Dynamic schema
}

// Database-based multi-tenancy (multiple datasources)
model DatabaseAwareModel {
    id    Int    @id @auto
    name  String

    @@datasource("tenant_<TENANT_ID>")   // Dynamic datasource
}`;

  documentation = `/// User account for authentication and profile management
///
/// This model stores user credentials and profile information.
/// Soft deletes are supported via the deletedAt field.
///
/// @since 1.0.0
/// @see Profile for extended profile information
/// @see Post for user's content
model User {
    /// Unique identifier, auto-generated
    /// @internal Used for foreign keys
    id        Int      @id @auto

    /// User's email address
    /// @example "john@example.com"
    /// @validation Must be a valid email format
    email     String   @unique @validate.email

    /// Display name
    /// @nullable
    /// @maxLength 100
    name      String?  @validate.maxLength(100)

    /// @deprecated Use 'role' enum instead
    /// @since 0.1.0
    /// @until 2.0.0
    isAdmin   Boolean  @default(false)
}`;

  naming = `// ✅ Good naming conventions
model User { }                    // Singular PascalCase
model BlogPost { }                // Multi-word PascalCase
model APIKey { }                  // Acronyms in caps

model Example {
    id        Int      @id       // Lowercase field names
    firstName String              // camelCase for multi-word
    createdAt DateTime            // Common timestamp names
    userId    Int                 // Foreign key: modelId

    @@map("examples")            // Lowercase plural table name
}

// ❌ Avoid these patterns
// model users { }               // Don't use plural
// model user { }                // Don't use lowercase
// model USER_TABLE { }          // Don't use SCREAMING_CASE
// model UserModel { }           // Don't add "Model" suffix`;

  generatedCode = `// Generated Rust code from the User model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub name: Option<String>,
    pub role: Role,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Query builder module
pub mod user {
    pub mod id {
        pub fn equals(value: i32) -> Filter { ... }
        pub fn in_(values: Vec<i32>) -> Filter { ... }
        pub fn lt(value: i32) -> Filter { ... }
        pub fn gt(value: i32) -> Filter { ... }
    }

    pub mod email {
        pub fn equals(value: &str) -> Filter { ... }
        pub fn contains(value: &str) -> Filter { ... }
        pub fn starts_with(value: &str) -> Filter { ... }
    }

    // ... more fields
}

// Usage
let users = client
    .user()
    .find_many()
    .where(user::role::equals(Role::ADMIN))
    .order_by(user::created_at::desc())
    .exec()
    .await?;`;
}
