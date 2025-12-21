//! Schema caching for improved performance.
//!
//! This module provides:
//! - Schema caching to avoid re-parsing
//! - String interning for documentation strings
//! - Lazy computed fields
//!
//! # Examples
//!
//! ```rust
//! use prax_schema::cache::{SchemaCache, DocString};
//!
//! // Cache parsed schemas
//! let mut cache = SchemaCache::new();
//!
//! let schema = cache.get_or_parse("model User { id Int @id }").unwrap();
//! let schema2 = cache.get_or_parse("model User { id Int @id }").unwrap();
//! // schema2 is the same Arc as schema (cached)
//!
//! // Intern documentation strings
//! let doc1 = DocString::intern("User profile information");
//! let doc2 = DocString::intern("User profile information");
//! // doc1 and doc2 share the same allocation
//! ```

use parking_lot::RwLock;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::ast::Schema;
use crate::error::SchemaResult;
use crate::parser::parse_schema;

// ============================================================================
// Schema Cache
// ============================================================================

/// A cache for parsed schemas.
///
/// Caches parsed schemas by their source text hash to avoid re-parsing
/// identical schemas.
#[derive(Debug, Default)]
pub struct SchemaCache {
    cache: RwLock<HashMap<u64, Arc<Schema>>>,
    stats: RwLock<CacheStats>,
}

/// Statistics for the schema cache.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of schemas currently cached.
    pub cached_count: usize,
}

impl CacheStats {
    /// Get the cache hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl SchemaCache {
    /// Create a new empty schema cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cache with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::with_capacity(capacity)),
            stats: RwLock::default(),
        }
    }

    /// Get a cached schema or parse and cache a new one.
    ///
    /// Returns an `Arc<Schema>` which can be cloned cheaply.
    pub fn get_or_parse(&self, source: &str) -> SchemaResult<Arc<Schema>> {
        let hash = hash_source(source);

        // Try to get from cache first
        {
            let cache = self.cache.read();
            if let Some(schema) = cache.get(&hash) {
                self.stats.write().hits += 1;
                return Ok(Arc::clone(schema));
            }
        }

        // Parse and cache
        let schema = parse_schema(source)?;
        let schema = Arc::new(schema);

        {
            let mut cache = self.cache.write();
            cache.insert(hash, Arc::clone(&schema));
        }

        {
            let mut stats = self.stats.write();
            stats.misses += 1;
            stats.cached_count = self.cache.read().len();
        }

        Ok(schema)
    }

    /// Check if a schema is cached.
    pub fn contains(&self, source: &str) -> bool {
        let hash = hash_source(source);
        self.cache.read().contains_key(&hash)
    }

    /// Clear the cache.
    pub fn clear(&self) {
        self.cache.write().clear();
        self.stats.write().cached_count = 0;
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let mut stats = self.stats.read().clone();
        stats.cached_count = self.cache.read().len();
        stats
    }

    /// Get the number of cached schemas.
    pub fn len(&self) -> usize {
        self.cache.read().len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.read().is_empty()
    }

    /// Remove a specific schema from the cache.
    pub fn remove(&self, source: &str) -> bool {
        let hash = hash_source(source);
        self.cache.write().remove(&hash).is_some()
    }
}

