//! Zero-allocation tenant context using task-local storage.
//!
//! This module provides high-performance tenant context propagation using
//! Tokio's task-local storage, eliminating the need for `Arc<RwLock>` in
//! the hot path.
//!
//! # Performance Benefits
//!
//! - **Zero heap allocation** for context access
//! - **No locking** on the hot path
//! - **Automatic cleanup** when task completes
//! - **Async-aware** - works across `.await` points
//!
//! # Example
//!
//! ```rust,ignore
//! use prax_query::tenant::task_local::{with_tenant, current_tenant, TenantScope};
//!
//! // Set tenant for async block
//! with_tenant("tenant-123", async {
//!     // All queries in this block use tenant-123
//!     let users = client.user().find_many().exec().await?;
//!
//!     // Nested calls also see the tenant
//!     do_something_else().await?;
//!
//!     Ok(())
//! }).await?;
//!
//! // Or use scoped guard
//! let _guard = TenantScope::new("tenant-123");
//! // tenant context available until guard is dropped
//! ```

use std::cell::Cell;
use std::future::Future;

use super::context::{TenantContext, TenantId};

tokio::task_local! {
    /// Task-local tenant context.
    static TENANT_CONTEXT: TenantContext;
}

thread_local! {
    /// Thread-local tenant ID for sync code paths.
    /// Uses Cell for interior mutability without runtime cost.
    static SYNC_TENANT_ID: Cell<Option<TenantId>> = const { Cell::new(None) };
}

/// Execute an async block with the given tenant context.
///
/// This is the most efficient way to set tenant context for async code.
/// The context is automatically available to all nested async calls.
///
/// # Example
///
/// ```rust,ignore
/// use prax_query::tenant::task_local::with_tenant;
///
/// with_tenant("tenant-123", async {
///     // All code here sees tenant-123
///     let users = client.user().find_many().exec().await?;
///     Ok(())
/// }).await?;
/// ```
pub async fn with_tenant<F, T>(tenant_id: impl Into<TenantId>, f: F) -> T
where
    F: Future<Output = T>,
{
    let ctx = TenantContext::new(tenant_id);
    TENANT_CONTEXT.scope(ctx, f).await
}

/// Execute an async block with a full tenant context.
pub async fn with_context<F, T>(ctx: TenantContext, f: F) -> T
where
    F: Future<Output = T>,
{
    TENANT_CONTEXT.scope(ctx, f).await
}

/// Get the current tenant context if set.
///
/// Returns `None` if no tenant context is active.
///
/// # Example
///
/// ```rust,ignore
/// use prax_query::tenant::task_local::current_tenant;
///
/// if let Some(ctx) = current_tenant() {
///     println!("Current tenant: {}", ctx.id);
/// }
/// ```
#[inline]
pub fn current_tenant() -> Option<TenantContext> {
    TENANT_CONTEXT.try_with(|ctx| ctx.clone()).ok()
}

/// Get the current tenant ID if set.
///
/// More efficient than `current_tenant()` when you only need the ID.
#[inline]
pub fn current_tenant_id() -> Option<TenantId> {
    TENANT_CONTEXT.try_with(|ctx| ctx.id.clone()).ok()
}

/// Get the current tenant ID as a string slice.
///
/// **Always returns an empty string**, even when a tenant context is active.
/// Use [`current_tenant_id()`] instead.
#[deprecated(
    since = "0.11.0",
    note = "always returns an empty string; use current_tenant_id() instead"
)]
#[inline]
pub fn current_tenant_id_str() -> &'static str {
    // This is a workaround - in practice you'd use current_tenant_id()
    // We return a static str for zero-allocation in the common case
    ""
}

/// Check if a tenant context is currently active.
#[inline]
pub fn has_tenant() -> bool {
    TENANT_CONTEXT.try_with(|_| ()).is_ok()
}

