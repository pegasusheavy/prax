//! String interning for efficient field name storage.
//!
//! This module provides string interning to reduce memory allocations when the same
//! field names are used across many filters. Interned strings share memory, making
//! cloning essentially free.
//!
//! # When to Use
//!
//! String interning is most beneficial when:
//! - The same field names are used in many filters
//! - Field names come from dynamic sources (not `&'static str`)
//! - You're building complex query trees with repeated field references
//!
//! For static field names (`&'static str`), use them directly - they're already
//! "interned" by the compiler with zero overhead.
//!
//! # Examples
//!
//! ## Using Pre-defined Field Names
//!
//! ```rust
//! use prax_query::intern::fields;
//! use prax_query::{Filter, FilterValue};
//!
//! // Common field names are pre-defined as constants
//! let filter = Filter::Equals(fields::ID.into(), FilterValue::Int(42));
//! let filter = Filter::Equals(fields::EMAIL.into(), FilterValue::String("test@example.com".into()));
//! let filter = Filter::Gt(fields::CREATED_AT.into(), FilterValue::String("2024-01-01".into()));
//! ```
//!
//! ## Interning Dynamic Strings
//!
//! ```rust
//! use prax_query::intern::{intern, intern_cow};
//! use prax_query::{Filter, FilterValue, FieldName};
//!
//! // Intern a dynamic string - subsequent calls return the same Arc<str>
//! let field1 = intern("dynamic_field");
//! let field2 = intern("dynamic_field");
//! // field1 and field2 point to the same memory
//!
//! // Use interned string directly in filters
//! let filter = Filter::Equals(intern_cow("user_id"), FilterValue::Int(1));
//! ```

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

/// Pre-defined common field name constants.
///
/// These are compile-time `&'static str` values that require zero allocation.
/// Use these when your field names match common database column names.
pub mod fields {
    /// Primary key field: "id"
    pub const ID: &str = "id";
    /// UUID field: "uuid"
    pub const UUID: &str = "uuid";
    /// Email field: "email"
    pub const EMAIL: &str = "email";
    /// Name field: "name"
    pub const NAME: &str = "name";
    /// Title field: "title"
    pub const TITLE: &str = "title";
    /// Description field: "description"
    pub const DESCRIPTION: &str = "description";
    /// Status field: "status"
    pub const STATUS: &str = "status";
    /// Active flag: "active"
    pub const ACTIVE: &str = "active";
    /// Enabled flag: "enabled"
    pub const ENABLED: &str = "enabled";
    /// Deleted flag: "deleted"
    pub const DELETED: &str = "deleted";
    /// Created timestamp: "created_at"
    pub const CREATED_AT: &str = "created_at";
    /// Updated timestamp: "updated_at"
    pub const UPDATED_AT: &str = "updated_at";
    /// Deleted timestamp: "deleted_at"
    pub const DELETED_AT: &str = "deleted_at";
    /// User ID foreign key: "user_id"
    pub const USER_ID: &str = "user_id";
    /// Author ID foreign key: "author_id"
    pub const AUTHOR_ID: &str = "author_id";
    /// Parent ID foreign key: "parent_id"
    pub const PARENT_ID: &str = "parent_id";
    /// Owner ID foreign key: "owner_id"
    pub const OWNER_ID: &str = "owner_id";
    /// Tenant ID for multi-tenancy: "tenant_id"
    pub const TENANT_ID: &str = "tenant_id";
    /// Organization ID: "org_id"
    pub const ORG_ID: &str = "org_id";
    /// Type discriminator: "type"
    pub const TYPE: &str = "type";
    /// Kind discriminator: "kind"
    pub const KIND: &str = "kind";
    /// Slug field: "slug"
    pub const SLUG: &str = "slug";
    /// Content field: "content"
    pub const CONTENT: &str = "content";
    /// Body field: "body"
    pub const BODY: &str = "body";
    /// Order/position field: "order"
    pub const ORDER: &str = "order";
    /// Position field: "position"
    pub const POSITION: &str = "position";
    /// Priority field: "priority"
    pub const PRIORITY: &str = "priority";
    /// Score field: "score"
    pub const SCORE: &str = "score";
    /// Count field: "count"
    pub const COUNT: &str = "count";
    /// Price field: "price"
    pub const PRICE: &str = "price";
    /// Amount field: "amount"
    pub const AMOUNT: &str = "amount";
    /// Quantity field: "quantity"
    pub const QUANTITY: &str = "quantity";
    /// Version field: "version"
    pub const VERSION: &str = "version";
    /// Age field: "age"
    pub const AGE: &str = "age";
    /// Role field: "role"
    pub const ROLE: &str = "role";
    /// Verified field: "verified"
    pub const VERIFIED: &str = "verified";
    /// Password field: "password"
    pub const PASSWORD: &str = "password";
    /// First name: "first_name"
    pub const FIRST_NAME: &str = "first_name";
    /// Last name: "last_name"
    pub const LAST_NAME: &str = "last_name";
    /// Category field: "category"
    pub const CATEGORY: &str = "category";
    /// Tags field: "tags"
    pub const TAGS: &str = "tags";
    /// Published flag: "published"
    pub const PUBLISHED: &str = "published";
    /// Published timestamp: "published_at"
    pub const PUBLISHED_AT: &str = "published_at";
    /// Expires timestamp: "expires_at"
    pub const EXPIRES_AT: &str = "expires_at";
    /// Started timestamp: "started_at"
    pub const STARTED_AT: &str = "started_at";
    /// Completed timestamp: "completed_at"
    pub const COMPLETED_AT: &str = "completed_at";
    /// Archived flag: "archived"
    pub const ARCHIVED: &str = "archived";
    /// Flagged flag: "flagged"
    pub const FLAGGED: &str = "flagged";
    /// Data field: "data"
    pub const DATA: &str = "data";
    /// Metadata field: "metadata"
    pub const METADATA: &str = "metadata";
    /// URL field: "url"
    pub const URL: &str = "url";
    /// Image URL: "image_url"
    pub const IMAGE_URL: &str = "image_url";
    /// Avatar URL: "avatar_url"
    pub const AVATAR_URL: &str = "avatar_url";
    /// File field: "file"
    pub const FILE: &str = "file";
    /// Path field: "path"
    pub const PATH: &str = "path";
    /// Attempts field: "attempts"
    pub const ATTEMPTS: &str = "attempts";
    /// Max attempts: "max_attempts"
    pub const MAX_ATTEMPTS: &str = "max_attempts";

