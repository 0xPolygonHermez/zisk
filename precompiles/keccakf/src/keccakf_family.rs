//! Hand-written precompile family for Keccak-f: one operation type, two airs.
//!
//! * `Keccakf` proves the *unique* permutations of the execution, each one once.
//! * `KeccakfProxy` proves every invocation: its memory accesses and its operation.
//!
//! The emulators tell the two apart with the *dedup word* that follows the input state of every
//! Keccak-f in the minimal trace (see `zisk_core::keccakf_dedup`): the index of its unique
//! permutation, plus a flag when the Keccak-f is a *replica*, i.e. an earlier one had the same
//! input state. So:
//!
//! * the counter counts, per chunk, the invocations and which permutation each replica repeats;
//! * the planner sizes `Keccakf` on the invocations that are not replicas and `KeccakfProxy`
//!   on all of them, and adds up the replicas of every unique permutation. That table is only
//!   complete once every chunk has been counted, and the proxy rows of the originals need it, so
//!   it travels in the metadata of the proxy plans;
//! * a `Keccakf` collector skips the replicas, a `KeccakfProxy` collector takes everything.
//!
//! `zisk_precompile!` cannot express this: its airs are one air at several heights, all proving
//! the same operations. The four public type names (`KeccakfManager`, `KeccakfCounterInputGen`,
//! `KeccakfInstance`, `KeccakfCollector`) are what `executor`'s `register_precompiles!` expects.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use proofman_common::{ProofCtx, SetupCtx};
use proofman_fields::PrimeField64;
use zisk_common::{
    plan_ladder, BusDevice, BusDeviceMetrics, BusDeviceMode, BusId, CheckPoint, ChunkId,
    CollectSkipper, ComponentBuilder, ComponentPlanBuilder, ExtOperationData, InstCount, Instance,
    InstanceCtx, InstanceType, Metrics, PayloadType, Plan, Planner, StatsType, B, OPERATION_BUS_ID,
    OPERATION_PRECOMPILED_BUS_DATA_SIZE, OP_TYPE, STEP,
};
use zisk_core::{
    keccakf_dedup_index, keccakf_dedup_is_replica, ZiskOperationType, KECCAKF_DEDUP_STATE_WORDS,
};
use zisk_pil::{
    KeccakfProxyTrace, KeccakfProxyTraceRow, KeccakfProxyTraceRowPacked, KeccakfTrace,
    KeccakfTraceRow, KeccakfTraceRowPacked, ZISK_AIRGROUP_ID,
};
use zisk_precomp_common::{MemProcessor, PrecompileMemInputs};

use crate::{keccakf_constants::*, KeccakfInput, KeccakfProxySM, KeccakfSM};

/// Air of the unique permutations.
pub const KECCAKF_AIR_ID: usize = KeccakfTrace::<()>::AIR_ID;

/// Air of the invocations.
pub const KECCAKF_PROXY_AIR_ID: usize = KeccakfProxyTrace::<()>::AIR_ID;

/// The airs of the family, `Keccakf` first: the executor registers the family under this list.
pub const KECCAKF_FAMILY_AIR_IDS: &[usize] = &[KECCAKF_AIR_ID, KECCAKF_PROXY_AIR_ID];

/// Unique permutations one `Keccakf` instance holds.
pub const KECCAKF_OPS_PER_INSTANCE: usize = OPS_PER_SLOT * (KeccakfTrace::<()>::NUM_ROWS / CLOCKS);

/// Invocations one `KeccakfProxy` instance holds: one per row.
pub const KECCAKF_PROXY_OPS_PER_INSTANCE: usize = KeccakfProxyTrace::<()>::NUM_ROWS;

/// Position of the dedup word in the operation-bus payload of a Keccak-f: after the input state.
const DEDUP_WORD: usize = OPERATION_PRECOMPILED_BUS_DATA_SIZE + KECCAKF_DEDUP_STATE_WORDS;

// ============================================================================
// CounterInputGen — counts invocations and replicas, drives the mem-input generation.
// ============================================================================

