#![allow(dead_code, unused, clippy::type_complexity)]
//! # Relations Examples
//!
//! This example demonstrates working with relations in Prax:
//! - One-to-one relations
//! - One-to-many relations
//! - Many-to-many relations
//! - Self-referential relations
//! - Eager loading with include()
//! - Nested writes
//!
//! ## Running this example
//!
//! ```bash
//! cargo run --example relations
//! ```


// Mock types representing generated models
#[derive(Debug, Clone)]
struct User {
    id: i32,
    email: String,
    name: Option<String>,
    // Relations (loaded on demand)
    posts: Option<Vec<Post>>,
    profile: Option<Box<Profile>>,
}

#[derive(Debug, Clone)]
struct Post {
    id: i32,
    title: String,
    content: Option<String>,
    author_id: i32,
    // Relations
    author: Option<Box<User>>,
    tags: Option<Vec<Tag>>,
    comments: Option<Vec<Comment>>,
}

#[derive(Debug, Clone)]
struct Profile {
    id: i32,
    bio: Option<String>,
    user_id: i32,
    // Relations
    user: Option<Box<User>>,
}

#[derive(Debug, Clone)]
struct Tag {
    id: i32,
    name: String,
    // Relations
    posts: Option<Vec<Post>>,
}

#[derive(Debug, Clone)]
struct Comment {
    id: i32,
    content: String,
    post_id: i32,
    parent_id: Option<i32>,
    // Relations
    post: Option<Box<Post>>,
    parent: Option<Box<Comment>>,
    replies: Option<Vec<Comment>>,
}

// Mock query builder for demonstration
struct MockClient;

impl MockClient {
    fn user(&self) -> UserQuery {
        UserQuery
    }

    fn post(&self) -> PostQuery {
        PostQuery
    }
}

struct UserQuery;

impl UserQuery {
    fn find_unique(self) -> UserFindUnique {
        UserFindUnique { includes: vec![] }
    }

    fn find_many(self) -> UserFindMany {
        UserFindMany { includes: vec![] }
    }

    fn create(self, _data: CreateUserData) -> UserCreate {
        UserCreate
    }
}

struct UserFindUnique {
    includes: Vec<String>,
}

impl UserFindUnique {
    #[allow(non_snake_case)]
    fn r#where(self, _filter: &str) -> Self {
        self
    }

    fn include(mut self, relation: &str) -> Self {
        self.includes.push(relation.to_string());
        self
    }

    async fn exec(self) -> Result<Option<User>, Box<dyn std::error::Error>> {
        // Mock response with included relations
        let mut user = User {
            id: 1,
            email: "alice@example.com".to_string(),
            name: Some("Alice".to_string()),
            posts: None,
            profile: None,
        };

        if self.includes.contains(&"posts".to_string()) {
            user.posts = Some(vec![
                Post {
                    id: 1,
                    title: "First Post".to_string(),
                    content: Some("Content...".to_string()),
                    author_id: 1,
                    author: None,
                    tags: None,
                    comments: None,
                },
                Post {
                    id: 2,
                    title: "Second Post".to_string(),
                    content: Some("More content...".to_string()),
                    author_id: 1,
                    author: None,
                    tags: None,
                    comments: None,
                },
            ]);
        }

        if self.includes.contains(&"profile".to_string()) {
            user.profile = Some(Box::new(Profile {
                id: 1,
                bio: Some("Software developer".to_string()),
                user_id: 1,
                user: None,
            }));
        }

        Ok(Some(user))
    }
}

struct UserFindMany {
    includes: Vec<String>,
}

impl UserFindMany {
    fn include(mut self, relation: &str) -> Self {
        self.includes.push(relation.to_string());
        self
    }

    fn take(self, _count: usize) -> Self {
        self
    }

    async fn exec(self) -> Result<Vec<User>, Box<dyn std::error::Error>> {
        Ok(vec![User {
            id: 1,
            email: "alice@example.com".to_string(),
            name: Some("Alice".to_string()),
            posts: if self.includes.contains(&"posts".to_string()) {
                Some(vec![])
            } else {
                None
            },
            profile: None,
        }])
    }
}

