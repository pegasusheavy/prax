import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-schema-relations-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './schema-relations.page.html',
})
export class SchemaRelationsPage {
  relationBasics = `// Relations connect models together
model User {
    id    Int    @id @auto
    posts Post[]  // One-to-Many: User has many Posts
}

model Post {
    id       Int  @id @auto
    // Foreign key field
    authorId Int
    // Relation to User model
    author   User @relation(fields: [authorId], references: [id])
}

// Relations are always defined on BOTH sides:
// - One side has the foreign key field(s) + @relation attribute
// - Other side has the array or optional reference`;

  oneToOne = `// One-to-One: User has exactly one Profile
model User {
    id      Int      @id @auto
    email   String   @unique
    profile Profile?  // Optional: User might not have a profile
}

model Profile {
    id     Int    @id @auto
    bio    String?
    avatar String?

    // Foreign key (must be @unique for 1:1)
    userId Int    @unique
    user   User   @relation(fields: [userId], references: [id])
}

// Alternative: Profile ID is also the User ID
model UserWithProfile {
    id   Int @id @auto
    name String
}

model ProfileByUserId {
    // Use same ID as User (shared primary key)
    userId Int   @id
    bio    String?
    user   UserWithProfile @relation(fields: [userId], references: [id])
}`;

  oneToMany = `// One-to-Many: User has many Posts
model User {
    id    Int    @id @auto
    email String @unique
    posts Post[] // Array indicates "many" side
}

model Post {
    id       Int    @id @auto
    title    String
    content  String?

    // "One" side has the foreign key
    authorId Int
    author   User   @relation(fields: [authorId], references: [id])
}

// One-to-Many with optional relationship
model Category {
    id    Int     @id @auto
    name  String
    posts Post[]
}

model PostWithCategory {
    id         Int       @id @auto
    title      String
    categoryId Int?      // Optional foreign key
    category   Category? @relation(fields: [categoryId], references: [id])
}`;

  manyToMany = `// Many-to-Many: Posts have many Tags, Tags have many Posts
// Implicit join table (Prax manages it)
model Post {
    id    Int    @id @auto
    title String
    tags  Tag[]  // Many tags per post
}

model Tag {
    id    Int    @id @auto
    name  String @unique
    posts Post[] // Many posts per tag
}

// Explicit join table (you manage it)
// Use when you need additional fields on the relationship
model PostTagExplicit {
    // Composite primary key
    postId Int
    tagId  Int

    // Additional relationship data
    addedAt   DateTime @default(now())
    addedById Int?

    // Relations
    post Post @relation(fields: [postId], references: [id])
    tag  Tag  @relation(fields: [tagId], references: [id])

    @@id([postId, tagId])
    @@index([tagId])
}`;

  selfRelation = `// Self-relation: Comments can have replies (tree structure)
model Comment {
    id       Int       @id @auto
    content  String
    postId   Int

    // Self-relation for nested comments
    parentId Int?
    parent   Comment?  @relation("CommentReplies", fields: [parentId], references: [id])
    replies  Comment[] @relation("CommentReplies")
}

// Self-relation: Users can follow other users
model User {
    id         Int    @id @auto
    name       String

    // Users I follow
    following  User[] @relation("UserFollows")
    // Users following me
    followers  User[] @relation("UserFollows")
}

// Self-relation: Employee -> Manager hierarchy
model Employee {
    id        Int        @id @auto
    name      String
    managerId Int?
    manager   Employee?  @relation("EmployeeManager", fields: [managerId], references: [id])
    reports   Employee[] @relation("EmployeeManager")
}`;

  multipleRelations = `// Multiple relations between the same models
model User {
    id            Int       @id @auto
    email         String    @unique
    writtenPosts  Post[]    @relation("PostAuthor")    // Posts I wrote
    editedPosts   Post[]    @relation("PostEditor")    // Posts I edited
    likedPosts    Post[]    @relation("PostLikes")     // Posts I liked
}

model Post {
    id         Int      @id @auto
    title      String
    content    String?

    // Different relations to User
    authorId   Int
    author     User     @relation("PostAuthor", fields: [authorId], references: [id])

    editorId   Int?
    editor     User?    @relation("PostEditor", fields: [editorId], references: [id])

    likedBy    User[]   @relation("PostLikes")  // Many-to-many
}`;

