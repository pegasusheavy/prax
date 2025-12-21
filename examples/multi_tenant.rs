#![allow(dead_code, unused, clippy::type_complexity)]
//! # Multi-Tenant Examples
//!
//! This example demonstrates multi-tenant support in Prax:
//! - Row-level tenant isolation
//! - Schema-based tenant isolation
//! - Database-per-tenant isolation
//! - Tenant middleware configuration
//! - Dynamic tenant resolution
//!
//! ## Running this example
//!
//! ```bash
//! cargo run --example multi_tenant
//! ```

use std::collections::HashMap;

// Tenant isolation strategies
#[derive(Debug, Clone)]
enum IsolationStrategy {
    /// All tenants share tables, filtered by tenant_id column
    RowLevel { tenant_column: String },
    /// Each tenant has a separate database schema
    Schema { schema_prefix: String },
    /// Each tenant has a separate database
    Database { url_template: String },
}

// Tenant context
#[derive(Debug, Clone)]
struct TenantContext {
    id: String,
    name: Option<String>,
    metadata: HashMap<String, String>,
}

impl TenantContext {
    fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            metadata: HashMap::new(),
        }
    }

    fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

// Tenant configuration
struct TenantConfig {
    strategy: IsolationStrategy,
    default_tenant: Option<String>,
    require_tenant: bool,
}

impl TenantConfig {
    fn builder() -> TenantConfigBuilder {
        TenantConfigBuilder::default()
    }
}

#[derive(Default)]
struct TenantConfigBuilder {
    strategy: Option<IsolationStrategy>,
    default_tenant: Option<String>,
    require_tenant: bool,
}

impl TenantConfigBuilder {
    fn strategy(mut self, strategy: IsolationStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    fn default_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.default_tenant = Some(tenant.into());
        self
    }

    fn require_tenant(mut self) -> Self {
        self.require_tenant = true;
        self
    }

    fn build(self) -> TenantConfig {
        TenantConfig {
            strategy: self.strategy.unwrap_or(IsolationStrategy::RowLevel {
                tenant_column: "tenant_id".to_string(),
            }),
            default_tenant: self.default_tenant,
            require_tenant: self.require_tenant,
        }
    }
}

// Tenant middleware
struct TenantMiddleware {
    config: TenantConfig,
}

impl TenantMiddleware {
    fn new(config: TenantConfig) -> Self {
        Self { config }
    }
}

// Tenant resolver trait
trait TenantResolver: Send + Sync {
    fn resolve(&self, request: &MockRequest) -> Option<TenantContext>;
}

// Header-based resolver
struct HeaderResolver {
    header_name: String,
}

impl HeaderResolver {
    fn new(header_name: impl Into<String>) -> Self {
        Self {
            header_name: header_name.into(),
        }
    }
}

impl TenantResolver for HeaderResolver {
    fn resolve(&self, request: &MockRequest) -> Option<TenantContext> {
        request
            .headers
            .get(&self.header_name)
            .map(|id| TenantContext::new(id.clone()))
    }
}

// Subdomain-based resolver
struct SubdomainResolver;

impl TenantResolver for SubdomainResolver {
    fn resolve(&self, request: &MockRequest) -> Option<TenantContext> {
        request.host.as_ref().and_then(|host| {
            let parts: Vec<&str> = host.split('.').collect();
            if parts.len() >= 2 {
                Some(TenantContext::new(parts[0].to_string()))
            } else {
                None
            }
        })
    }
}

// Mock request for demonstration
struct MockRequest {
    headers: HashMap<String, String>,
    host: Option<String>,
    path: String,
}

// Mock client with tenant support
struct TenantAwareClient {
    tenant: Option<TenantContext>,
    config: TenantConfig,
}

impl TenantAwareClient {
    fn new(config: TenantConfig) -> Self {
        Self {
            tenant: None,
            config,
        }
    }

    fn with_tenant(&self, tenant: impl Into<TenantContext>) -> Self {
        Self {
            tenant: Some(tenant.into()),
            config: TenantConfig {
                strategy: self.config.strategy.clone(),
                default_tenant: self.config.default_tenant.clone(),
                require_tenant: self.config.require_tenant,
            },
        }
    }

    fn user(&self) -> TenantUserQuery {
        TenantUserQuery {
            tenant: self.tenant.clone(),
            strategy: self.config.strategy.clone(),
        }
    }

    fn current_tenant(&self) -> Option<&TenantContext> {
        self.tenant.as_ref()
    }
}

impl From<String> for TenantContext {
    fn from(id: String) -> Self {
        TenantContext::new(id)
    }
}

impl From<&str> for TenantContext {
    fn from(id: &str) -> Self {
        TenantContext::new(id)
    }
}

