//! Lazy loading utilities for relations.
//!
//! This module provides lazy loading wrappers that defer loading of related
//! data until it is actually accessed, improving performance when relations
//! are not always needed.
//!
//! # Example
//!
//! ```rust,ignore
//! use prax_query::lazy::Lazy;
//!
//! struct User {
//!     id: i64,
//!     name: String,
//!     // Posts are lazily loaded
//!     posts: Lazy<Vec<Post>>,
//! }
//!
//! // Posts are not loaded until accessed
//! let posts = user.posts.load(&engine).await?;
//! ```

use std::cell::UnsafeCell;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

/// State of a lazy value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum LazyState {
    /// Not yet loaded.
    Unloaded = 0,
    /// Currently loading.
    Loading = 1,
    /// Loaded successfully.
    Loaded = 2,
    /// Failed to load.
    Failed = 3,
}

impl From<u8> for LazyState {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Unloaded,
            1 => Self::Loading,
            2 => Self::Loaded,
            3 => Self::Failed,
            _ => Self::Unloaded,
        }
    }
}

/// A lazily-loaded value.
///
/// The value is not loaded until explicitly requested, allowing
/// for deferred loading of expensive relations.
pub struct Lazy<T> {
    state: AtomicU8,
    value: UnsafeCell<Option<T>>,
}

// SAFETY: Lazy uses atomic operations for state and only allows
// mutable access when state transitions are valid.
unsafe impl<T: Send> Send for Lazy<T> {}
unsafe impl<T: Sync> Sync for Lazy<T> {}

impl<T> Lazy<T> {
    /// Create a new unloaded lazy value.
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(LazyState::Unloaded as u8),
            value: UnsafeCell::new(None),
        }
    }

    /// Create a lazy value that is already loaded.
    pub fn loaded(value: T) -> Self {
        Self {
            state: AtomicU8::new(LazyState::Loaded as u8),
            value: UnsafeCell::new(Some(value)),
        }
    }

    /// Check if the value has been loaded.
    #[inline]
    pub fn is_loaded(&self) -> bool {
        LazyState::from(self.state.load(Ordering::Acquire)) == LazyState::Loaded
    }

    /// Check if the value is currently loading.
    #[inline]
    pub fn is_loading(&self) -> bool {
        LazyState::from(self.state.load(Ordering::Acquire)) == LazyState::Loading
    }

    /// Get the value if it has been loaded.
    pub fn get(&self) -> Option<&T> {
        if self.is_loaded() {
            // SAFETY: We only read when state is Loaded, which means
            // the value has been written and won't change.
            unsafe { (*self.value.get()).as_ref() }
        } else {
            None
        }
    }

    /// Get a mutable reference to the value if loaded.
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.is_loaded() {
            self.value.get_mut().as_mut()
        } else {
            None
        }
    }

    /// Set the value directly.
    pub fn set(&self, value: T) {
        // SAFETY: We're transitioning to Loaded state
        unsafe {
            *self.value.get() = Some(value);
        }
        self.state.store(LazyState::Loaded as u8, Ordering::Release);
    }

    /// Take the value, leaving the lazy unloaded.
    pub fn take(&mut self) -> Option<T> {
        if self.is_loaded() {
            self.state
                .store(LazyState::Unloaded as u8, Ordering::Release);
            self.value.get_mut().take()
        } else {
            None
        }
    }

    /// Reset to unloaded state.
    pub fn reset(&mut self) {
        self.state
            .store(LazyState::Unloaded as u8, Ordering::Release);
        *self.value.get_mut() = None;
    }

    /// Load the value using the provided async loader function.
    ///
    /// If already loaded, returns the cached value.
    /// If loading fails, the error is returned but the state remains unloaded.
    pub async fn load_with<F, Fut, E>(&self, loader: F) -> Result<&T, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        // Fast path: already loaded
        if self.is_loaded() {
            // SAFETY: State is Loaded, value is immutable
            return Ok(unsafe { (*self.value.get()).as_ref().unwrap() });
        }

        // Try to transition to loading state
        let prev = self.state.compare_exchange(
            LazyState::Unloaded as u8,
            LazyState::Loading as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        );

        match prev {
            Ok(_) => {
                // We own the loading transition
                match loader().await {
                    Ok(value) => {
                        // SAFETY: We're the only writer (we hold Loading state)
                        unsafe {
                            *self.value.get() = Some(value);
                        }
                        self.state.store(LazyState::Loaded as u8, Ordering::Release);
                        // SAFETY: We just stored the value
                        Ok(unsafe { (*self.value.get()).as_ref().unwrap() })
                    }
                    Err(e) => {
                        self.state
                            .store(LazyState::Unloaded as u8, Ordering::Release);
                        Err(e)
                    }
                }
            }
            Err(current) => {
                // Someone else is loading or already loaded
                match LazyState::from(current) {
                    LazyState::Loaded => {
                        // SAFETY: State is Loaded
                        Ok(unsafe { (*self.value.get()).as_ref().unwrap() })
                    }
                    LazyState::Loading => {
                        // Wait for the other loader (spin with yield)
                        loop {
                            tokio::task::yield_now().await;
                            let state = LazyState::from(self.state.load(Ordering::Acquire));
                            match state {
                                LazyState::Loaded => {
                                    return Ok(unsafe { (*self.value.get()).as_ref().unwrap() });
                                }
                                LazyState::Unloaded | LazyState::Failed => {
                                    // Other loader failed, try again
                                    return Box::pin(self.load_with(loader)).await;
                                }
                                LazyState::Loading => continue,
                            }
                        }
                    }
                    _ => {
                        // Retry loading
                        Box::pin(self.load_with(loader)).await
                    }
                }
            }
        }
    }
}