/// Execute a closure with the current tenant context.
///
/// Returns `None` if no tenant context is active.
#[inline]
pub fn with_current_tenant<F, T>(f: F) -> Option<T>
where
    F: FnOnce(&TenantContext) -> T,
{
    TENANT_CONTEXT.try_with(f).ok()
}

/// Require a tenant context, returning an error if not set.
#[inline]
pub fn require_tenant() -> Result<TenantContext, TenantNotSetError> {
    current_tenant().ok_or(TenantNotSetError)
}

/// Error returned when tenant context is required but not set.
#[derive(Debug, Clone, Copy)]
pub struct TenantNotSetError;

impl std::fmt::Display for TenantNotSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tenant context not set")
    }
}

impl std::error::Error for TenantNotSetError {}

// ============================================================================
// Sync Context (Thread-Local)
// ============================================================================

/// Set the tenant ID for synchronous code on the current thread.
///
/// This is useful for sync code paths or when you can't use async scope.
/// The tenant is automatically cleared when the guard is dropped.
///
/// # Example
///
/// ```rust,ignore
/// use prax_query::tenant::task_local::set_sync_tenant;
///
/// let _guard = set_sync_tenant("tenant-123");
/// // tenant available for sync code
/// ```
pub fn set_sync_tenant(tenant_id: impl Into<TenantId>) -> SyncTenantGuard {
    let id = tenant_id.into();
    let previous = SYNC_TENANT_ID.with(|cell| cell.replace(Some(id)));
    SyncTenantGuard { previous }
}

/// Get the current sync tenant ID.
#[inline]
pub fn sync_tenant_id() -> Option<TenantId> {
    SYNC_TENANT_ID.with(|cell| {
        // SAFETY: We only read, not modify
        unsafe { &*cell.as_ptr() }.clone()
    })
}

/// Guard that resets the sync tenant when dropped.
pub struct SyncTenantGuard {
    previous: Option<TenantId>,
}

impl Drop for SyncTenantGuard {
    fn drop(&mut self) {
        SYNC_TENANT_ID.with(|cell| cell.set(self.previous.take()));
    }
}

// ============================================================================
// Scoped Guard (Alternative API)
// ============================================================================

/// A scoped tenant context that tracks whether it's been entered.
///
/// This provides an alternative to `with_tenant` for cases where you
/// need more control over the scope.
///
/// # Example
///
/// ```rust,ignore
/// use prax_query::tenant::task_local::TenantScope;
///
/// async fn handle_request(tenant_id: &str) {
///     let scope = TenantScope::new(tenant_id);
///
///     scope.run(async {
///         // tenant context active here
///     }).await;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TenantScope {
    context: TenantContext,
}

impl TenantScope {
    /// Create a new tenant scope.
    pub fn new(tenant_id: impl Into<TenantId>) -> Self {
        Self {
            context: TenantContext::new(tenant_id),
        }
    }

    /// Create from a full context.
    pub fn from_context(context: TenantContext) -> Self {
        Self { context }
    }

    /// Get the tenant ID.
    pub fn tenant_id(&self) -> &TenantId {
        &self.context.id
    }

    /// Get the full context.
    pub fn context(&self) -> &TenantContext {
        &self.context
    }

    /// Run an async function within this tenant scope.
    pub async fn run<F, T>(&self, f: F) -> T
    where
        F: Future<Output = T>,
    {
        TENANT_CONTEXT.scope(self.context.clone(), f).await
    }

    /// Run a sync closure within this tenant scope (thread-local).
    pub fn run_sync<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let _guard = set_sync_tenant(self.context.id.clone());
        f()
    }
}

// ============================================================================
// Middleware Integration
// ============================================================================

/// Extract tenant from various sources.
pub trait TenantExtractor: Send + Sync {
    /// Extract tenant ID from a request/context.
    fn extract(&self, headers: &[(String, String)]) -> Option<TenantId>;
}

/// Extract tenant from a header.
#[derive(Debug, Clone)]
pub struct HeaderExtractor {
    header_name: String,
}