    /// All registered static field names (sorted for binary search).
    /// Use `lookup()` to check if a field name is registered.
    pub const ALL_SORTED: &[&str] = &[
        ACTIVE,
        AGE,
        AMOUNT,
        ARCHIVED,
        ATTEMPTS,
        AUTHOR_ID,
        AVATAR_URL,
        BODY,
        CATEGORY,
        COMPLETED_AT,
        CONTENT,
        COUNT,
        CREATED_AT,
        DATA,
        DELETED,
        DELETED_AT,
        DESCRIPTION,
        EMAIL,
        ENABLED,
        EXPIRES_AT,
        FILE,
        FIRST_NAME,
        FLAGGED,
        ID,
        IMAGE_URL,
        KIND,
        LAST_NAME,
        MAX_ATTEMPTS,
        METADATA,
        NAME,
        ORDER,
        ORG_ID,
        OWNER_ID,
        PARENT_ID,
        PASSWORD,
        PATH,
        POSITION,
        PRICE,
        PRIORITY,
        PUBLISHED,
        PUBLISHED_AT,
        QUANTITY,
        ROLE,
        SCORE,
        SLUG,
        STARTED_AT,
        STATUS,
        TAGS,
        TENANT_ID,
        TITLE,
        TYPE,
        UPDATED_AT,
        URL,
        USER_ID,
        UUID,
        VERIFIED,
        VERSION,
    ];