impl<T: Default> Default for Lazy<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for Lazy<T> {
    fn clone(&self) -> Self {
        if self.is_loaded() {
            Self::loaded(self.get().unwrap().clone())
        } else {
            Self::new()
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Lazy<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = LazyState::from(self.state.load(Ordering::Acquire));
        match state {
            LazyState::Loaded => {
                if let Some(value) = self.get() {
                    f.debug_struct("Lazy")
                        .field("state", &"Loaded")
                        .field("value", value)
                        .finish()
                } else {
                    f.debug_struct("Lazy").field("state", &"Loaded").finish()
                }
            }
            _ => f.debug_struct("Lazy").field("state", &state).finish(),
        }
    }
}

/// A relation that can be lazily loaded.
///
/// This is similar to `Lazy` but includes the loader configuration.
pub struct LazyRelation<T, L> {
    /// The lazy value.
    pub value: Lazy<T>,
    /// The loader (e.g., query parameters).
    pub loader: L,
}

impl<T, L> LazyRelation<T, L> {
    /// Create a new lazy relation.
    pub fn new(loader: L) -> Self {
        Self {
            value: Lazy::new(),
            loader,
        }
    }

    /// Create a lazy relation with a pre-loaded value.
    pub fn loaded(value: T, loader: L) -> Self {
        Self {
            value: Lazy::loaded(value),
            loader,
        }
    }

    /// Check if the relation has been loaded.
    #[inline]
    pub fn is_loaded(&self) -> bool {
        self.value.is_loaded()
    }

    /// Get the loaded value if available.
    pub fn get(&self) -> Option<&T> {
        self.value.get()
    }
}

impl<T: Clone, L: Clone> Clone for LazyRelation<T, L> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            loader: self.loader.clone(),
        }
    }
}

impl<T: fmt::Debug, L: fmt::Debug> fmt::Debug for LazyRelation<T, L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LazyRelation")
            .field("value", &self.value)
            .field("loader", &self.loader)
            .finish()
    }
}

/// Configuration for a one-to-many relation loader.
#[derive(Debug, Clone)]
pub struct OneToManyLoader {
    /// The foreign key column in the related table.
    pub foreign_key: String,
    /// The local key value.
    pub local_key_value: crate::filter::FilterValue,
    /// The related table name.
    pub table: String,
}

impl OneToManyLoader {
    /// Create a new one-to-many loader.
    pub fn new(
        table: impl Into<String>,
        foreign_key: impl Into<String>,
        local_key_value: impl Into<crate::filter::FilterValue>,
    ) -> Self {
        Self {
            table: table.into(),
            foreign_key: foreign_key.into(),
            local_key_value: local_key_value.into(),
        }
    }
}

/// Configuration for a many-to-one relation loader.
#[derive(Debug, Clone)]
pub struct ManyToOneLoader {
    /// The foreign key value.
    pub foreign_key_value: crate::filter::FilterValue,
    /// The related table name.
    pub table: String,
    /// The primary key column in the related table.
    pub primary_key: String,
}

impl ManyToOneLoader {
    /// Create a new many-to-one loader.
    pub fn new(
        table: impl Into<String>,
        primary_key: impl Into<String>,
        foreign_key_value: impl Into<crate::filter::FilterValue>,
    ) -> Self {
        Self {
            table: table.into(),
            primary_key: primary_key.into(),
            foreign_key_value: foreign_key_value.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_new() {
        let lazy: Lazy<i32> = Lazy::new();
        assert!(!lazy.is_loaded());
        assert!(lazy.get().is_none());
    }

    #[test]
    fn test_lazy_loaded() {
        let lazy = Lazy::loaded(42);
        assert!(lazy.is_loaded());
        assert_eq!(lazy.get(), Some(&42));
    }

    #[test]
    fn test_lazy_set() {
        let lazy: Lazy<i32> = Lazy::new();
        lazy.set(42);
        assert!(lazy.is_loaded());
        assert_eq!(lazy.get(), Some(&42));
    }

    #[test]
    fn test_lazy_take() {
        let mut lazy = Lazy::loaded(42);
        let value = lazy.take();
        assert_eq!(value, Some(42));
        assert!(!lazy.is_loaded());
    }

    #[test]
    fn test_lazy_reset() {
        let mut lazy = Lazy::loaded(42);
        lazy.reset();
        assert!(!lazy.is_loaded());
        assert!(lazy.get().is_none());
    }

    #[test]
    fn test_lazy_clone() {
        let lazy = Lazy::loaded(42);
        let cloned = lazy.clone();
        assert!(cloned.is_loaded());
        assert_eq!(cloned.get(), Some(&42));
    }

    #[test]
    fn test_lazy_clone_unloaded() {
        let lazy: Lazy<i32> = Lazy::new();
        let cloned = lazy.clone();
        assert!(!cloned.is_loaded());
    }

    #[tokio::test]
    async fn test_lazy_load_with() {
        let lazy: Lazy<i32> = Lazy::new();

        let result = lazy.load_with(|| async { Ok::<_, &str>(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), &42);
        assert!(lazy.is_loaded());
    }

    #[tokio::test]
    async fn test_lazy_load_cached() {
        let lazy = Lazy::loaded(42);

        // Should return cached value
        let result = lazy.load_with(|| async { Ok::<_, &str>(100) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), &42); // Original value, not 100
    }

    #[test]
    fn test_lazy_relation() {
        let relation: LazyRelation<Vec<i32>, OneToManyLoader> =
            LazyRelation::new(OneToManyLoader::new("posts", "user_id", 1i64));

        assert!(!relation.is_loaded());
        assert!(relation.get().is_none());
    }
}