/// Hash a source string for caching.
fn hash_source(source: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

// ============================================================================
// Documentation String Interning
// ============================================================================

/// An interned documentation string.
///
/// Uses `Arc<str>` for efficient sharing of identical documentation strings.
/// Documentation comments are often duplicated across models (e.g., "id" field docs).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocString(Arc<str>);

impl DocString {
    /// Create a new documentation string (not interned).
    pub fn new(s: impl AsRef<str>) -> Self {
        Self(Arc::from(s.as_ref()))
    }

    /// Intern a documentation string.
    ///
    /// Returns a shared reference to the string if it's already interned,
    /// or interns and returns a new reference.
    pub fn intern(s: impl AsRef<str>) -> Self {
        DOC_INTERNER.intern(s.as_ref())
    }

    /// Get the string as a slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the underlying Arc.
    pub fn as_arc(&self) -> &Arc<str> {
        &self.0
    }
}

impl AsRef<str> for DocString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DocString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for DocString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for DocString {
    fn from(s: String) -> Self {
        Self(Arc::from(s))
    }
}

/// Global documentation string interner.
static DOC_INTERNER: std::sync::LazyLock<DocInterner> = std::sync::LazyLock::new(DocInterner::new);

/// Interner for documentation strings.
#[derive(Debug, Default)]
struct DocInterner {
    strings: RwLock<HashMap<u64, Arc<str>>>,
}

impl DocInterner {
    fn new() -> Self {
        Self::default()
    }

    fn intern(&self, s: &str) -> DocString {
        let hash = hash_source(s);

        // Check if already interned
        {
            let strings = self.strings.read();
            if let Some(arc) = strings.get(&hash) {
                return DocString(Arc::clone(arc));
            }
        }

        // Intern the string
        let arc: Arc<str> = Arc::from(s);
        {
            let mut strings = self.strings.write();
            strings.insert(hash, Arc::clone(&arc));
        }

        DocString(arc)
    }
}

// ============================================================================
// Lazy Field Attributes
// ============================================================================

/// Lazily computed field attributes.
///
/// Caches expensive attribute extraction to avoid repeated computation.
#[derive(Debug, Clone, Default)]
pub struct LazyFieldAttrs {
    computed: std::sync::OnceLock<FieldAttrsCache>,
}

/// Cached field attribute values.
#[derive(Debug, Clone, Default)]
pub struct FieldAttrsCache {
    /// Is this an ID field?
    pub is_id: bool,
    /// Is this an auto-generated field?
    pub is_auto: bool,
    /// Is this a unique field?
    pub is_unique: bool,
    /// Is this an indexed field?
    pub is_indexed: bool,
    /// Is this an updated_at field?
    pub is_updated_at: bool,
    /// Default value expression (if any).
    pub default_value: Option<String>,
    /// Mapped column name (if different from field name).
    pub mapped_name: Option<String>,
}

impl LazyFieldAttrs {
    /// Create new lazy field attributes.
    pub const fn new() -> Self {
        Self {
            computed: std::sync::OnceLock::new(),
        }
    }

    /// Get or compute the cached attributes.
    pub fn get_or_init<F>(&self, f: F) -> &FieldAttrsCache
    where
        F: FnOnce() -> FieldAttrsCache,
    {
        self.computed.get_or_init(f)
    }

    /// Check if attributes have been computed.
    pub fn is_computed(&self) -> bool {
        self.computed.get().is_some()
    }

    /// Clear the cached attributes (creates a new instance).
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// ============================================================================
// Optimized Validation Type Pool
// ============================================================================

/// Pool of commonly used validation types.
///
/// Pre-allocates common validation type combinations to avoid repeated
/// allocation during schema parsing.
#[derive(Debug, Default)]
pub struct ValidationTypePool {
    /// String validation (email, url, etc.)
    pub string_validators: HashMap<&'static str, Arc<ValidatorDef>>,
    /// Numeric validation (min, max, etc.)
    pub numeric_validators: HashMap<&'static str, Arc<ValidatorDef>>,
}

/// A cached validator definition.
#[derive(Debug, Clone)]
pub struct ValidatorDef {
    /// Validator name.
    pub name: &'static str,
    /// Validator type.
    pub validator_type: ValidatorType,
}

/// Type of validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidatorType {
    /// String format validator (email, url, uuid, etc.)
    StringFormat,
    /// String length validator.
    StringLength,
    /// Numeric range validator.
    NumericRange,
    /// Regex pattern validator.
    Pattern,
    /// Custom validator.
    Custom,
}

impl ValidationTypePool {
    /// Create a new pool with common validators pre-allocated.
    pub fn new() -> Self {
        let mut pool = Self::default();
        pool.init_common_validators();
        pool
    }

    fn init_common_validators(&mut self) {
        // Common string format validators
        let string_formats = [
            "email", "url", "uuid", "cuid", "cuid2", "nanoid", "ulid", "ipv4", "ipv6", "date",
            "datetime", "time",
        ];

        for name in string_formats {
            self.string_validators.insert(
                name,
                Arc::new(ValidatorDef {
                    name,
                    validator_type: ValidatorType::StringFormat,
                }),
            );
        }

        // Common numeric validators
        let numeric_validators = [
            "min",
            "max",
            "positive",
            "negative",
            "nonNegative",
            "nonPositive",
        ];
        for name in numeric_validators {
            self.numeric_validators.insert(
                name,
                Arc::new(ValidatorDef {
                    name,
                    validator_type: ValidatorType::NumericRange,
                }),
            );
        }
    }

    /// Get a string format validator.
    pub fn get_string_validator(&self, name: &str) -> Option<&Arc<ValidatorDef>> {
        self.string_validators.get(name)
    }

    /// Get a numeric validator.
    pub fn get_numeric_validator(&self, name: &str) -> Option<&Arc<ValidatorDef>> {
        self.numeric_validators.get(name)
    }
}

/// Global validation type pool.
pub static VALIDATION_POOL: std::sync::LazyLock<ValidationTypePool> =
    std::sync::LazyLock::new(ValidationTypePool::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_cache_hit() {
        let cache = SchemaCache::new();

        let schema1 = cache.get_or_parse("model User { id Int @id }").unwrap();
        let schema2 = cache.get_or_parse("model User { id Int @id }").unwrap();

        // Should be the same Arc
        assert!(Arc::ptr_eq(&schema1, &schema2));

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_schema_cache_miss() {
        let cache = SchemaCache::new();

        let _ = cache.get_or_parse("model User { id Int @id }").unwrap();
        let _ = cache.get_or_parse("model Post { id Int @id }").unwrap();

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 2);
    }

    #[test]
    fn test_schema_cache_clear() {
        let cache = SchemaCache::new();

        let _ = cache.get_or_parse("model User { id Int @id }").unwrap();
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_doc_string_interning() {
        let doc1 = DocString::intern("User profile information");
        let doc2 = DocString::intern("User profile information");

        // Should share the same Arc
        assert!(Arc::ptr_eq(doc1.as_arc(), doc2.as_arc()));
    }

    #[test]
    fn test_doc_string_different() {
        let doc1 = DocString::intern("User profile");
        let doc2 = DocString::intern("Post content");

        assert_ne!(doc1.as_str(), doc2.as_str());
    }

    #[test]
    fn test_lazy_field_attrs() {
        let lazy = LazyFieldAttrs::new();

        assert!(!lazy.is_computed());

        let attrs = lazy.get_or_init(|| FieldAttrsCache {
            is_id: true,
            is_auto: true,
            ..Default::default()
        });

        assert!(lazy.is_computed());
        assert!(attrs.is_id);
        assert!(attrs.is_auto);
    }

    #[test]
    fn test_validation_pool() {
        let pool = ValidationTypePool::new();

        assert!(pool.get_string_validator("email").is_some());
        assert!(pool.get_string_validator("url").is_some());
        assert!(pool.get_numeric_validator("min").is_some());
        assert!(pool.get_numeric_validator("max").is_some());
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats {
            hits: 8,
            misses: 2,
            cached_count: 5,
        };

        assert!((stats.hit_rate() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_cache_stats_zero() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
    }
}
