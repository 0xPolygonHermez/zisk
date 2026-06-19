//! Port traits for the executor's interaction with `proofman_common`.
//!
//! These traits are the executor's **anti-corruption layer**: instead of
//! letting `ProofCtx` / `SetupCtx` types flow through every phase, we
//! depend on the small surface defined here. Concrete adapters live in
//! [`crate::adapters`].
//!
//! ## Trait shape
//!
//! - [`Dctx`] — distribution / proof-context bits shared by every
//!   pctx-touching component: instance info, rank ownership, witness
//!   readiness flag.
//! - [`ProofRegistry`] (`: Dctx`) — adds the pctx-mutating operations
//!   used by `PlanPhase`: instance/table assignment, chunk
//!   configuration, public-output injection.

#[cfg(test)]
use crate::error::ExecutorError;
use crate::error::ExecutorResult;

/// Newtype around a global instance ID assigned by the proof context.
///
/// Wraps a `usize` so phase signatures don't pass around bare integers
/// and so `From` conversions make the conceptual cast explicit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlobalId(pub usize);

impl GlobalId {
    /// Returns the underlying `usize`.
    #[inline]
    pub fn as_usize(self) -> usize {
        self.0
    }
}

impl From<usize> for GlobalId {
    #[inline]
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<GlobalId> for usize {
    #[inline]
    fn from(value: GlobalId) -> Self {
        value.0
    }
}

/// Self-documenting `(airgroup_id, air_id)` pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InstanceInfo {
    /// The AIR group this instance belongs to.
    pub airgroup_id: usize,
    /// The AIR id within the group.
    pub air_id: usize,
}

impl InstanceInfo {
    /// Constructs a new [`InstanceInfo`].
    #[inline]
    pub fn new(airgroup_id: usize, air_id: usize) -> Self {
        Self { airgroup_id, air_id }
    }
}

impl From<(usize, usize)> for InstanceInfo {
    #[inline]
    fn from(value: (usize, usize)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<InstanceInfo> for (usize, usize) {
    #[inline]
    fn from(info: InstanceInfo) -> Self {
        (info.airgroup_id, info.air_id)
    }
}

/// Distribution-context bits shared by every pctx-touching component.
///
/// Implementations exist on [`crate::adapters::ProofmanAdapter`] (real
/// `ProofCtx<F>`) and on test fakes.
pub trait Dctx {
    /// Returns `(airgroup_id, air_id)` for the instance `gid`.
    fn instance_info(&self, gid: GlobalId) -> ExecutorResult<InstanceInfo>;

    /// Returns `true` if the local rank owns the instance `gid`.
    fn is_my_process_instance(&self, gid: GlobalId) -> ExecutorResult<bool>;

    /// Marks the witness for `gid` as ready (`true`) or not-ready
    /// (`false`).
    fn set_witness_ready(&self, gid: GlobalId, ready: bool);

    /// Returns `true` if the local rank is the first process in the
    /// distribution group. Drives ASM ROM-histogram ownership.
    fn is_first_process(&self) -> bool;
}

/// Operations `MaterializePhase` needs from the proof context.
///
/// Inherits [`Dctx`] for the shared distribution queries.
pub trait ProofRegistry: Dctx {
    /// Registers a distributed instance (any rank may own it). Returns
    /// the assigned global id.
    fn add_instance(&self, info: InstanceInfo) -> ExecutorResult<GlobalId>;

    /// Registers a rank-owned instance (this rank owns it). Returns the
    /// assigned global id. Used for ROM and `rank_assign: true`
    /// precompiles (today: only Keccakf).
    fn add_instance_assign(&self, info: InstanceInfo) -> ExecutorResult<GlobalId>;

    /// Registers a table instance. Returns the assigned global id.
    fn add_table(&self, info: InstanceInfo) -> ExecutorResult<GlobalId>;

    /// Looks up the previously-assigned global id for an AIR. Used by
    /// the planner to attach the ROM instance to its existing
    /// rank-assignment.
    fn find_instance_id(&self, info: InstanceInfo) -> ExecutorResult<GlobalId>;

    /// Configures which chunks the instance `gid` needs.
    ///
    /// `is_memory_related` flags AIRs that the dctx must treat as
    /// memory-bound (MEM, ROM_DATA, INPUT_DATA).
    fn set_chunks(&self, gid: GlobalId, chunks: &[usize], is_memory_related: bool);

    /// Writes public-output values into the proof context's publics.
    /// Each entry is `(index, value32)`; the adapter handles any
    /// field-specific conversion (e.g. `F::from_u32`).
    fn write_pub_outs(&self, pub_outs: &[(u64, u32)]);

    /// Per-`(airgroup_id, air_id)` count of instances registered during planning.
    /// Used to build the execution plan summary. Defaults to empty for registries
    /// that don't track it.
    fn instance_counts(&self) -> std::collections::HashMap<(usize, usize), usize> {
        std::collections::HashMap::new()
    }
}

// ────────────────────────────────────────────────────────────────────
// In-crate test fixtures. Compiled only under `cfg(test)`.
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub(crate) mod fakes {
    //! Test fakes for the port traits. Available to any `#[cfg(test)]`
    //! code in the executor crate.
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// Kind of registration call made via [`FakeProofRegistry`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AddKind {
        /// `add_instance` (distributed).
        Instance,
        /// `add_instance_assign` (rank-owned).
        InstanceAssign,
        /// `add_table`.
        Table,
    }

    /// Record of one registration call.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AddCall {
        pub kind: AddKind,
        pub info: InstanceInfo,
        pub gid: GlobalId,
    }