/// Counts the Keccak-f invocations of a chunk and drives `PrecompileMemInputs`. Used in all
/// three bus modes (`Counter`, `CounterAsm`, `InputGenerator`).
pub struct KeccakfCounterInputGen<F: PrimeField64> {
    /// Keccak-f invocations of the chunk, replicas included.
    pub count: u64,
    /// For every replica of the chunk, the index of the unique permutation it repeats.
    pub replicas: Vec<u32>,
    mode: BusDeviceMode,
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> KeccakfCounterInputGen<F> {
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { count: 0, replicas: Vec::new(), mode, _phantom: std::marker::PhantomData }
    }

    /// Invocations of the chunk that are the first of their permutation.
    pub fn originals(&self) -> u64 {
        self.count - self.replicas.len() as u64
    }

    #[inline(always)]
    pub fn process_data<P: MemProcessor>(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        mem_processors: &mut P,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] as u32 != ZiskOperationType::Keccak as u32 {
            return true;
        }

        let step_main = data[STEP];
        let addr_main = data[B];

        match self.mode {
            BusDeviceMode::Counter => {
                Metrics::measure(self, data);
                <KeccakfSM<F> as PrecompileMemInputs>::generate(
                    addr_main,
                    step_main,
                    data,
                    true,
                    mem_processors,
                );
            }
            BusDeviceMode::CounterAsm => {
                Metrics::measure(self, data);
            }
            BusDeviceMode::InputGenerator => {
                if <KeccakfSM<F> as PrecompileMemInputs>::should_skip(
                    addr_main,
                    data,
                    mem_processors,
                ) {
                    return true;
                }
                <KeccakfSM<F> as PrecompileMemInputs>::generate(
                    addr_main,
                    step_main,
                    data,
                    false,
                    mem_processors,
                );
            }
        }

        true
    }
}

impl<F: PrimeField64> Metrics for KeccakfCounterInputGen<F> {
    #[inline(always)]
    fn measure(&mut self, data: &[u64]) {
        self.count += 1;
        let dedup_word = data[DEDUP_WORD];
        if keccakf_dedup_is_replica(dedup_word) {
            self.replicas.push(keccakf_dedup_index(dedup_word) as u32);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<F: PrimeField64> BusDevice<u64> for KeccakfCounterInputGen<F> {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

// ============================================================================
// Plan metadata
// ============================================================================

/// What a plan of the family carries to its instance.
pub struct KeccakfPlanMeta {
    /// Per chunk, how many invocations the instance collects and how many it skips first. A
    /// `Keccakf` instance counts only the invocations that are not replicas.
    pub windows: HashMap<ChunkId, (u64, CollectSkipper)>,
    /// `KeccakfProxy` only: replicas of every unique permutation of the execution, indexed by
    /// the index of the unique permutation. Shared by all the proxy instances.
    pub replicas: Option<Arc<Vec<u32>>>,
}

// ============================================================================
// Collector — gathers the invocations of one chunk for one instance.
// ============================================================================

pub struct KeccakfCollector {
    pub inputs: Vec<KeccakfInput>,
    num_operations: u64,
    collect_skipper: CollectSkipper,
    /// True for a `Keccakf` instance: replicas are not its business, it neither collects nor
    /// counts them.
    skip_replicas: bool,
}

impl KeccakfCollector {
    pub fn new(num_operations: u64, collect_skipper: CollectSkipper, skip_replicas: bool) -> Self {
        Self {
            inputs: Vec::with_capacity(num_operations as usize),
            num_operations,
            collect_skipper,
            skip_replicas,
        }
    }

    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[PayloadType]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return false;
        }

        if data[OP_TYPE] as u32 != ZiskOperationType::Keccak as u32 {
            return true;
        }

        if self.skip_replicas && keccakf_dedup_is_replica(data[DEDUP_WORD]) {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("KeccakfCollector: failed to convert bus data");
        match data {
            ExtOperationData::OperationKeccakData(bus_data) => {
                self.inputs.push(KeccakfInput::from(&bus_data));
            }
            _ => panic!("KeccakfCollector: unexpected ExtOperationData variant"),
        }

        self.inputs.len() < self.num_operations as usize
    }
}

impl BusDevice<PayloadType> for KeccakfCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

// ============================================================================
// Planner
// ============================================================================

/// Turns the per-chunk counters into the plans of both airs.
pub struct KeccakfPlanner<F: PrimeField64> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> Default for KeccakfPlanner<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField64> KeccakfPlanner<F> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }

    /// Plans of one air: `counts` invocations per chunk, cut at `capacity` per instance.
    fn plan_air(
        plans: &mut Vec<Plan>,
        air_id: usize,
        counts: &[InstCount],
        capacity: usize,
        replicas: Option<&Arc<Vec<u32>>>,
    ) {
        let total: u64 = counts.iter().map(|c| c.inst_count).sum();
        let instances = total.div_ceil(capacity as u64) as usize;
        let capacities = vec![capacity as u64; instances];
        for (_, check_point, windows) in plan_ladder(counts, &capacities) {
            plans.push(Plan::new(
                ZISK_AIRGROUP_ID,
                air_id,
                None,
                InstanceType::Instance,
                check_point,
                Some(Box::new(KeccakfPlanMeta { windows, replicas: replicas.cloned() })),
            ));
        }
    }
}

impl<F: PrimeField64> Planner for KeccakfPlanner<F> {
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        let mut invocations: Vec<InstCount> = Vec::with_capacity(counters.len());
        let mut originals: Vec<InstCount> = Vec::with_capacity(counters.len());
        let mut num_originals = 0u64;
        for (chunk_id, counter) in counters.iter() {
            let counter = Metrics::as_any(&**counter)
                .downcast_ref::<KeccakfCounterInputGen<F>>()
                .expect("KeccakfPlanner: unexpected counter type");
            invocations.push(InstCount::new(*chunk_id, counter.count));
            originals.push(InstCount::new(*chunk_id, counter.originals()));
            num_originals += counter.originals();
        }

