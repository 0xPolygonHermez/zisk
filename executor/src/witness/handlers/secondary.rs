//! [`SecondaryWitnessHandler`] — witness compute for non-ROM `InstanceType::Instance`
//! secondary state machines.

use std::collections::HashMap;
use std::sync::Mutex;

use proofman_common::{BufferPool, ProofCtx, SetupCtx};
use proofman_fields::PrimeField64;

use crate::error::{ExecutorError, ExecutorResult, MutexExt, RwLockExt};
use crate::state::ExecutionState;
use crate::{AirClassifier, ChunkDataCollector, WitnessGenerator};
use zisk_common::{BusDevice, Instance, PayloadType};

/// The collectors of one instance, as `collect_instance` returns them.
type Collectors = Vec<(usize, Box<dyn BusDevice<PayloadType>>)>;

/// The memory instances collected on the CPU after the device dropped the block: one collection
/// pass at a time, over every memory instance that still needs collectors, so an instance failing
/// or coming up meanwhile finds its collectors ready. A pass fills its store entries chunk by
/// chunk, so a memory instance reads the store only under this lock, once the pass is complete.
/// `on_device` holds the instances a witness hook is computing from the device rows right now: a
/// pass collects them too, as they may fail next.
struct MemFallback {
    on_device: Vec<usize>,
}
static MEM_FALLBACK: Mutex<MemFallback> = Mutex::new(MemFallback { on_device: Vec::new() });

/// Secondary handler witness computation for non-ROM `InstanceType::Instance` secondaries.
pub struct SecondaryWitnessHandler;

impl SecondaryWitnessHandler {
    /// Compute the witness for `global_id` (assumed to be a non-ROM `InstanceType::Instance`).
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch<F: PrimeField64>(
        generator: &WitnessGenerator,
        collector: &ChunkDataCollector<F>,
        state: &ExecutionState<F>,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_id: usize,
        buffer_pool: &dyn BufferPool<F>,
        stats_scope_id: u64,
        collect: bool,
    ) -> ExecutorResult<()> {
        let secn_instances = state.instance_set.secn_instances.read_or_poison("secn_instances")?;
        let secn_instance =
            secn_instances.get(&global_id).ok_or(ExecutorError::InstanceNotFound { global_id })?;

        let instance = &**secn_instance;
        let has_collectors = |global_id: usize| -> ExecutorResult<bool> {
            Ok(state
                .collector_store
                .inner
                .read_or_poison("collector_store")?
                .contains_key(&global_id))
        };
        // An instance whose rows come from the GPU planner gets no collectors; should the device
        // fail it after all, it is collected here and computed on the CPU like any other.
        if !collect {
            MEM_FALLBACK.lock_or_poison("mem_fallback")?.on_device.push(global_id);
            let trace_buffer = buffer_pool.take_buffer();
            let computed = generator.compute_secn_witness(
                pctx,
                sctx,
                state,
                global_id,
                instance,
                Vec::new(),
                trace_buffer,
                stats_scope_id,
            );
            MEM_FALLBACK.lock_or_poison("mem_fallback")?.on_device.retain(|&g| g != global_id);
            match computed {
                Ok(()) => return Ok(()),
                Err(e) => zisk_asm_runner::device_mem_witness_failed(&e.to_string()),
            }
        }
        let (_, air_id) = pctx.dctx_get_instance_info(global_id)?;
        if AirClassifier::is_memory_related(air_id) || AirClassifier::is_mem_align(air_id) {
            Self::collect_mem_pass(collector, state, pctx, &secn_instances, global_id, instance)?;
        } else if !has_collectors(global_id)? {
            collector.collect_single(pctx, state, global_id, instance)?;
        }
        let collectors: Collectors =
            state.take_collectors_for_instance(global_id, instance.instance_type())?;
        let trace_buffer = buffer_pool.take_buffer();

        generator.compute_secn_witness(
            pctx,
            sctx,
            state,
            global_id,
            instance,
            collectors,
            trace_buffer,
            stats_scope_id,
        )
    }

    /// Collects, when `global_id` has no collectors yet, every memory instance of the block this
    /// process proves that still needs them in one pass: `global_id`, the ones not computed yet
    /// and without collectors in the store, and the ones running on the device rows. Falls back
    /// to collecting `global_id` alone when that set cannot be listed.
    fn collect_mem_pass<F: PrimeField64>(
        collector: &ChunkDataCollector<F>,
        state: &ExecutionState<F>,
        pctx: &ProofCtx<F>,
        secn_instances: &HashMap<usize, Box<dyn Instance<F>>>,
        global_id: usize,
        instance: &dyn Instance<F>,
    ) -> ExecutorResult<()> {
        let fallback = MEM_FALLBACK.lock_or_poison("mem_fallback")?;
        if state.collector_store.inner.read_or_poison("collector_store")?.contains_key(&global_id) {
            return Ok(());
        }
        let pending = (|| -> ExecutorResult<HashMap<usize, &dyn Instance<F>>> {
            let store = state.collector_store.inner.read_or_poison("collector_store")?;
            let mut pending = HashMap::new();
            for (&gid, instance) in secn_instances.iter() {
                let (_, air_id) = pctx.dctx_get_instance_info(gid)?;
                if !(AirClassifier::is_memory_related(air_id)
                    || AirClassifier::is_mem_align(air_id))
                {
                    continue;
                }
                let needed = gid == global_id
                    || fallback.on_device.contains(&gid)
                    || !pctx.dctx_is_instance_calculated(gid);
                if !needed || store.contains_key(&gid) || !pctx.dctx_is_my_process_instance(gid)? {
                    continue;
                }
                pending.insert(gid, &**instance);
            }
            Ok(pending)
        })();
        match pending {
            Ok(pending) => {
                tracing::info!(
                    "collecting {} memory instances on the CPU in one pass",
                    pending.len()
                );
                collector.collect(pctx, state, pending)
            }
            Err(e) => {
                tracing::warn!(
                    "the memory instances still to compute could not be listed ({e}); each is collected as it comes up"
                );
                collector.collect_single(pctx, state, global_id, instance)
            }
        }
    }
}