    /// In-memory implementation of [`ProofRegistry`] / [`Dctx`] for unit
    /// tests. Records every call so tests can assert on the sequence.
    ///
    /// Defaults: `is_my_process_instance` = `true` for any gid. Override
    /// via the configuration fields.
    pub struct FakeProofRegistry {
        next_id: RefCell<usize>,
        /// Sequence of `add_*` calls, in order.
        pub additions: RefCell<Vec<AddCall>>,
        /// Witness-ready flag per gid (latest value wins).
        pub witness_ready: RefCell<HashMap<GlobalId, bool>>,
        /// Sequence of `set_chunks` calls, in order.
        pub set_chunks_calls: RefCell<Vec<(GlobalId, Vec<usize>, bool)>>,
        /// Cumulative public outputs written.
        pub pub_outs: RefCell<Vec<(u64, u32)>>,
        /// Per-gid ownership override. Missing key = owned (`true`).
        pub ownership: RefCell<HashMap<GlobalId, bool>>,
    }

    impl Default for FakeProofRegistry {
        fn default() -> Self {
            Self {
                next_id: RefCell::new(0),
                additions: RefCell::default(),
                witness_ready: RefCell::default(),
                set_chunks_calls: RefCell::default(),
                pub_outs: RefCell::default(),
                ownership: RefCell::default(),
            }
        }
    }

    impl FakeProofRegistry {
        /// New fake with all defaults.
        pub fn new() -> Self {
            Self::default()
        }

        fn next_gid(&self, kind: AddKind, info: InstanceInfo) -> GlobalId {
            let mut next = self.next_id.borrow_mut();
            let gid = GlobalId(*next);
            *next += 1;
            self.additions.borrow_mut().push(AddCall { kind, info, gid });
            gid
        }
    }

    impl Dctx for FakeProofRegistry {
        fn instance_info(&self, gid: GlobalId) -> ExecutorResult<InstanceInfo> {
            self.additions
                .borrow()
                .iter()
                .find(|a| a.gid == gid)
                .map(|a| a.info)
                .ok_or(ExecutorError::InstanceNotFound { global_id: gid.0 })
        }

        fn is_my_process_instance(&self, gid: GlobalId) -> ExecutorResult<bool> {
            Ok(self.ownership.borrow().get(&gid).copied().unwrap_or(true))
        }

        fn set_witness_ready(&self, gid: GlobalId, ready: bool) {
            self.witness_ready.borrow_mut().insert(gid, ready);
        }

        fn is_first_process(&self) -> bool {
            true
        }
    }

