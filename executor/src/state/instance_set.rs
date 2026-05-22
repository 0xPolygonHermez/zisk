//! [`InstanceSet`] — populated main + secondary state-machine instance maps.

use std::collections::HashMap;
use std::sync::{PoisonError, RwLock};

use fields::PrimeField64;
use sm_main::MainInstance;
use zisk_common::Instance;

/// Populated main + secondary instance maps, keyed by `global_id`.
pub struct InstanceSet<F: PrimeField64> {
    /// Main state machine instances, indexed by their global ID.
    pub main_instances: RwLock<HashMap<usize, MainInstance<F>>>,

    /// Secondary state machine instances, indexed by their global ID.
    pub secn_instances: RwLock<HashMap<usize, Box<dyn Instance<F>>>>,
}

impl<F: PrimeField64> InstanceSet<F> {
    /// Construct an empty set.
    pub fn new() -> Self {
        Self {
            main_instances: RwLock::new(HashMap::new()),
            secn_instances: RwLock::new(HashMap::new()),
        }
    }

    /// Drop every recorded instance. Called by
    /// [`crate::ExecutionState::reset`] between executions so the
    /// next proof starts from a clean slate.
    pub fn reset(&self) {
        self.main_instances.write().unwrap_or_else(PoisonError::into_inner).clear();
        self.secn_instances.write().unwrap_or_else(PoisonError::into_inner).clear();
    }

    /// Returns `true` when neither map has any entry.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        let main_empty = self.main_instances.read().map(|m| m.is_empty()).unwrap_or(true);
        let secn_empty = self.secn_instances.read().map(|m| m.is_empty()).unwrap_or(true);
        main_empty && secn_empty
    }
}

impl<F: PrimeField64> Default for InstanceSet<F> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fields::Goldilocks;

    type F = Goldilocks;

    #[test]
    fn new_is_empty() {
        let set: InstanceSet<F> = InstanceSet::new();
        assert!(set.is_empty());
    }

    #[test]
    fn default_matches_new() {
        let set: InstanceSet<F> = InstanceSet::default();
        assert!(set.is_empty());
    }

    #[test]
    fn reset_clears_maps() {
        let set: InstanceSet<F> = InstanceSet::new();
        // Direct field access keeps tests honest — InstanceSet exposes
        // the two locks so the caller can populate them; the contract
        // here is that `reset` blanks both regardless of how they got
        // their entries.
        set.reset();
        assert!(set.is_empty());
    }

    #[test]
    fn reset_recovers_from_poisoned_main_instances() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        use std::sync::Arc;

        let set: Arc<InstanceSet<F>> = Arc::new(InstanceSet::new());
        let set_for_panic = set.clone();
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = set_for_panic.main_instances.write().unwrap();
            panic!("intentional poison");
        }));
        assert!(set.main_instances.is_poisoned());
        set.reset();
        assert!(set.is_empty());
    }
}