struct CreateUserData {
    email: String,
    name: Option<String>,
    posts: Option<NestedPostCreate>,
    profile: Option<NestedProfileCreate>,
}

struct NestedPostCreate {
    data: Vec<CreatePostData>,
}

struct CreatePostData {
    title: String,
    content: Option<String>,
}

struct NestedProfileCreate {
    data: CreateProfileData,
}

struct CreateProfileData {
    bio: Option<String>,
}

struct UserCreate;

impl UserCreate {
    fn include(self, _relation: &str) -> Self {
        self
    }

    async fn exec(self) -> Result<User, Box<dyn std::error::Error>> {
        Ok(User {
            id: 3,
            email: "new@example.com".to_string(),
            name: Some("New User".to_string()),
            posts: Some(vec![Post {
                id: 10,
                title: "My First Post".to_string(),
                content: Some("Hello world!".to_string()),
                author_id: 3,
                author: None,
                tags: None,
                comments: None,
            }]),
            profile: Some(Box::new(Profile {
                id: 3,
                bio: Some("Just joined!".to_string()),
                user_id: 3,
                user: None,
            })),
        })
    }
}

struct PostQuery;

impl PostQuery {
    fn find_unique(self) -> PostFindUnique {
        PostFindUnique { includes: vec![] }
    }
}

struct PostFindUnique {
    includes: Vec<String>,
}

