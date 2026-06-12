//! Concrete adapters bridging external library types to the executor's
//! port traits in [`crate::ports`].
//!
//! Today the only adapter is [`ProofmanAdapter`] wrapping `proofman_common::ProofCtx<F>` and
//! `SetupCtx<F>`. If a second adapter (e.g. a remote-proof backend) appears, split this file
//! into a directory module.

use fields::PrimeField64;
use proofman_common::{ProofCtx, Setup, SetupCtx};
use zisk_common::{StatsCostPerType, StatsType};
use zisk_pil::{
    ZiskPublicValues, MAIN_AIR_IDS, VIRTUAL_TABLE_ZISK_0_AIR_IDS, VIRTUAL_TABLE_ZISK_1_AIR_IDS,
    ZISK_AIRGROUP_ID,
};

use crate::error::{ExecutorResult, RwLockExt};
use crate::ports::{Dctx, GlobalId, InstanceInfo, ProofRegistry};
use crate::state::ExecutionState;

/// Adapter wrapping `ProofCtx<F>` + `SetupCtx<F>` for use through the port traits.
///
/// Holds the borrows for the duration of a phase call; constructed at
/// each `WitnessComponent::execute` / `calculate_witness` entry point.
pub struct ProofmanAdapter<'a, F: PrimeField64> {
    pctx: &'a ProofCtx<F>,
    sctx: &'a SetupCtx<F>,
    /// Per-`(airgroup_id, air_id)` instance count accumulated during planning, so the
    /// execution plan summary can be built without re-planning. Same lifetime as the
    /// adapter, which spans a single `execute_inner` call.
    instance_counts: std::sync::Mutex<std::collections::HashMap<(usize, usize), usize>>,
}

impl<'a, F: PrimeField64> ProofmanAdapter<'a, F> {
    #[inline]
    pub fn new(pctx: &'a ProofCtx<F>, sctx: &'a SetupCtx<F>) -> Self {
        Self {
            pctx,
            sctx,
            instance_counts: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    #[inline]
    fn track(&self, info: &InstanceInfo) {
        *self
            .instance_counts
            .lock()
            .expect("instance_counts mutex")
            .entry((info.airgroup_id, info.air_id))
            .or_insert(0) += 1;
    }

    /// Raw `ProofCtx<F>` borrow — used by `WitnessPhase::configure_sm_instances`
    /// which writes typed flags into `ZiskProofValues`. Only reached on the
    /// witness-mode path.
    #[inline]
    pub fn pctx(&self) -> &ProofCtx<F> {
        self.pctx
    }

    /// Reserve proofman's unified GPU buffer for the MO count-and-plan window.
    #[inline]
    pub fn acquire_gpu_buffer(&self) {
        self.pctx.acquire_first_gpu_buffer();
    }

    /// Release the buffer back to proofman once the MO runner has joined.
    #[inline]
    pub fn release_gpu_buffer(&self) {
        self.pctx.release_first_gpu_buffer();
    }

    /// Per-stats-type proving cost. Sole `SetupCtx<F>` consumer in the
    /// plan path; standalone callers skip it.
    pub fn compute_costs(
        &self,
        state: &ExecutionState<F>,
        main_instances_count: usize,
    ) -> ExecutorResult<StatsCostPerType> {
        let mut cost_per_type = StatsCostPerType::default();

        let setup_cost = |setup: &Setup<F>| -> u64 {
            let n_bits = setup.stark_info.stark_struct.n_bits;
            let total_cols: u64 = setup
                .stark_info
                .map_sections_n
                .iter()
                .filter(|(k, _)| *k != "const")
                .map(|(_, v)| *v)
                .sum();
            (1u64 << n_bits) * total_cols
        };

        let setup_main = self.sctx.get_setup(ZISK_AIRGROUP_ID, MAIN_AIR_IDS[0])?;
        cost_per_type
            .add_cost(StatsType::Main, setup_cost(setup_main) * main_instances_count as u64);

        let secn_instances = state.instance_set.secn_instances.read_or_poison("secn_instances")?;
        for (global_id, instance) in secn_instances.iter() {
            let info = self.instance_info(GlobalId(*global_id))?;
            let setup = self.sctx.get_setup(info.airgroup_id, info.air_id)?;
            cost_per_type.add_cost(instance.stats_type(), setup_cost(setup));
        }

        for air_id in [VIRTUAL_TABLE_ZISK_0_AIR_IDS[0], VIRTUAL_TABLE_ZISK_1_AIR_IDS[0]] {
            let setup = self.sctx.get_setup(ZISK_AIRGROUP_ID, air_id)?;
            cost_per_type.add_cost(StatsType::Tables, setup_cost(setup));
        }

        Ok(cost_per_type)
    }
}

impl<F: PrimeField64> Dctx for ProofmanAdapter<'_, F> {
    fn instance_info(&self, gid: GlobalId) -> ExecutorResult<InstanceInfo> {
        let (airgroup_id, air_id) = self.pctx.dctx_get_instance_info(gid.0)?;
        Ok(InstanceInfo::new(airgroup_id, air_id))
    }

    fn is_my_process_instance(&self, gid: GlobalId) -> ExecutorResult<bool> {
        Ok(self.pctx.dctx_is_my_process_instance(gid.0)?)
    }

    fn set_witness_ready(&self, gid: GlobalId, ready: bool) {
        self.pctx.set_witness_ready(gid.0, ready);
    }

    fn is_first_process(&self) -> bool {
        self.pctx.dctx_is_first_process()
    }
}

impl<F: PrimeField64> ProofRegistry for ProofmanAdapter<'_, F> {
    fn add_instance(&self, info: InstanceInfo) -> ExecutorResult<GlobalId> {
        self.track(&info);
        Ok(GlobalId(self.pctx.add_instance(info.airgroup_id, info.air_id)?))
    }

