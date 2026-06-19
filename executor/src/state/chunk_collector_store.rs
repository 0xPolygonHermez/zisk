//! [`ChunkCollectorStore`] — per-instance chunk collector map.
use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use crate::state::ChunkCollector;

/// Map of `global_id → per-chunk collector vector`.
pub struct ChunkCollectorStore {
    /// Backing map.
    pub inner: Arc<RwLock<HashMap<usize, Vec<Option<ChunkCollector>>>>>,
}

impl ChunkCollectorStore {
    /// Construct an empty store.
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Drop every recorded collector. Called by
    /// [`crate::ExecutionState::reset`] between executions so the
    /// next proof starts from a clean slate.
    pub fn reset(&self) {
        self.inner.write().unwrap_or_else(PoisonError::into_inner).clear();
    }

    /// Returns `true` when no collectors are recorded.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.inner.read().map(|g| g.is_empty()).unwrap_or(true)
    }
}

impl Default for ChunkCollectorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let store = ChunkCollectorStore::new();
        assert!(store.is_empty());
    }

    #[test]
    fn default_matches_new() {
        let store = ChunkCollectorStore::default();
        assert!(store.is_empty());
    }

    #[test]
    fn reset_clears_map() {
        let store = ChunkCollectorStore::new();
        store.inner.write().unwrap().insert(7, vec![None, None]);
        assert!(!store.is_empty());
        store.reset();
        assert!(store.is_empty());
    }

    #[test]
    fn reset_recovers_from_poisoned_inner() {
        use std::panic::{catch_unwind, AssertUnwindSafe};

        let store = Arc::new(ChunkCollectorStore::new());
        let store_for_panic = store.clone();
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = store_for_panic.inner.write().unwrap();
            panic!("intentional poison");
        }));
        assert!(store.inner.is_poisoned());
        store.reset();
        assert!(store.is_empty());
    }
}
