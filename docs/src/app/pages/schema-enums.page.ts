import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-schema-enums-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './schema-enums.page.html',
})
export class SchemaEnumsPage {
  basicEnum = `// Basic enum definition
enum Role {
    USER
    ADMIN
    MODERATOR
}

// Using enums in models
model User {
    id    Int    @id @auto
    role  Role   @default(USER)
}`;

  enumWithValues = `// Enum with custom database values
enum Status {
    ACTIVE      @map("active")
    INACTIVE    @map("inactive")
    PENDING     @map("pending_review")
    SUSPENDED   @map("account_suspended")
}

// Map the entire enum to a different type name
enum OrderStatus {
    NEW
    PROCESSING
    SHIPPED
    DELIVERED
    CANCELLED
    REFUNDED

    @@map("order_status_enum")
}`;

  documentedEnum = `/// User subscription tier
/// Determines access levels and pricing
enum SubscriptionTier {
    /// Free tier with limited features
    FREE

    /// Basic paid tier
    /// @since 1.0.0
    BASIC

    /// Professional tier with all features
    /// @since 1.0.0
    PRO

    /// Enterprise tier with custom features
    /// @since 2.0.0
    ENTERPRISE
}`;

  enumWithModel = `// Complete example with multiple enums
enum UserStatus {
    ACTIVE
    INACTIVE
    SUSPENDED
    DELETED
}

enum NotificationType {
    EMAIL
    SMS
    PUSH
    IN_APP
}

enum Priority {
    LOW
    MEDIUM
    HIGH
    URGENT
}

model Notification {
    id        Int              @id @auto
    userId    Int
    user      User             @relation(fields: [userId], references: [id])
    type      NotificationType
    priority  Priority         @default(MEDIUM)
    title     String
    message   String
    read      Boolean          @default(false)
    createdAt DateTime         @default(now())

    @@index([userId, read])
    @@index([type, priority])
}`;

  enumArrays = `// Using enum arrays
enum Tag {
    FEATURED
    NEW
    SALE
    POPULAR
    LIMITED
}

enum Category {
    ELECTRONICS
    CLOTHING
    HOME
    SPORTS
    BOOKS
}

model Product {
    id         Int        @id @auto
    name       String
    tags       Tag[]      // Array of enum values
    categories Category[] // Multiple categories
    mainTag    Tag?       // Single optional enum

    @@index([tags], type: GIN)  // PostgreSQL GIN index for arrays
}`;

  enumBestPractices = `// ✅ Good: Descriptive enum names
enum PaymentStatus {
    PENDING_CONFIRMATION
    PROCESSING_PAYMENT
    PAYMENT_COMPLETED
    PAYMENT_FAILED
    REFUND_INITIATED
    REFUND_COMPLETED
}

// ✅ Good: Consistent naming convention (SCREAMING_SNAKE_CASE)
enum HttpMethod {
    GET
    POST
    PUT
    PATCH
    DELETE
    OPTIONS
    HEAD
}

// ✅ Good: Group related values
enum PermissionLevel {
    // Read permissions
    READ_OWN
    READ_TEAM
    READ_ALL

    // Write permissions
    WRITE_OWN
    WRITE_TEAM
    WRITE_ALL

    // Admin permissions
    ADMIN_TEAM
    ADMIN_ALL
}

// ❌ Avoid: Single-letter or unclear names
// enum S { A B C }  // Bad!

// ❌ Avoid: Mixing conventions
// enum Status { Active INACTIVE pending }  // Bad!`;

  enumInQueries = `// Generated Rust code usage
use prax::generated::{User, Role, user};

// Filter by enum value
let admins = client
    .user()
    .find_many()
    .where(user::role::equals(Role::ADMIN))
    .exec()
    .await?;

// Filter by multiple enum values
let privileged = client
    .user()
    .find_many()
    .where(user::role::in_(vec![Role::ADMIN, Role::MODERATOR]))
    .exec()
    .await?;

// Update with enum
let user = client
    .user()
    .update()
    .where(user::id::equals(1))
    .data(data! { role: Role::ADMIN })
    .exec()
    .await?;

// Create with enum default
let user = client
    .user()
    .create(data! {
        email: "user@example.com",
        // role defaults to USER
    })
    .exec()
    .await?;`;

  databaseMapping = `// PostgreSQL: Creates native ENUM type
// CREATE TYPE "Role" AS ENUM ('USER', 'ADMIN', 'MODERATOR');
enum Role {
    USER
    ADMIN
    MODERATOR
}

// MySQL: Uses ENUM column type
// ENUM('USER', 'ADMIN', 'MODERATOR')
enum Role {
    USER
    ADMIN
    MODERATOR
}

// SQLite: Uses CHECK constraint
// CHECK(role IN ('USER', 'ADMIN', 'MODERATOR'))
enum Role {
    USER
    ADMIN
    MODERATOR
}

// Custom database name
enum Role {
    USER
    ADMIN
    MODERATOR

    @@map("user_role")  // PostgreSQL type name: user_role
}`;
}


