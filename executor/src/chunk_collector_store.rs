//! [`ChunkCollectorStore`] — the lock-protected per-instance chunk
//! collector map, written during the witness phase by rayon worker
//! threads.
//!
//! Splitting this out of [`crate::ExecutionState`] is the
//! `ChunkCollectorStore` half of step 3.4: the genuinely
//! lock-contested storage lives behind a clearly-named type, separate
//! from the *write-once* [`crate::InstanceSet`] populated by
//! `MaterializePhase`.
//!
//! The contained `Arc<RwLock<HashMap<...>>>` is intentionally exposed
//! as a public field so the `ChunkDataCollector` rayon scope can
//! clone it; this is the one place in the executor where a deliberate
//! shared-mutable hot-path-adjacent lock lives, and the type's name
//! is the contract.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::state::ChunkCollector;

/// Map of `global_id → per-chunk collector vector`. Wrapped in an
/// `Arc<RwLock<...>>` so the rayon scope inside
/// [`crate::ChunkDataCollector::collect`] can clone the handle and
/// have multiple worker threads write to it concurrently.
pub struct ChunkCollectorStore {
    /// Backing map. Public because the rayon scope clones the `Arc`
    /// into worker tasks; encapsulating it further would require
    /// per-call wrappers without changing the hot-path semantics.
    pub inner: Arc<RwLock<HashMap<usize, Vec<Option<ChunkCollector>>>>>,
}

impl ChunkCollectorStore {
    /// Construct an empty store. Worker threads populate it during
    /// `calculate_witness`.
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Drop every recorded collector. Called by
    /// [`crate::ExecutionState::reset`] between executions.
    pub fn reset(&self) {
        self.inner.write().unwrap().clear();
    }

    /// Returns `true` when no collectors are recorded. Useful as a
    /// post-`MaterializePhase` assertion (collectors only fill
    /// during the subsequent witness phase).
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
}
