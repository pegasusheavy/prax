#![allow(dead_code, unused, clippy::type_complexity)]
//! # Schema Parsing Examples
//!
//! This example demonstrates how to parse and work with Prax schema files:
//! - Parsing schema strings
//! - Parsing schema files
//! - Accessing model information
//! - Working with relations
//! - Validating schemas
//!
//! ## Running this example
//!
//! ```bash
//! cargo run --example schema_parsing
//! ```

use prax_orm::schema::{PraxConfig, parse_schema, validate_schema};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Prax Schema Parsing Examples ===\n");

    // =========================================================================
    // BASIC SCHEMA PARSING
    // =========================================================================
    println!("--- Basic Schema Parsing ---");

    let schema_str = r#"
        /// User account in the system
        model User {
            id        Int      @id @auto
            email     String   @unique
            name      String?
            role      Role     @default(User)
            createdAt DateTime @default(now())
            posts     Post[]
            profile   Profile?
        }

        /// Blog post
        model Post {
            id        Int      @id @auto
            title     String
            content   String?  @db.Text
            published Boolean  @default(false)
            authorId  Int
            author    User     @relation(fields: [authorId], references: [id])
        }

        /// User profile
        model Profile {
            id      Int    @id @auto
            bio     String?
            userId  Int    @unique
            user    User   @relation(fields: [userId], references: [id])
        }

        /// User roles
        enum Role {
            User
            Admin
            Moderator
        }
    "#;

    let schema = parse_schema(schema_str)?;

    println!("Parsed schema successfully!");
    println!("  Models: {}", schema.model_names().count());
    println!("  Enums: {}", schema.enum_names().count());
    println!();

    // =========================================================================
    // ACCESSING MODEL INFORMATION
    // =========================================================================
    println!("--- Accessing Model Information ---");

    // List all models
    println!("Models in schema:");
    for model_name in schema.model_names() {
        println!("  - {}", model_name);
    }
    println!();

    // Get specific model details
    if let Some(user_model) = schema.get_model("User") {
        println!("User model details:");
        println!("  Name: {}", user_model.name.name);
        println!("  Documentation: {:?}", user_model.documentation);
        println!("  Fields:");
        for (field_name, field) in &user_model.fields {
            let type_str = format!("{}", field.field_type);
            let attrs: Vec<String> = field
                .attributes
                .iter()
                .map(|a| format!("@{}", a.name.name))
                .collect();
            println!("    {} {} {}", field_name, type_str, attrs.join(" "));
        }
    }
    println!();

    // =========================================================================
    // WORKING WITH ENUMS
    // =========================================================================
    println!("--- Working with Enums ---");

    println!("Enums in schema:");
    for enum_name in schema.enum_names() {
        if let Some(enum_def) = schema.get_enum(enum_name) {
            println!("  {} with variants:", enum_def.name.name);
            for variant in &enum_def.variants {
                println!("    - {}", variant.name.name);
            }
        }
    }
    println!();

    // =========================================================================
    // VALIDATING SCHEMAS
    // =========================================================================
    println!("--- Schema Validation ---");

    // Valid schema
    let valid_schema = validate_schema(
        r#"
        model User {
            id    Int    @id @auto
            email String @unique
        }
    "#,
    );

    match valid_schema {
        Ok(_) => println!("✓ Valid schema parsed successfully"),
        Err(e) => println!("✗ Validation error: {}", e),
    }

    // Schema with potential issues
    let schema_with_relation = validate_schema(
        r#"
        model Post {
            id       Int    @id @auto
            title    String
            authorId Int
            author   User   @relation(fields: [authorId], references: [id])
        }

        model User {
            id    Int    @id @auto
            posts Post[]
        }
    "#,
    );

    match schema_with_relation {
        Ok(_) => println!("✓ Schema with relations validated"),
        Err(e) => println!("✗ Validation error: {}", e),
    }
    println!();

    // =========================================================================
    // SCHEMA STATISTICS
    // =========================================================================
    println!("--- Schema Statistics ---");

    let stats = schema.stats();
    println!("Schema statistics:");
    println!("  Total models: {}", stats.model_count);
    println!("  Total enums: {}", stats.enum_count);
    println!("  Total views: {}", stats.view_count);
    println!("  Total types: {}", stats.type_count);
    println!("  Total server groups: {}", stats.server_group_count);
    println!();

    // =========================================================================
    // WORKING WITH VIEWS
    // =========================================================================
    println!("--- Working with Views ---");

    let schema_with_view = parse_schema(
        r#"
        model User {
            id    Int    @id @auto
            email String @unique
            name  String?
        }

        /// Active users view
        view ActiveUsers {
            id    Int    @unique
            email String
            name  String?

            @@map("active_users_view")
        }
    "#,
    )?;

    println!("Views in schema:");
    for (view_name, view) in &schema_with_view.views {
        println!("  {} - {} fields", view_name, view.fields.len());
    }
    println!();

    // =========================================================================
    // WORKING WITH SERVER GROUPS
    // =========================================================================
    println!("--- Working with Server Groups ---");

    let schema_with_servers = parse_schema(
        r#"
        model User {
            id    Int    @id @auto
            email String @unique
        }

        serverGroup DatabaseCluster {
            primary {
                url  = env("PRIMARY_DB_URL")
                role = "primary"
            }

            replica1 {
                url    = env("REPLICA1_DB_URL")
                role   = "replica"
                weight = 50
            }

            replica2 {
                url    = env("REPLICA2_DB_URL")
                role   = "replica"
                weight = 50
            }

            @@strategy("read_replica")
            @@loadBalance("round_robin")
        }
    "#,
    )?;

    println!("Server groups in schema:");
    for sg_name in schema_with_servers.server_group_names() {
        if let Some(sg) = schema_with_servers.get_server_group(sg_name) {
            println!("  {} - {} servers", sg.name.name, sg.servers.len());
            for (server_name, _server) in &sg.servers {
                println!("    - {}", server_name);
            }
        }
    }
    println!();

    // =========================================================================
    // CONFIGURATION PARSING
    // =========================================================================
    println!("--- Configuration Parsing ---");

    let config_str = r#"
        [database]
        provider = "postgresql"
        url = "postgresql://localhost:5432/mydb"

        [database.pool]
        max_connections = 10

        [generator.client]
        output = "./src/generated"

        [migrations]
        directory = "./prax/migrations"
    "#;

    let config: PraxConfig = toml::from_str(config_str)?;

    println!("Configuration loaded:");
    println!("  Database provider: {:?}", config.database.provider);
    println!("  Database URL: {:?}", config.database.url);
    println!(
        "  Max connections: {}",
        config.database.pool.max_connections
    );
    println!("  Generator output: {}", config.generator.client.output);
    println!();

    println!("=== All examples completed successfully! ===");

    Ok(())
}