    fn add_instance_assign(&self, info: InstanceInfo) -> ExecutorResult<GlobalId> {
        self.track(&info);
        Ok(GlobalId(self.pctx.add_instance_assign(info.airgroup_id, info.air_id)?))
    }

    fn add_table(&self, info: InstanceInfo) -> ExecutorResult<GlobalId> {
        self.track(&info);
        Ok(GlobalId(self.pctx.add_table(info.airgroup_id, info.air_id)?))
    }

    fn instance_counts(&self) -> std::collections::HashMap<(usize, usize), usize> {
        self.instance_counts.lock().expect("instance_counts mutex").clone()
    }

    fn find_instance_id(&self, info: InstanceInfo) -> ExecutorResult<GlobalId> {
        let (_, gid) = self.pctx.dctx_find_instance_id(info.airgroup_id, info.air_id)?;
        Ok(GlobalId(gid))
    }

    fn set_chunks(&self, gid: GlobalId, chunks: &[usize], is_memory_related: bool) {
        self.pctx.dctx_set_chunks(gid.0, chunks.to_vec(), is_memory_related);
    }

    fn write_pub_outs(&self, pub_outs: &[(u64, u32)]) {
        let mut publics = ZiskPublicValues::from_vec_guard(self.pctx.get_publics());
        for &(index, value) in pub_outs {
            publics.inputs[index as usize] = F::from_u32(value);
        }
    }
}

/// Registry impl for the standalone execution path. Assignments return
/// `GlobalId(0)` — safe because standalone never consumes the assigned
/// ids (no populator, no checkpoints, no cost). `Dctx::instance_info`
/// is unreachable on this path. `write_pub_outs` is captured so the
/// standalone caller can retrieve the program's public outputs; each
/// `add_instance*` / `add_table` call is counted per `(airgroup, air_id)`
/// so the caller can build a plan summary.
#[derive(Default)]
pub struct NoopProofRegistry {
    pub_outs: std::sync::Mutex<Vec<(u64, u32)>>,
    instance_counts: std::sync::Mutex<std::collections::HashMap<(usize, usize), usize>>,
}

impl NoopProofRegistry {
    /// Drain the captured public outputs.
    pub fn take_pub_outs(&self) -> Vec<(u64, u32)> {
        std::mem::take(&mut *self.pub_outs.lock().expect("pub_outs mutex"))
    }

    /// Drain the per-`(airgroup, air_id)` instance counts.
    pub fn take_instance_counts(&self) -> std::collections::HashMap<(usize, usize), usize> {
        std::mem::take(&mut *self.instance_counts.lock().expect("instance_counts mutex"))
    }

    fn track(&self, info: InstanceInfo) {
        *self
            .instance_counts
            .lock()
            .expect("instance_counts mutex")
            .entry((info.airgroup_id, info.air_id))
            .or_insert(0) += 1;
    }
}

impl Dctx for NoopProofRegistry {
    fn instance_info(&self, gid: GlobalId) -> ExecutorResult<InstanceInfo> {
        Err(crate::error::ExecutorError::InstanceNotFound { global_id: gid.0 })
    }
    fn is_my_process_instance(&self, _gid: GlobalId) -> ExecutorResult<bool> {
        Ok(true)
    }
    fn set_witness_ready(&self, _gid: GlobalId, _ready: bool) {}
    fn is_first_process(&self) -> bool {
        true
    }
}

impl ProofRegistry for NoopProofRegistry {
    fn add_instance(&self, info: InstanceInfo) -> ExecutorResult<GlobalId> {
        self.track(info);
        Ok(GlobalId(0))
    }
    fn add_instance_assign(&self, info: InstanceInfo) -> ExecutorResult<GlobalId> {
        self.track(info);
        Ok(GlobalId(0))
    }
    fn add_table(&self, info: InstanceInfo) -> ExecutorResult<GlobalId> {
        self.track(info);
        Ok(GlobalId(0))
    }
    fn find_instance_id(&self, _info: InstanceInfo) -> ExecutorResult<GlobalId> {
        Ok(GlobalId(0))
    }
    fn set_chunks(&self, _gid: GlobalId, _chunks: &[usize], _is_memory_related: bool) {}
    fn write_pub_outs(&self, pub_outs: &[(u64, u32)]) {
        self.pub_outs.lock().expect("pub_outs mutex").extend_from_slice(pub_outs);
    }
    fn instance_counts(&self) -> std::collections::HashMap<(usize, usize), usize> {
        self.instance_counts.lock().expect("instance_counts mutex").clone()
    }
}
