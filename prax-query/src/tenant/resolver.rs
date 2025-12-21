//! Tenant resolvers for dynamic tenant lookup.

use super::context::{TenantContext, TenantId, TenantInfo};
use crate::error::QueryResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

/// A resolver that can look up tenant information.
#[async_trait]
pub trait TenantResolver: Send + Sync {
    /// Resolve a tenant ID to a full tenant context.
    async fn resolve(&self, tenant_id: &TenantId) -> QueryResult<TenantContext>;

    /// Validate that a tenant exists and is active.
    async fn validate(&self, tenant_id: &TenantId) -> QueryResult<bool> {
        Ok(self.resolve(tenant_id).await.is_ok())
    }

    /// Get the schema name for a tenant (schema-based isolation).
    async fn schema_for(&self, tenant_id: &TenantId) -> QueryResult<Option<String>> {
        let ctx = self.resolve(tenant_id).await?;
        Ok(ctx.info.schema)
    }

    /// Get the database name for a tenant (database-based isolation).
    async fn database_for(&self, tenant_id: &TenantId) -> QueryResult<Option<String>> {
        let ctx = self.resolve(tenant_id).await?;
        Ok(ctx.info.database)
    }
}

/// A static resolver that maps tenant IDs to contexts.
#[derive(Debug, Clone, Default)]
pub struct StaticResolver {
    tenants: Arc<RwLock<HashMap<String, TenantContext>>>,
}

impl StaticResolver {
    /// Create a new static resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tenant.
    pub fn register(&self, tenant_id: impl Into<String>, context: TenantContext) -> &Self {
        self.tenants
            .write()
            .expect("lock poisoned")
            .insert(tenant_id.into(), context);
        self
    }

    /// Register a simple tenant with just an ID.
    pub fn register_simple(&self, tenant_id: impl Into<String>) -> &Self {
        let id: String = tenant_id.into();
        let context = TenantContext::new(id.clone());
        self.register(id, context)
    }

    /// Unregister a tenant.
    pub fn unregister(&self, tenant_id: &str) -> Option<TenantContext> {
        self.tenants
            .write()
            .expect("lock poisoned")
            .remove(tenant_id)
    }

    /// Check if a tenant is registered.
    pub fn contains(&self, tenant_id: &str) -> bool {
        self.tenants
            .read()
            .expect("lock poisoned")
            .contains_key(tenant_id)
    }

    /// Get the number of registered tenants.
    pub fn len(&self) -> usize {
        self.tenants.read().expect("lock poisoned").len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl TenantResolver for StaticResolver {
    async fn resolve(&self, tenant_id: &TenantId) -> QueryResult<TenantContext> {
        self.tenants
            .read()
            .expect("lock poisoned")
            .get(tenant_id.as_str())
            .cloned()
            .ok_or_else(|| crate::error::QueryError::not_found(format!("Tenant {}", tenant_id)))
    }

    async fn validate(&self, tenant_id: &TenantId) -> QueryResult<bool> {
        Ok(self.contains(tenant_id.as_str()))
    }
}

/// Type alias for async resolver functions.
pub type ResolverFn = Arc<
    dyn Fn(TenantId) -> Pin<Box<dyn Future<Output = QueryResult<TenantContext>> + Send>>
        + Send
        + Sync,
>;

/// A dynamic resolver using a callback function.
pub struct DynamicResolver {
    resolve_fn: ResolverFn,
}

impl DynamicResolver {
    /// Create a new dynamic resolver with a callback.
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: Fn(TenantId) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = QueryResult<TenantContext>> + Send + 'static,
    {
        Self {
            resolve_fn: Arc::new(move |id| Box::pin(f(id))),
        }
    }
}

impl std::fmt::Debug for DynamicResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicResolver").finish()
    }
}

#[async_trait]
impl TenantResolver for DynamicResolver {
    async fn resolve(&self, tenant_id: &TenantId) -> QueryResult<TenantContext> {
        (self.resolve_fn)(tenant_id.clone()).await
    }
}

/// A resolver that looks up tenants from the database.
pub struct DatabaseResolver<F>
where
    F: Fn(String) -> Pin<Box<dyn Future<Output = QueryResult<Option<TenantInfo>>> + Send>>
        + Send
        + Sync,
{
    query_fn: F,
    cache: Arc<RwLock<HashMap<String, TenantContext>>>,
    cache_ttl: std::time::Duration,
}