    impl ProofRegistry for FakeProofRegistry {
        fn add_instance(&self, info: InstanceInfo) -> ExecutorResult<GlobalId> {
            Ok(self.next_gid(AddKind::Instance, info))
        }
        fn add_instance_assign(&self, info: InstanceInfo) -> ExecutorResult<GlobalId> {
            Ok(self.next_gid(AddKind::InstanceAssign, info))
        }
        fn add_table(&self, info: InstanceInfo) -> ExecutorResult<GlobalId> {
            Ok(self.next_gid(AddKind::Table, info))
        }
        fn find_instance_id(&self, info: InstanceInfo) -> ExecutorResult<GlobalId> {
            self.additions
                .borrow()
                .iter()
                .find(|a| a.info == info)
                .map(|a| a.gid)
                .ok_or(ExecutorError::SecnPlanMissing { phase: "find_instance_id" })
        }
        fn set_chunks(&self, gid: GlobalId, chunks: &[usize], is_memory_related: bool) {
            self.set_chunks_calls.borrow_mut().push((gid, chunks.to_vec(), is_memory_related));
        }
        fn write_pub_outs(&self, pub_outs: &[(u64, u32)]) {
            self.pub_outs.borrow_mut().extend_from_slice(pub_outs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fakes::{AddKind, FakeProofRegistry};

    #[test]
    fn global_id_round_trips_through_usize() {
        let gid: GlobalId = 42usize.into();
        let back: usize = gid.into();
        assert_eq!(back, 42);
        assert_eq!(gid.as_usize(), 42);
    }

    #[test]
    fn instance_info_round_trips_through_tuple() {
        let info: InstanceInfo = (3, 7).into();
        let back: (usize, usize) = info.into();
        assert_eq!(back, (3, 7));
        assert_eq!(info.airgroup_id, 3);
        assert_eq!(info.air_id, 7);
    }

    #[test]
    fn fake_registry_records_add_instance_calls_in_order() {
        let reg = FakeProofRegistry::new();
        let a = reg.add_instance(InstanceInfo::new(0, 10)).unwrap();
        let b = reg.add_instance_assign(InstanceInfo::new(0, 11)).unwrap();
        let c = reg.add_table(InstanceInfo::new(0, 12)).unwrap();
        let additions = reg.additions.borrow();
        assert_eq!(additions.len(), 3);
        assert_eq!(additions[0].kind, AddKind::Instance);
        assert_eq!(additions[1].kind, AddKind::InstanceAssign);
        assert_eq!(additions[2].kind, AddKind::Table);
        assert_eq!(additions[0].gid, a);
        assert_eq!(additions[1].gid, b);
        assert_eq!(additions[2].gid, c);
    }

    #[test]
    fn fake_registry_instance_info_round_trips_assignment() {
        let reg = FakeProofRegistry::new();
        let info = InstanceInfo::new(1, 99);
        let gid = reg.add_instance(info).unwrap();
        assert_eq!(reg.instance_info(gid).unwrap(), info);
    }

    #[test]
    fn fake_registry_find_by_info() {
        let reg = FakeProofRegistry::new();
        let info = InstanceInfo::new(1, 99);
        let gid = reg.add_instance(info).unwrap();
        assert_eq!(reg.find_instance_id(info).unwrap(), gid);
        assert!(reg.find_instance_id(InstanceInfo::new(0, 0)).is_err());
    }

    #[test]
    fn fake_registry_ownership_default_is_owned() {
        let reg = FakeProofRegistry::new();
        let gid = reg.add_instance(InstanceInfo::new(0, 1)).unwrap();
        assert!(reg.is_my_process_instance(gid).unwrap());
    }

    #[test]
    fn fake_registry_ownership_override() {
        let reg = FakeProofRegistry::new();
        let gid = reg.add_instance(InstanceInfo::new(0, 1)).unwrap();
        reg.ownership.borrow_mut().insert(gid, false);
        assert!(!reg.is_my_process_instance(gid).unwrap());
    }

    #[test]
    fn fake_registry_records_set_chunks_in_order() {
        let reg = FakeProofRegistry::new();
        let a = reg.add_instance(InstanceInfo::new(0, 1)).unwrap();
        let b = reg.add_instance(InstanceInfo::new(0, 2)).unwrap();
        reg.set_chunks(a, &[0, 1, 2], false);
        reg.set_chunks(b, &[5], true);
        let calls = reg.set_chunks_calls.borrow();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], (a, vec![0, 1, 2], false));
        assert_eq!(calls[1], (b, vec![5], true));
    }

    #[test]
    fn fake_registry_accumulates_pub_outs() {
        let reg = FakeProofRegistry::new();
        reg.write_pub_outs(&[(0, 0xAA), (1, 0xBB)]);
        reg.write_pub_outs(&[(2, 0xCC)]);
        assert_eq!(*reg.pub_outs.borrow(), vec![(0, 0xAA), (1, 0xBB), (2, 0xCC)]);
    }
}