        // Replicas of every unique permutation. The unique permutations are numbered in order of
        // first appearance, so there are as many as originals
        let mut replicas = vec![0u32; num_originals as usize];
        for (_, counter) in counters.iter() {
            let counter =
                Metrics::as_any(&**counter).downcast_ref::<KeccakfCounterInputGen<F>>().unwrap();
            for &index in &counter.replicas {
                let index = index as usize;
                assert!(
                    index < replicas.len(),
                    "KeccakfPlanner: a replica refers to the unique permutation {index}, but \
                     the execution only has {num_originals}"
                );
                replicas[index] += 1;
            }
        }
        let replicas = Arc::new(replicas);

        let mut plans = Vec::new();
        Self::plan_air(&mut plans, KECCAKF_AIR_ID, &originals, KECCAKF_OPS_PER_INSTANCE, None);
        Self::plan_air(
            &mut plans,
            KECCAKF_PROXY_AIR_ID,
            &invocations,
            KECCAKF_PROXY_OPS_PER_INSTANCE,
            Some(&replicas),
        );
        plans
    }
}

// ============================================================================
// Instance
// ============================================================================

pub struct KeccakfInstance<F: PrimeField64> {
    keccakf_sm: Arc<KeccakfSM<F>>,
    keccakf_proxy_sm: Arc<KeccakfProxySM<F>>,
    ictx: InstanceCtx,
}

impl<F: PrimeField64> KeccakfInstance<F> {
    pub fn new(
        keccakf_sm: Arc<KeccakfSM<F>>,
        keccakf_proxy_sm: Arc<KeccakfProxySM<F>>,
        ictx: InstanceCtx,
    ) -> Self {
        assert!(
            KECCAKF_FAMILY_AIR_IDS.contains(&ictx.plan.air_id),
            "KeccakfInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );
        Self { keccakf_sm, keccakf_proxy_sm, ictx }
    }

    fn meta(&self) -> &KeccakfPlanMeta {
        self.ictx
            .plan
            .meta
            .as_ref()
            .and_then(|meta| meta.downcast_ref::<KeccakfPlanMeta>())
            .expect("KeccakfInstance: expected KeccakfPlanMeta in plan.meta")
    }

    pub fn build_keccakf_collector(&self, chunk_id: ChunkId) -> KeccakfCollector {
        let (num_ops, collect_skipper) = self.meta().windows[&chunk_id];
        KeccakfCollector::new(num_ops, collect_skipper, self.ictx.plan.air_id == KECCAKF_AIR_ID)
    }
}

impl<F: PrimeField64> Instance<F> for KeccakfInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> zisk_common::WitnessResult<F> {
        let inputs: Vec<Vec<KeccakfInput>> = collectors
            .into_iter()
            .map(|(_, collector)| collector.as_any().downcast::<KeccakfCollector>().unwrap().inputs)
            .collect();