  refActions = `// Referential actions control cascading behavior
model User {
    id    Int    @id @auto
    posts Post[]
}

model Post {
    id       Int  @id @auto
    authorId Int

    author User @relation(
        fields: [authorId],
        references: [id],
        onDelete: Cascade,     // Delete posts when user is deleted
        onUpdate: Cascade      // Update FK when user ID changes
    )
}

// All referential actions
model Example {
    parentId Int
    parent   Parent @relation(
        fields: [parentId],
        references: [id],
        onDelete: Cascade,     // Delete this when parent deleted
        // onDelete: Restrict,  // Prevent parent deletion if this exists
        // onDelete: SetNull,   // Set FK to NULL (field must be optional)
        // onDelete: SetDefault,// Set FK to default value
        // onDelete: NoAction,  // Database default (usually error)
        onUpdate: Cascade
    )
}`;

  compositeRelations = `// Relation using composite foreign key
model TenantUser {
    tenantId Int
    id       Int
    email    String
    posts    TenantPost[]

    @@id([tenantId, id])
}

model TenantPost {
    tenantId     Int
    id           Int
    title        String

    // Composite foreign key
    authorTenant Int
    authorId     Int
    author       TenantUser @relation(
        fields: [authorTenant, authorId],
        references: [tenantId, id]
    )

    @@id([tenantId, id])
    @@index([authorTenant, authorId])
}`;

  queryingRelations = `use prax::generated::{user, post, include};

// Include related data (eager loading)
let user_with_posts = client
    .user()
    .find_unique()
    .where(user::id::equals(1))
    .include(user::posts::fetch())
    .exec()
    .await?;

// Nested includes
let user_with_full_posts = client
    .user()
    .find_unique()
    .where(user::id::equals(1))
    .include(user::posts::fetch()
        .include(post::tags::fetch())
        .include(post::comments::fetch()))
    .exec()
    .await?;

// Filter by related records
let users_with_published = client
    .user()
    .find_many()
    .where(user::posts::some(post::published::equals(true)))
    .exec()
    .await?;

// Filter: all, some, none, is, isNot
let authors = client
    .user()
    .find_many()
    .where(user::posts::some(post::likes::gt(100)))  // Has popular post
    .where(user::profile::is(profile::verified::equals(true)))  // Verified
    .exec()
    .await?;`;

  nestedWrites = `use prax_query::data;

// Create with nested relation
let user = client
    .user()
    .create(data! {
        email: "alice@example.com",
        name: "Alice",
        // Create related profile
        profile: {
            create: {
                bio: "Software engineer",
                avatar: "https://example.com/alice.jpg"
            }
        },
        // Create multiple posts
        posts: {
            create: [
                { title: "Hello World", content: "My first post" },
                { title: "Second Post", published: true }
            ]
        }
    })
    .exec()
    .await?;

// Connect to existing records
let post = client
    .post()
    .create(data! {
        title: "New Post",
        author: {
            connect: { id: 1 }
        },
        tags: {
            connect: [{ id: 1 }, { id: 2 }]
        }
    })
    .exec()
    .await?;

// Disconnect relations
let post = client
    .post()
    .update()
    .where(post::id::equals(1))
    .data(data! {
        tags: {
            disconnect: [{ id: 3 }]
        }
    })
    .exec()
    .await?;`;

  bestPractices = `// ✅ Good: Clear naming for relation fields
model User {
    id            Int      @id @auto
    posts         Post[]   @relation("AuthoredPosts")
    favoriteBooks Book[]   @relation("FavoriteBooks")
}

// ✅ Good: Index foreign key columns
model Post {
    id       Int  @id @auto
    authorId Int
    author   User @relation(fields: [authorId], references: [id])

    @@index([authorId])  // Important for query performance!
}

// ✅ Good: Use Cascade carefully
model UserSession {
    id     Int  @id @auto
    userId Int
    user   User @relation(
        fields: [userId],
        references: [id],
        onDelete: Cascade  // Sessions deleted when user deleted
    )
}

// ⚠️ Careful: Restrict for important data
model Order {
    id         Int  @id @auto
    customerId Int
    customer   Customer @relation(
        fields: [customerId],
        references: [id],
        onDelete: Restrict  // Can't delete customer with orders
    )
}`;
}