#[derive(Debug, Clone)]
struct User {
    id: i32,
    email: String,
    tenant_id: String,
}

struct TenantUserQuery {
    tenant: Option<TenantContext>,
    strategy: IsolationStrategy,
}

impl TenantUserQuery {
    fn find_many(self) -> TenantUserFindMany {
        TenantUserFindMany {
            tenant: self.tenant,
            strategy: self.strategy,
        }
    }

    fn create(self, _data: CreateUserData) -> TenantUserCreate {
        TenantUserCreate {
            tenant: self.tenant,
            strategy: self.strategy,
        }
    }
}

struct TenantUserFindMany {
    tenant: Option<TenantContext>,
    strategy: IsolationStrategy,
}

impl TenantUserFindMany {
    async fn exec(self) -> Result<Vec<User>, Box<dyn std::error::Error>> {
        let tenant_id = self
            .tenant
            .as_ref()
            .map(|t| t.id.clone())
            .unwrap_or_else(|| "default".to_string());

        // Show how the query would be modified
        match &self.strategy {
            IsolationStrategy::RowLevel { tenant_column } => {
                println!(
                    "  [RowLevel] Adding WHERE {} = '{}'",
                    tenant_column, tenant_id
                );
            }
            IsolationStrategy::Schema { schema_prefix } => {
                println!("  [Schema] Using schema: {}_{}", schema_prefix, tenant_id);
            }
            IsolationStrategy::Database { url_template } => {
                println!(
                    "  [Database] Connecting to: {}",
                    url_template.replace("{tenant}", &tenant_id)
                );
            }
        }

        Ok(vec![
            User {
                id: 1,
                email: format!("user1@{}.example.com", tenant_id),
                tenant_id: tenant_id.clone(),
            },
            User {
                id: 2,
                email: format!("user2@{}.example.com", tenant_id),
                tenant_id,
            },
        ])
    }
}

struct CreateUserData {
    email: String,
}

struct TenantUserCreate {
    tenant: Option<TenantContext>,
    strategy: IsolationStrategy,
}