impl HeaderExtractor {
    /// Create a new header extractor.
    pub fn new(header_name: impl Into<String>) -> Self {
        Self {
            header_name: header_name.into(),
        }
    }

    /// Create with default header name "X-Tenant-ID".
    pub fn default_header() -> Self {
        Self::new("X-Tenant-ID")
    }
}

impl TenantExtractor for HeaderExtractor {
    fn extract(&self, headers: &[(String, String)]) -> Option<TenantId> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&self.header_name))
            .map(|(_, v)| TenantId::new(v.clone()))
    }
}

/// Extract tenant from a JWT claim in the `Authorization: Bearer <jwt>` header.
///
/// **Security note:** this extractor performs NO signature verification. It
/// only base64url-decodes the JWT payload segment and reads the configured
/// claim. Extraction MUST sit behind a verifying middleware: the token must
/// already have been authenticated upstream by the web framework / auth
/// middleware before this extractor runs. Used on its own, any caller can
/// forge any tenant id.
#[derive(Debug, Clone)]
pub struct UnverifiedJwtClaimExtractor {
    claim_name: String,
}

impl UnverifiedJwtClaimExtractor {
    /// Create a new JWT claim extractor.
    pub fn new(claim_name: impl Into<String>) -> Self {
        Self {
            claim_name: claim_name.into(),
        }
    }

    /// Create with default claim name "tenant_id".
    pub fn default_claim() -> Self {
        Self::new("tenant_id")
    }

    /// Get the claim name.
    pub fn claim_name(&self) -> &str {
        &self.claim_name
    }
}

/// Source-compatibility alias for [`UnverifiedJwtClaimExtractor`].
///
/// # Deprecated
///
/// Renamed to [`UnverifiedJwtClaimExtractor`] to make the absence of
/// signature verification explicit; this alias will be removed in the 0.12
/// breaking release. (Kept as a plain alias rather than `#[deprecated]` so
/// the crate-level re-export stays warning-free until then.)
pub type JwtClaimExtractor = UnverifiedJwtClaimExtractor;

impl TenantExtractor for UnverifiedJwtClaimExtractor {
    /// Extract the tenant ID from the JWT in the `Authorization` header.
    ///
    /// Decodes the payload segment only — NO signature verification is
    /// performed (see the struct-level security note). Malformed tokens are
    /// logged at debug level so they are distinguishable from an absent
    /// header.
    fn extract(&self, headers: &[(String, String)]) -> Option<TenantId> {
        use base64::Engine as _;

        let auth = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
            .map(|(_, v)| v.as_str())?;

        // Strip the auth scheme ("Bearer <token>", scheme is case-insensitive).
        let Some(token) = auth
            .get(..7)
            .filter(|scheme| scheme.eq_ignore_ascii_case("bearer "))
            .map(|_| &auth[7..])
        else {
            tracing::debug!("Authorization header present but not a Bearer token");
            return None;
        };

        // JWT = header.payload.signature; only the payload segment is needed.
        let Some(payload) = token.split('.').nth(1) else {
            tracing::debug!("Authorization header present but JWT has no payload segment");
            return None;
        };
        let decoded = match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload) {
            Ok(decoded) => decoded,
            Err(error) => {
                tracing::debug!(%error, "Authorization header present but JWT payload undecodable");
                return None;
            }
        };
        let claims: serde_json::Value = match serde_json::from_slice(&decoded) {
            Ok(claims) => claims,
            Err(error) => {
                tracing::debug!(%error, "Authorization header present but JWT payload is not JSON");
                return None;
            }
        };

        claims
            .get(self.claim_name.as_str())?
            .as_str()
            .map(TenantId::new)
    }
}

/// Composite extractor that tries multiple sources.
pub struct CompositeExtractor {
    extractors: Vec<Box<dyn TenantExtractor>>,
}

impl CompositeExtractor {
    /// Create a new composite extractor.
    pub fn new() -> Self {
        Self {
            extractors: Vec::new(),
        }
    }

