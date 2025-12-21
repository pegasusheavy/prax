//! Tenant context for tracking the current tenant.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// A unique identifier for a tenant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantId(String);

impl TenantId {
    /// Create a new tenant ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the tenant ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TenantId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for TenantId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<uuid::Uuid> for TenantId {
    fn from(u: uuid::Uuid) -> Self {
        Self::new(u.to_string())
    }
}

impl From<i64> for TenantId {
    fn from(i: i64) -> Self {
        Self::new(i.to_string())
    }
}

impl From<i32> for TenantId {
    fn from(i: i32) -> Self {
        Self::new(i.to_string())
    }
}

/// Additional information about a tenant.
#[derive(Debug, Clone, Default)]
pub struct TenantInfo {
    /// Display name for the tenant.
    pub name: Option<String>,
    /// Schema name (for schema-based isolation).
    pub schema: Option<String>,
    /// Database name (for database-based isolation).
    pub database: Option<String>,
    /// Whether this tenant has superuser privileges (bypasses filters).
    pub is_superuser: bool,
    /// Whether this is the system/default tenant.
    pub is_system: bool,
    /// Custom metadata.
    metadata: HashMap<String, Arc<dyn Any + Send + Sync>>,
}

impl TenantInfo {
    /// Create a new tenant info.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the tenant name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the schema name.
    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    /// Set the database name.
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Mark as superuser.
    pub fn as_superuser(mut self) -> Self {
        self.is_superuser = true;
        self
    }

    /// Mark as system tenant.
    pub fn as_system(mut self) -> Self {
        self.is_system = true;
        self
    }

    /// Add custom metadata.
    pub fn with_metadata<T: Any + Send + Sync>(mut self, key: impl Into<String>, value: T) -> Self {
        self.metadata.insert(key.into(), Arc::new(value));
        self
    }

    /// Get custom metadata.
    pub fn get_metadata<T: Any + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.metadata.get(key).and_then(|v| v.downcast_ref())
    }
}

/// Context for the current tenant.
///
/// This context is passed through the query pipeline to ensure all operations
/// are scoped to the correct tenant.
#[derive(Debug, Clone)]
pub struct TenantContext {
    /// The tenant identifier.
    pub id: TenantId,
    /// Additional tenant information.
    pub info: TenantInfo,
}

impl TenantContext {
    /// Create a new tenant context with just an ID.
    pub fn new(id: impl Into<TenantId>) -> Self {
        Self {
            id: id.into(),
            info: TenantInfo::default(),
        }
    }

    /// Create a tenant context with additional info.
    pub fn with_info(id: impl Into<TenantId>, info: TenantInfo) -> Self {
        Self {
            id: id.into(),
            info,
        }
    }

    /// Create a system/superuser context that bypasses tenant filters.
    pub fn system() -> Self {
        Self {
            id: TenantId::new("__system__"),
            info: TenantInfo::new().as_system().as_superuser(),
        }
    }

    /// Check if this context should bypass tenant filters.
    pub fn should_bypass(&self) -> bool {
        self.info.is_superuser || self.info.is_system
    }

    /// Get the schema for this tenant (schema-based isolation).
    pub fn schema(&self) -> Option<&str> {
        self.info.schema.as_deref()
    }

    /// Get the database for this tenant (database-based isolation).
    pub fn database(&self) -> Option<&str> {
        self.info.database.as_deref()
    }
}

/// Thread-local tenant context storage.
#[cfg(feature = "thread-local-tenant")]
mod thread_local {
    use super::TenantContext;
    use std::cell::RefCell;

    thread_local! {
        static CURRENT_TENANT: RefCell<Option<TenantContext>> = const { RefCell::new(None) };
    }

    /// Set the current tenant context for this thread.
    pub fn set_current_tenant(ctx: TenantContext) {
        CURRENT_TENANT.with(|t| {
            *t.borrow_mut() = Some(ctx);
        });
    }

    /// Get the current tenant context for this thread.
    pub fn get_current_tenant() -> Option<TenantContext> {
        CURRENT_TENANT.with(|t| t.borrow().clone())
    }

    /// Clear the current tenant context.
    pub fn clear_current_tenant() {
        CURRENT_TENANT.with(|t| {
            *t.borrow_mut() = None;
        });
    }

    /// Execute a closure with a specific tenant context.
    pub fn with_tenant<F, R>(ctx: TenantContext, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let previous = get_current_tenant();
        set_current_tenant(ctx);
        let result = f();
        if let Some(prev) = previous {
            set_current_tenant(prev);
        } else {
            clear_current_tenant();
        }
        result
    }
}

#[cfg(feature = "thread-local-tenant")]
pub use thread_local::{clear_current_tenant, get_current_tenant, set_current_tenant, with_tenant};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_id_creation() {
        let id1 = TenantId::new("tenant-123");
        assert_eq!(id1.as_str(), "tenant-123");

        let id2: TenantId = "tenant-456".into();
        assert_eq!(id2.as_str(), "tenant-456");

        let id3: TenantId = 123_i64.into();
        assert_eq!(id3.as_str(), "123");
    }

    #[test]
    fn test_tenant_context() {
        let ctx = TenantContext::new("tenant-123");
        assert_eq!(ctx.id.as_str(), "tenant-123");
        assert!(!ctx.should_bypass());
    }

    #[test]
    fn test_system_context() {
        let ctx = TenantContext::system();
        assert!(ctx.should_bypass());
        assert!(ctx.info.is_system);
        assert!(ctx.info.is_superuser);
    }

    #[test]
    fn test_tenant_info() {
        let info = TenantInfo::new()
            .with_name("Acme Corp")
            .with_schema("tenant_acme")
            .with_metadata("plan", "enterprise".to_string());

        assert_eq!(info.name, Some("Acme Corp".to_string()));
        assert_eq!(info.schema, Some("tenant_acme".to_string()));
        assert_eq!(
            info.get_metadata::<String>("plan"),
            Some(&"enterprise".to_string())
        );
    }
}