impl TenantUserCreate {
    async fn exec(self) -> Result<User, Box<dyn std::error::Error>> {
        let tenant_id = self
            .tenant
            .as_ref()
            .map(|t| t.id.clone())
            .unwrap_or_else(|| "default".to_string());

        match &self.strategy {
            IsolationStrategy::RowLevel { tenant_column } => {
                println!("  [RowLevel] Setting {} = '{}'", tenant_column, tenant_id);
            }
            IsolationStrategy::Schema { schema_prefix } => {
                println!(
                    "  [Schema] Inserting into: {}_{}.users",
                    schema_prefix, tenant_id
                );
            }
            IsolationStrategy::Database { .. } => {
                println!("  [Database] Inserting into tenant database");
            }
        }

        Ok(User {
            id: 3,
            email: format!("new@{}.example.com", tenant_id),
            tenant_id,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Prax Multi-Tenant Examples ===\n");

    // =========================================================================
    // ROW-LEVEL ISOLATION
    // =========================================================================
    println!("--- Row-Level Tenant Isolation ---");
    println!("All tenants share the same tables, filtered by tenant_id column.\n");

    let config = TenantConfig::builder()
        .strategy(IsolationStrategy::RowLevel {
            tenant_column: "tenant_id".to_string(),
        })
        .require_tenant()
        .build();

    let client = TenantAwareClient::new(config);

    // Set tenant context
    let tenant_client = client.with_tenant("acme-corp");

    println!("Querying users for tenant 'acme-corp':");
    let users = tenant_client.user().find_many().exec().await?;
    for user in &users {
        println!("  {} (tenant: {})", user.email, user.tenant_id);
    }
    println!();

    // Different tenant
    let other_tenant = client.with_tenant("other-corp");
    println!("Querying users for tenant 'other-corp':");
    let users = other_tenant.user().find_many().exec().await?;
    for user in &users {
        println!("  {} (tenant: {})", user.email, user.tenant_id);
    }
    println!();

    // =========================================================================
    // SCHEMA-BASED ISOLATION
    // =========================================================================
    println!("--- Schema-Based Tenant Isolation ---");
    println!("Each tenant has a separate database schema.\n");

    let schema_config = TenantConfig::builder()
        .strategy(IsolationStrategy::Schema {
            schema_prefix: "tenant".to_string(),
        })
        .build();

    let schema_client = TenantAwareClient::new(schema_config);
    let tenant_client = schema_client.with_tenant("acme");

    println!("Querying users in schema 'tenant_acme':");
    let _users = tenant_client.user().find_many().exec().await?;
    println!();

    // =========================================================================
    // DATABASE-PER-TENANT ISOLATION
    // =========================================================================
    println!("--- Database-Per-Tenant Isolation ---");
    println!("Each tenant has a separate database.\n");

    let db_config = TenantConfig::builder()
        .strategy(IsolationStrategy::Database {
            url_template: "postgresql://localhost/{tenant}_db".to_string(),
        })
        .build();

    let db_client = TenantAwareClient::new(db_config);
    let tenant_client = db_client.with_tenant("acme");

    println!("Querying users in database 'acme_db':");
    let _users = tenant_client.user().find_many().exec().await?;
    println!();

    // =========================================================================
    // TENANT RESOLUTION
    // =========================================================================
    println!("--- Tenant Resolution ---");

    // Header-based resolution
    println!("Header-based resolver (X-Tenant-ID):");
    let header_resolver = HeaderResolver::new("X-Tenant-ID");

    let request = MockRequest {
        headers: [("X-Tenant-ID".to_string(), "acme-corp".to_string())]
            .into_iter()
            .collect(),
        host: None,
        path: "/api/users".to_string(),
    };

    if let Some(tenant) = header_resolver.resolve(&request) {
        println!("  Resolved tenant: {}", tenant.id);
    }
    println!();

    // Subdomain-based resolution
    println!("Subdomain-based resolver:");
    let subdomain_resolver = SubdomainResolver;

    let request = MockRequest {
        headers: HashMap::new(),
        host: Some("acme.myapp.com".to_string()),
        path: "/api/users".to_string(),
    };

    if let Some(tenant) = subdomain_resolver.resolve(&request) {
        println!("  Resolved tenant from 'acme.myapp.com': {}", tenant.id);
    }
    println!();

    // =========================================================================
    // TENANT CONTEXT WITH METADATA
    // =========================================================================
    println!("--- Tenant Context with Metadata ---");

    let tenant = TenantContext::new("acme-corp")
        .with_name("Acme Corporation")
        .with_metadata("plan", "enterprise")
        .with_metadata("region", "us-west");

    println!(
        "Tenant: {} ({})",
        tenant.id,
        tenant.name.as_deref().unwrap_or("")
    );
    println!("Metadata:");
    for (key, value) in &tenant.metadata {
        println!("  {}: {}", key, value);
    }
    println!();

    // =========================================================================
    // CREATING RECORDS WITH TENANT
    // =========================================================================
    println!("--- Creating Records with Tenant Context ---");

    let row_config = TenantConfig::builder()
        .strategy(IsolationStrategy::RowLevel {
            tenant_column: "tenant_id".to_string(),
        })
        .build();

    let client = TenantAwareClient::new(row_config);
    let tenant_client = client.with_tenant("acme-corp");

    println!("Creating user for tenant 'acme-corp':");
    let user = tenant_client
        .user()
        .create(CreateUserData {
            email: "new@acme-corp.example.com".to_string(),
        })
        .exec()
        .await?;

    println!("  Created: {} (tenant: {})", user.email, user.tenant_id);
    println!();

    // =========================================================================
    // CONFIGURATION REFERENCE
    // =========================================================================
    println!("--- Configuration Reference ---");
    println!(
        r#"
Multi-tenant configuration in prax.toml:

```toml
[tenant]
# Enable multi-tenant support
enabled = true

# Isolation strategy: "row_level", "schema", or "database"
strategy = "row_level"

# Row-level isolation settings
[tenant.row_level]
tenant_column = "tenant_id"
auto_filter = true
auto_set = true

# Schema-based isolation settings
[tenant.schema]
schema_prefix = "tenant_"
create_on_demand = true

# Database-per-tenant settings
[tenant.database]
url_template = "postgresql://localhost/{{tenant}}_db"
pool_per_tenant = true
max_tenants_cached = 100

# Tenant resolution
[tenant.resolver]
type = "header"  # "header", "subdomain", "path", or "custom"
header_name = "X-Tenant-ID"

# Default tenant (optional)
default_tenant = "public"

# Require tenant for all queries
require_tenant = true
```

Usage in code:

```rust
use prax::tenant::{{TenantConfig, IsolationStrategy}};

let config = TenantConfig::builder()
    .strategy(IsolationStrategy::RowLevel {{
        tenant_column: "tenant_id".into(),
    }})
    .require_tenant()
    .build();

let client = PraxClient::new(database_url)
    .await?
    .with_tenant_config(config);

// Set tenant for requests
let tenant_client = client.with_tenant("acme-corp");

// All queries are now scoped to this tenant
let users = tenant_client.user().find_many().exec().await?;
```
"#
    );

    println!("=== All examples completed successfully! ===");

    Ok(())
}