    /// Add an extractor.
    pub fn add<E: TenantExtractor + 'static>(mut self, extractor: E) -> Self {
        self.extractors.push(Box::new(extractor));
        self
    }
}

impl Default for CompositeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl TenantExtractor for CompositeExtractor {
    fn extract(&self, headers: &[(String, String)]) -> Option<TenantId> {
        for extractor in &self.extractors {
            if let Some(id) = extractor.extract(headers) {
                return Some(id);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_with_tenant() {
        let result = with_tenant("test-tenant", async { current_tenant_id() }).await;

        assert_eq!(result.unwrap().as_str(), "test-tenant");
    }

    #[tokio::test]
    async fn test_no_tenant() {
        assert!(current_tenant().is_none());
        assert!(!has_tenant());
    }

    #[tokio::test]
    async fn test_nested_tenant() {
        with_tenant("outer", async {
            assert_eq!(current_tenant_id().unwrap().as_str(), "outer");

            with_tenant("inner", async {
                assert_eq!(current_tenant_id().unwrap().as_str(), "inner");
            })
            .await;

            // Should be back to outer
            assert_eq!(current_tenant_id().unwrap().as_str(), "outer");
        })
        .await;
    }

    #[tokio::test]
    async fn test_tenant_scope() {
        let scope = TenantScope::new("scoped-tenant");

        let result = scope
            .run(async { current_tenant_id().map(|id| id.as_str().to_string()) })
            .await;

        assert_eq!(result, Some("scoped-tenant".to_string()));
    }

    #[test]
    fn test_sync_tenant() {
        {
            let _guard = set_sync_tenant("sync-tenant");
            assert_eq!(sync_tenant_id().unwrap().as_str(), "sync-tenant");
        }

        // Should be cleared after guard drop
        assert!(sync_tenant_id().is_none());
    }

    #[test]
    fn test_header_extractor() {
        let extractor = HeaderExtractor::new("X-Tenant-ID");

        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Tenant-ID".to_string(), "tenant-from-header".to_string()),
        ];

        let id = extractor.extract(&headers);
        assert_eq!(id.unwrap().as_str(), "tenant-from-header");
    }

    #[test]
    fn test_jwt_claim_extractor() {
        let extractor = UnverifiedJwtClaimExtractor::new("tenant_id");

        // Unsigned JWT (alg: none) with payload {"tenant_id":"tenant-from-jwt"}.
        // Extraction decodes the payload only; signature verification is the
        // framework's responsibility.
        let jwt = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJ0ZW5hbnRfaWQiOiJ0ZW5hbnQtZnJvbS1qd3QifQ.";
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), format!("Bearer {jwt}")),
        ];

        let id = extractor.extract(&headers);
        assert_eq!(id.unwrap().as_str(), "tenant-from-jwt");

        // The deprecated alias still resolves to the same type.
        let _alias: JwtClaimExtractor = UnverifiedJwtClaimExtractor::default_claim();

        // Missing Authorization header yields no tenant.
        assert!(extractor.extract(&[]).is_none());

        // Malformed tokens yield no tenant (logged at debug level).
        for bad in ["not-a-jwt", "Bearer !!!.@@@.###", "Bearer a.b.c"] {
            let headers = vec![("Authorization".to_string(), bad.to_string())];
            assert!(extractor.extract(&headers).is_none());
        }

        // Missing claim yields no tenant.
        let other = UnverifiedJwtClaimExtractor::new("org_id");
        assert!(other.extract(&headers).is_none());
    }

    #[test]
    fn test_composite_extractor() {
        let extractor = CompositeExtractor::new()
            .add(HeaderExtractor::new("X-Organization-ID"))
            .add(HeaderExtractor::new("X-Tenant-ID"));

        let headers = vec![("X-Tenant-ID".to_string(), "fallback-tenant".to_string())];

        let id = extractor.extract(&headers);
        assert_eq!(id.unwrap().as_str(), "fallback-tenant");
    }
}