        if self.ictx.plan.air_id == KECCAKF_PROXY_AIR_ID {
            let replicas = self
                .meta()
                .replicas
                .as_ref()
                .expect("KeccakfInstance: a KeccakfProxy plan carries the replicas table");
            return if packed {
                Ok(Some(self.keccakf_proxy_sm.compute_witness::<
                    KeccakfProxyTraceRowPacked<F>,
                    { KeccakfProxyTrace::<()>::NUM_ROWS },
                    { KeccakfProxyTrace::<()>::AIR_ID },
                >(&inputs, replicas, trace_buffer)?))
            } else {
                Ok(Some(self.keccakf_proxy_sm.compute_witness::<
                    KeccakfProxyTraceRow<F>,
                    { KeccakfProxyTrace::<()>::NUM_ROWS },
                    { KeccakfProxyTrace::<()>::AIR_ID },
                >(&inputs, replicas, trace_buffer)?))
            };
        }

        // With a kernel registered for this air the prover writes cm1 itself, so the buffer
        // carries the kernel's staged inputs instead of a trace. The prover's declaration and
        // the setup's geometry are the ones the commit checks against.
        #[cfg(gpu)]
        {
            let airgroup_id = self.ictx.plan.airgroup_id;
            if let Some(decl) = _pctx.gpu_witness_air(airgroup_id, KECCAKF_AIR_ID) {
                let setup = _sctx.get_setup(airgroup_id, KECCAKF_AIR_ID)?;
                let num_rows = 1usize << setup.stark_info.stark_struct.n_bits;
                let n_cols = setup.stark_info.map_sections_n["cm1"] as usize;
                let (air_instance, _ops) = ::proofman_common::stage_gpu_witness::<
                    F,
                    crate::keccakf_gpu::GpuOp,
                    _,
                >(
                    decl, num_rows, n_cols, trace_buffer, &inputs
                )?;
                return Ok(Some(air_instance));
            }
        }

        if packed {
            Ok(Some(self.keccakf_sm.compute_witness::<
                KeccakfTraceRowPacked<F>,
                { KeccakfTrace::<()>::NUM_ROWS },
                { KeccakfTrace::<()>::AIR_ID },
            >(_sctx, &inputs, trace_buffer)?))
        } else {
            Ok(Some(self.keccakf_sm.compute_witness::<
                KeccakfTraceRow<F>,
                { KeccakfTrace::<()>::NUM_ROWS },
                { KeccakfTrace::<()>::AIR_ID },
            >(_sctx, &inputs, trace_buffer)?))
        }
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn stats_type(&self) -> StatsType {
        StatsType::Precompiled
    }

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(self.build_keccakf_collector(chunk_id)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Manager — plan/build wiring.
// ============================================================================

pub struct KeccakfManager<F: PrimeField64> {
    keccakf_sm: Arc<KeccakfSM<F>>,
    keccakf_proxy_sm: Arc<KeccakfProxySM<F>>,
}

impl<F: PrimeField64> KeccakfManager<F> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { keccakf_sm: KeccakfSM::new(), keccakf_proxy_sm: KeccakfProxySM::new() })
    }
}

impl<F: PrimeField64> ComponentPlanBuilder<F> for KeccakfManager<F> {
    type Counter = KeccakfCounterInputGen<F>;

    fn counter(is_asm_emulator: bool) -> Self::Counter {
        let mode = if is_asm_emulator { BusDeviceMode::CounterAsm } else { BusDeviceMode::Counter };
        KeccakfCounterInputGen::new(mode)
    }