impl PostFindUnique {
    #[allow(non_snake_case)]
    fn r#where(self, _filter: &str) -> Self {
        self
    }

    fn include(mut self, relation: &str) -> Self {
        self.includes.push(relation.to_string());
        self
    }

    async fn exec(self) -> Result<Option<Post>, Box<dyn std::error::Error>> {
        let mut post = Post {
            id: 1,
            title: "First Post".to_string(),
            content: Some("Content...".to_string()),
            author_id: 1,
            author: None,
            tags: None,
            comments: None,
        };

        if self.includes.contains(&"author".to_string()) {
            post.author = Some(Box::new(User {
                id: 1,
                email: "alice@example.com".to_string(),
                name: Some("Alice".to_string()),
                posts: None,
                profile: None,
            }));
        }

        if self.includes.contains(&"tags".to_string()) {
            post.tags = Some(vec![
                Tag {
                    id: 1,
                    name: "rust".to_string(),
                    posts: None,
                },
                Tag {
                    id: 2,
                    name: "tutorial".to_string(),
                    posts: None,
                },
            ]);
        }

        if self.includes.contains(&"comments".to_string()) {
            post.comments = Some(vec![Comment {
                id: 1,
                content: "Great post!".to_string(),
                post_id: 1,
                parent_id: None,
                post: None,
                parent: None,
                replies: Some(vec![Comment {
                    id: 2,
                    content: "Thanks!".to_string(),
                    post_id: 1,
                    parent_id: Some(1),
                    post: None,
                    parent: None,
                    replies: None,
                }]),
            }]);
        }

        Ok(Some(post))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Prax Relations Examples ===\n");

    let client = MockClient;

    // =========================================================================
    // ONE-TO-MANY: User -> Posts
    // =========================================================================
    println!("--- One-to-Many: User with Posts ---");

    let user = client
        .user()
        .find_unique()
        .r#where("id = 1")
        .include("posts")
        .exec()
        .await?
        .expect("User not found");

    println!(
        "User: {} ({})",
        user.email,
        user.name.as_deref().unwrap_or("")
    );
    if let Some(posts) = &user.posts {
        println!("Posts ({}):", posts.len());
        for post in posts {
            println!("  - {} (id: {})", post.title, post.id);
        }
    }
    println!();

    // =========================================================================
    // ONE-TO-ONE: User -> Profile
    // =========================================================================
    println!("--- One-to-One: User with Profile ---");

    let user = client
        .user()
        .find_unique()
        .r#where("id = 1")
        .include("profile")
        .exec()
        .await?
        .expect("User not found");

    println!("User: {}", user.email);
    if let Some(profile) = &user.profile {
        println!("Profile bio: {:?}", profile.bio);
    }
    println!();

    // =========================================================================
    // MULTIPLE RELATIONS
    // =========================================================================
    println!("--- Multiple Relations ---");

    let user = client
        .user()
        .find_unique()
        .r#where("id = 1")
        .include("posts")
        .include("profile")
        .exec()
        .await?
        .expect("User not found");

    println!("User: {}", user.email);
    println!("Has posts: {}", user.posts.is_some());
    println!("Has profile: {}", user.profile.is_some());
    println!();

    // =========================================================================
    // MANY-TO-MANY: Post -> Tags
    // =========================================================================
    println!("--- Many-to-Many: Post with Tags ---");

    let post = client
        .post()
        .find_unique()
        .r#where("id = 1")
        .include("tags")
        .exec()
        .await?
        .expect("Post not found");

    println!("Post: {}", post.title);
    if let Some(tags) = &post.tags {
        println!(
            "Tags: {:?}",
            tags.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
    }
    println!();

    // =========================================================================
    // REVERSE RELATION: Post -> Author
    // =========================================================================
    println!("--- Reverse Relation: Post with Author ---");

    let post = client
        .post()
        .find_unique()
        .r#where("id = 1")
        .include("author")
        .exec()
        .await?
        .expect("Post not found");

    println!("Post: {}", post.title);
    if let Some(author) = &post.author {
        println!(
            "Author: {} ({})",
            author.email,
            author.name.as_deref().unwrap_or("")
        );
    }
    println!();

    // =========================================================================
    // NESTED RELATIONS: Post with Comments and Replies
    // =========================================================================
    println!("--- Nested Relations: Post with Comments ---");

    let post = client
        .post()
        .find_unique()
        .r#where("id = 1")
        .include("comments")
        .exec()
        .await?
        .expect("Post not found");

    println!("Post: {}", post.title);
    if let Some(comments) = &post.comments {
        for comment in comments {
            println!("  Comment: {}", comment.content);
            if let Some(replies) = &comment.replies {
                for reply in replies {
                    println!("    Reply: {}", reply.content);
                }
            }
        }
    }
    println!();

    // =========================================================================
    // NESTED WRITES: Create User with Posts and Profile
    // =========================================================================
    println!("--- Nested Writes: Create User with Relations ---");

    let new_user = client
        .user()
        .create(CreateUserData {
            email: "new@example.com".to_string(),
            name: Some("New User".to_string()),
            posts: Some(NestedPostCreate {
                data: vec![CreatePostData {
                    title: "My First Post".to_string(),
                    content: Some("Hello world!".to_string()),
                }],
            }),
            profile: Some(NestedProfileCreate {
                data: CreateProfileData {
                    bio: Some("Just joined!".to_string()),
                },
            }),
        })
        .include("posts")
        .include("profile")
        .exec()
        .await?;

    println!("Created user: {} (id: {})", new_user.email, new_user.id);
    if let Some(posts) = &new_user.posts {
        println!("With {} post(s)", posts.len());
    }
    if let Some(profile) = &new_user.profile {
        println!("With profile: {:?}", profile.bio);
    }
    println!();

    // =========================================================================
    // SCHEMA EXAMPLE
    // =========================================================================
    println!("--- Relation Schema Example ---");

    let schema_example = r#"
// One-to-Many: User has many Posts
model User {
    id    Int    @id @auto
    email String @unique
    posts Post[]   // One-to-many relation
}

model Post {
    id       Int  @id @auto
    title    String
    authorId Int
    author   User @relation(fields: [authorId], references: [id])
}

// One-to-One: User has one Profile
model Profile {
    id     Int    @id @auto
    bio    String?
    userId Int    @unique  // Unique makes it one-to-one
    user   User   @relation(fields: [userId], references: [id])
}

// Many-to-Many: Post has many Tags, Tag has many Posts
model Tag {
    id    Int    @id @auto
    name  String @unique
    posts Post[] @relation("PostTags")
}

// Self-referential: Comment replies
model Comment {
    id       Int       @id @auto
    content  String
    parentId Int?
    parent   Comment?  @relation("CommentReplies", fields: [parentId], references: [id])
    replies  Comment[] @relation("CommentReplies")
}
"#;

    println!("Schema for relations:");
    println!("{}", schema_example);

    println!("=== All examples completed successfully! ===");

    Ok(())
}