    /// Look up a field name in the static registry using binary search.
    /// Returns `Some(&'static str)` if found, `None` otherwise.
    ///
    /// # Performance
    ///
    /// O(log n) binary search through ~57 entries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use prax_query::intern::fields;
    ///
    /// assert_eq!(fields::lookup("id"), Some("id"));
    /// assert_eq!(fields::lookup("email"), Some("email"));
    /// assert_eq!(fields::lookup("unknown"), None);
    /// ```
    #[inline]
    pub fn lookup(name: &str) -> Option<&'static str> {
        ALL_SORTED.binary_search(&name).ok().map(|i| ALL_SORTED[i])
    }

    /// Get a field name as `Cow<'static, str>`, using static lookup first.
    ///
    /// If the field name matches a registered static field, returns `Cow::Borrowed`.
    /// Otherwise, returns `Cow::Owned` with the input string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use prax_query::intern::fields;
    /// use std::borrow::Cow;
    ///
    /// // Static field - zero allocation
    /// let name = fields::as_cow("id");
    /// assert!(matches!(name, Cow::Borrowed(_)));
    ///
    /// // Unknown field - allocates
    /// let name = fields::as_cow("custom_field");
    /// assert!(matches!(name, Cow::Owned(_)));
    /// ```
    #[inline]
    pub fn as_cow(name: &str) -> std::borrow::Cow<'static, str> {
        match lookup(name) {
            Some(s) => std::borrow::Cow::Borrowed(s),
            None => std::borrow::Cow::Owned(name.to_string()),
        }
    }
}

/// Thread-local string interner.
///
/// Uses `Arc<str>` for reference-counted string storage. Interned strings are
/// stored in a thread-local `HashSet` for deduplication.
thread_local! {
    static INTERNER: RefCell<HashSet<Arc<str>>> = RefCell::new(HashSet::new());
}

/// Intern a string, returning a reference-counted pointer.
///
/// If the string has been interned before, returns the existing `Arc<str>`.
/// Otherwise, allocates a new `Arc<str>` and stores it for future lookups.
///
/// # Performance
///
/// - First call for a string: O(n) where n is string length (allocation + hash)
/// - Subsequent calls: O(n) for hash lookup, but no allocation
/// - Cloning the result: O(1) (just incrementing reference count)
///
/// # Examples
///
/// ```rust
/// use prax_query::intern::intern;
/// use std::sync::Arc;
///
/// let s1 = intern("field_name");
/// let s2 = intern("field_name");
///
/// // Both point to the same allocation
/// assert!(Arc::ptr_eq(&s1, &s2));
/// ```
#[inline]
pub fn intern(s: &str) -> Arc<str> {
    INTERNER.with(|interner| {
        let mut set = interner.borrow_mut();

        // Check if already interned
        if let Some(existing) = set.get(s) {
            return Arc::clone(existing);
        }

        // Intern the new string
        let arc: Arc<str> = Arc::from(s);
        set.insert(Arc::clone(&arc));
        arc
    })
}

/// Intern a string and return it as a `Cow<'static, str>`.
///
/// This is a convenience function for use with filter APIs that expect `FieldName`.
/// The returned `Cow` contains an owned `String` created from the interned `Arc<str>`.
///
/// # Note
///
/// For static strings, prefer using them directly (e.g., `"id".into()`) as that
/// creates a `Cow::Borrowed` with zero allocation. Use this function only for
/// dynamic strings that are repeated many times.
///
/// # Examples
///
/// ```rust
/// use prax_query::intern::intern_cow;
/// use prax_query::{Filter, FilterValue};
///
/// // Good: Interning a dynamic field name used in many filters
/// let field_name = format!("field_{}", 42);
/// let filter1 = Filter::Equals(intern_cow(&field_name), FilterValue::Int(1));
/// let filter2 = Filter::Equals(intern_cow(&field_name), FilterValue::Int(2));
/// ```
#[inline]
pub fn intern_cow(s: &str) -> Cow<'static, str> {
    // Note: We convert Arc<str> to String here because Cow<'static, str>
    // can't hold an Arc. The interning benefit comes from the HashSet
    // deduplication during the intern() call.
    Cow::Owned(intern(s).to_string())
}

/// Clear all interned strings from the thread-local cache.
///
/// This is primarily useful for testing or when you know interned strings
/// will no longer be needed and want to free memory.
///
/// # Examples
///
/// ```rust
/// use prax_query::intern::{intern, clear_interned};
///
/// let _ = intern("some_field");
/// // ... use the interned string ...
///
/// // Later, free all interned strings
/// clear_interned();
/// ```
pub fn clear_interned() {
    INTERNER.with(|interner| {
        interner.borrow_mut().clear();
    });
}