    fn planner(_is_asm_emulator: bool) -> Box<dyn Planner> {
        Box::new(KeccakfPlanner::<F>::new())
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for KeccakfManager<F> {
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        Box::new(KeccakfInstance::new(self.keccakf_sm.clone(), self.keccakf_proxy_sm.clone(), ictx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_core::KECCAKF_DEDUP_REPLICA_FLAG;

    type F = Goldilocks;

    /// Counter of a chunk with `originals` first invocations and one replica per entry of
    /// `replicas`, each naming the unique permutation it repeats.
    fn counter(originals: u64, replicas: &[u32]) -> Box<dyn BusDeviceMetrics> {
        let mut counter = KeccakfCounterInputGen::<F>::new(BusDeviceMode::CounterAsm);
        counter.count = originals + replicas.len() as u64;
        counter.replicas = replicas.to_vec();
        Box::new(counter)
    }

    fn meta(plan: &Plan) -> &KeccakfPlanMeta {
        plan.meta.as_ref().unwrap().downcast_ref::<KeccakfPlanMeta>().unwrap()
    }

    fn collected(plans: &[&Plan]) -> u64 {
        plans.iter().flat_map(|plan| meta(plan).windows.values()).map(|(count, _)| count).sum()
    }

    #[test]
    fn planner_sizes_keccakf_on_originals_and_the_proxy_on_every_invocation() {
        // Three chunks, more originals than one Keccakf instance holds
        let cap = KECCAKF_OPS_PER_INSTANCE as u64;
        let counters = vec![
            (ChunkId(0), counter(cap - 10, &[0, 0, 5])),
            (ChunkId(1), counter(20, &[5, cap as u32])),
            (ChunkId(2), counter(0, &[0])),
        ];
        let plans = KeccakfPlanner::<F>::new().plan(counters);

        let keccakf: Vec<&Plan> = plans.iter().filter(|p| p.air_id == KECCAKF_AIR_ID).collect();
        let proxy: Vec<&Plan> = plans.iter().filter(|p| p.air_id == KECCAKF_PROXY_AIR_ID).collect();
        assert_eq!(keccakf.len(), 2);
        assert_eq!(proxy.len(), 1);
        assert_eq!(collected(&keccakf), cap + 10);
        assert_eq!(collected(&proxy), cap + 10 + 6);

        // The second Keccakf instance starts 10 originals into chunk 1
        let (count, skipper) = meta(keccakf[1]).windows[&ChunkId(1)];
        assert_eq!((count, skipper.skip), (10, 10));
        assert!(keccakf.iter().all(|plan| meta(plan).replicas.is_none()));

        let replicas = meta(proxy[0]).replicas.as_ref().unwrap();
        assert_eq!(replicas.len() as u64, cap + 10);
        assert_eq!((replicas[0], replicas[5], replicas[cap as usize]), (3, 2, 1));
        assert_eq!(replicas.iter().map(|&r| r as u64).sum::<u64>(), 6);
    }

    #[test]
    fn planner_without_keccaks_plans_nothing() {
        let counters = vec![(ChunkId(0), counter(0, &[]))];
        assert!(KeccakfPlanner::<F>::new().plan(counters).is_empty());
    }

    /// Bus payload of a Keccak-f with a recognizable step and the given dedup word.
    fn payload(step: u64, dedup_word: u64) -> Vec<u64> {
        let mut data = vec![0u64; DEDUP_WORD + 1];
        data[OP_TYPE] = ZiskOperationType::Keccak as u64;
        data[zisk_common::OP] = zisk_core::zisk_ops::ZiskOp::Keccak.code() as u64;
        data[STEP] = step;
        data[DEDUP_WORD] = dedup_word;
        data
    }

    #[test]
    fn keccakf_collector_neither_collects_nor_counts_replicas() {
        let replica = KECCAKF_DEDUP_REPLICA_FLAG;
        let words = [0, replica, 1, 1 | replica, 2, 3, replica];

        // Skip one original, collect two: steps 2 and 4
        let mut collector = KeccakfCollector::new(2, CollectSkipper::new(1), true);
        for (step, word) in words.iter().enumerate() {
            collector.process_data(&OPERATION_BUS_ID, &payload(step as u64, *word));
        }
        let steps: Vec<u64> = collector.inputs.iter().map(|input| input.step_main).collect();
        assert_eq!(steps, [2, 4]);

        // The proxy takes them as they come: skip one invocation, collect three
        let mut collector = KeccakfCollector::new(3, CollectSkipper::new(1), false);
        for (step, word) in words.iter().enumerate() {
            collector.process_data(&OPERATION_BUS_ID, &payload(step as u64, *word));
        }
        let steps: Vec<u64> = collector.inputs.iter().map(|input| input.step_main).collect();
        assert_eq!(steps, [1, 2, 3]);
        assert!(keccakf_dedup_is_replica(collector.inputs[0].dedup_word));
    }
}