impl<F> DatabaseResolver<F>
where
    F: Fn(String) -> Pin<Box<dyn Future<Output = QueryResult<Option<TenantInfo>>> + Send>>
        + Send
        + Sync,
{
    /// Create a new database resolver.
    pub fn new(query_fn: F) -> Self {
        Self {
            query_fn,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: std::time::Duration::from_secs(300), // 5 minutes
        }
    }

    /// Set the cache TTL.
    pub fn with_cache_ttl(mut self, ttl: std::time::Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Clear the cache.
    pub fn clear_cache(&self) {
        self.cache.write().expect("lock poisoned").clear();
    }

    /// Invalidate a specific tenant in the cache.
    pub fn invalidate(&self, tenant_id: &str) {
        self.cache.write().expect("lock poisoned").remove(tenant_id);
    }
}

impl<F> std::fmt::Debug for DatabaseResolver<F>
where
    F: Fn(String) -> Pin<Box<dyn Future<Output = QueryResult<Option<TenantInfo>>> + Send>>
        + Send
        + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseResolver")
            .field("cache_ttl", &self.cache_ttl)
            .field("cache_size", &self.cache.read().expect("lock").len())
            .finish()
    }
}

#[async_trait]
impl<F> TenantResolver for DatabaseResolver<F>
where
    F: Fn(String) -> Pin<Box<dyn Future<Output = QueryResult<Option<TenantInfo>>> + Send>>
        + Send
        + Sync,
{
    async fn resolve(&self, tenant_id: &TenantId) -> QueryResult<TenantContext> {
        // Check cache first
        if let Some(ctx) = self
            .cache
            .read()
            .expect("lock poisoned")
            .get(tenant_id.as_str())
        {
            return Ok(ctx.clone());
        }

        // Query database
        let info = (self.query_fn)(tenant_id.as_str().to_string())
            .await?
            .ok_or_else(|| crate::error::QueryError::not_found(format!("Tenant {}", tenant_id)))?;

        let ctx = TenantContext::with_info(tenant_id.clone(), info);

        // Cache the result
        self.cache
            .write()
            .expect("lock poisoned")
            .insert(tenant_id.as_str().to_string(), ctx.clone());

        Ok(ctx)
    }
}

/// A composite resolver that tries multiple resolvers in order.
pub struct CompositeResolver {
    resolvers: Vec<Arc<dyn TenantResolver>>,
}

impl CompositeResolver {
    /// Create a new composite resolver.
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    /// Add a resolver to the chain.
    pub fn add<R: TenantResolver + 'static>(mut self, resolver: R) -> Self {
        self.resolvers.push(Arc::new(resolver));
        self
    }
}

impl Default for CompositeResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CompositeResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeResolver")
            .field("resolver_count", &self.resolvers.len())
            .finish()
    }
}

#[async_trait]
impl TenantResolver for CompositeResolver {
    async fn resolve(&self, tenant_id: &TenantId) -> QueryResult<TenantContext> {
        for resolver in &self.resolvers {
            if let Ok(ctx) = resolver.resolve(tenant_id).await {
                return Ok(ctx);
            }
        }
        Err(crate::error::QueryError::not_found(format!(
            "Tenant {} not found in any resolver",
            tenant_id
        )))
    }

    async fn validate(&self, tenant_id: &TenantId) -> QueryResult<bool> {
        for resolver in &self.resolvers {
            if resolver.validate(tenant_id).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_static_resolver() {
        let resolver = StaticResolver::new();
        resolver.register_simple("tenant-1");
        resolver.register(
            "tenant-2",
            TenantContext::with_info(
                "tenant-2",
                TenantInfo::new()
                    .with_name("Acme Corp")
                    .with_schema("tenant_acme"),
            ),
        );

        let ctx1 = resolver.resolve(&TenantId::new("tenant-1")).await.unwrap();
        assert_eq!(ctx1.id.as_str(), "tenant-1");

        let ctx2 = resolver.resolve(&TenantId::new("tenant-2")).await.unwrap();
        assert_eq!(ctx2.info.name, Some("Acme Corp".to_string()));
        assert_eq!(ctx2.info.schema, Some("tenant_acme".to_string()));

        assert!(resolver.validate(&TenantId::new("tenant-1")).await.unwrap());
        assert!(!resolver.validate(&TenantId::new("unknown")).await.unwrap());
    }

    #[tokio::test]
    async fn test_dynamic_resolver() {
        let resolver = DynamicResolver::new(|id| async move {
            if id.as_str() == "valid" {
                Ok(TenantContext::new(id))
            } else {
                Err(crate::error::QueryError::not_found("Tenant"))
            }
        });

        assert!(resolver.resolve(&TenantId::new("valid")).await.is_ok());
        assert!(resolver.resolve(&TenantId::new("invalid")).await.is_err());
    }

    #[tokio::test]
    async fn test_composite_resolver() {
        let static1 = StaticResolver::new();
        static1.register_simple("tenant-a");

        let static2 = StaticResolver::new();
        static2.register_simple("tenant-b");

        let resolver = CompositeResolver::new().add(static1).add(static2);

        assert!(resolver.resolve(&TenantId::new("tenant-a")).await.is_ok());
        assert!(resolver.resolve(&TenantId::new("tenant-b")).await.is_ok());
        assert!(resolver.resolve(&TenantId::new("tenant-c")).await.is_err());
    }
}