/// Get the number of currently interned strings.
///
/// Useful for debugging and memory profiling.
///
/// # Examples
///
/// ```rust
/// use prax_query::intern::{intern, interned_count, clear_interned};
///
/// clear_interned();
/// assert_eq!(interned_count(), 0);
///
/// intern("field1");
/// intern("field2");
/// intern("field1"); // Already interned, doesn't increase count
///
/// assert_eq!(interned_count(), 2);
/// ```
pub fn interned_count() -> usize {
    INTERNER.with(|interner| interner.borrow().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_returns_same_arc() {
        clear_interned();

        let s1 = intern("test_field");
        let s2 = intern("test_field");

        // Same Arc pointer
        assert!(Arc::ptr_eq(&s1, &s2));
        assert_eq!(&*s1, "test_field");
    }

    #[test]
    fn test_intern_different_strings() {
        clear_interned();

        let s1 = intern("field_a");
        let s2 = intern("field_b");

        // Different Arc pointers
        assert!(!Arc::ptr_eq(&s1, &s2));
        assert_eq!(&*s1, "field_a");
        assert_eq!(&*s2, "field_b");
    }

    #[test]
    fn test_interned_count() {
        clear_interned();

        assert_eq!(interned_count(), 0);

        intern("a");
        assert_eq!(interned_count(), 1);

        intern("b");
        assert_eq!(interned_count(), 2);

        intern("a"); // Already interned
        assert_eq!(interned_count(), 2);
    }

    #[test]
    fn test_clear_interned() {
        clear_interned();

        intern("x");
        intern("y");
        assert_eq!(interned_count(), 2);

        clear_interned();
        assert_eq!(interned_count(), 0);
    }

    #[test]
    fn test_intern_cow() {
        clear_interned();

        let cow = intern_cow("field_name");
        assert!(matches!(cow, Cow::Owned(_)));
        assert_eq!(cow.as_ref(), "field_name");
    }

    #[test]
    fn test_predefined_fields() {
        // Just verify the constants exist and have expected values
        assert_eq!(fields::ID, "id");
        assert_eq!(fields::EMAIL, "email");
        assert_eq!(fields::CREATED_AT, "created_at");
        assert_eq!(fields::USER_ID, "user_id");
        assert_eq!(fields::TENANT_ID, "tenant_id");
    }

    #[test]
    fn test_intern_empty_string() {
        clear_interned();

        let s1 = intern("");
        let s2 = intern("");

        assert!(Arc::ptr_eq(&s1, &s2));
        assert_eq!(&*s1, "");
    }

    #[test]
    fn test_intern_unicode() {
        clear_interned();

        let s1 = intern("フィールド");
        let s2 = intern("フィールド");

        assert!(Arc::ptr_eq(&s1, &s2));
        assert_eq!(&*s1, "フィールド");
    }

    #[test]
    fn test_fields_lookup() {
        // Test that lookup finds registered fields
        assert_eq!(fields::lookup("id"), Some("id"));
        assert_eq!(fields::lookup("email"), Some("email"));
        assert_eq!(fields::lookup("created_at"), Some("created_at"));
        assert_eq!(fields::lookup("user_id"), Some("user_id"));
        assert_eq!(fields::lookup("status"), Some("status"));

        // Test that lookup returns None for unknown fields
        assert_eq!(fields::lookup("unknown_field"), None);
        assert_eq!(fields::lookup("custom_field_123"), None);
    }

    #[test]
    fn test_fields_as_cow() {
        // Known field - should be Borrowed
        let cow = fields::as_cow("id");
        assert!(matches!(cow, Cow::Borrowed(_)));
        assert_eq!(cow.as_ref(), "id");

        // Unknown field - should be Owned
        let cow = fields::as_cow("custom_field");
        assert!(matches!(cow, Cow::Owned(_)));
        assert_eq!(cow.as_ref(), "custom_field");
    }

    #[test]
    fn test_fields_all_sorted() {
        // Verify the array is actually sorted
        let mut prev = "";
        for &field in fields::ALL_SORTED {
            assert!(
                field >= prev,
                "ALL_SORTED is not sorted: {} should come before {}",
                prev,
                field
            );
            prev = field;
        }
    }
}
