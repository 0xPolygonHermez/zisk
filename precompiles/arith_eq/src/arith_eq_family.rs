//! Hand-written precompile family for the multi-air `ArithEq`.
//!
//! Replaces the single-air `zisk_precompile!(ArithEq)` shell. One shared `ArithEqSM` serves every
//! config air; a single counter/planner routes each sub-operation to the cheapest covering air
//! (see [`crate::plan_air_strategy`]), and each per-air instance computes its witness via
//! [`ArithEqSM::compute_witness_for_air`]. Modeled on the `mem_align` family.
//!
//! The four public type names (`ArithEqManager`, `ArithEqCounterInputGen`, `ArithEqInstance`,
//! `ArithEqCollector`) match what `executor`'s `register_precompiles!` expects.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use precompiles_common::{MemProcessor, PrecompileMemInputs};
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use zisk_common::{
    BusDevice, BusDeviceMetrics, BusDeviceMode, BusId, CheckPoint, ChunkId, CollectCounter,
    ComponentBuilder, ComponentPlanBuilder, ExtOperationData, Instance, InstanceCtx, InstanceType,
    Metrics, PayloadType, Plan, Planner, SegmentId, StatsType, B, OP, OPERATION_BUS_ID, OP_TYPE,
    STEP,
};
use zisk_core::ZiskOperationType;
use zisk_pil::ZISK_AIRGROUP_ID;

use crate::{
    air_metas, arith_eq_air_ids, plan_air_strategy, Arith256Input, Arith256ModInput, ArithEqInput,
    ArithEqOp, ArithEqSM, Bn254ComplexAddInput, Bn254ComplexMulInput, Bn254ComplexSubInput,
    Bn254CurveAddInput, Bn254CurveDblInput, Secp256k1AddInput, Secp256k1DblInput,
    Secp256r1AddInput, Secp256r1DblInput, ARITH_EQ_OP_NUM, ARITH_EQ_ROWS_BY_OP,
};

// ============================================================================
// CheckPoint — per (instance, chunk): how many of each sub-op this instance collects.
// ============================================================================

/// Per-chunk collection window for one `ArithEq` instance: for each sub-op, the `CollectCounter`
/// (skip, count) that its collector applies when re-scanning the bus.
#[derive(Debug, Clone)]
pub struct ArithEqCheckPoint {
    pub air_id: usize,
    pub chunk_id: ChunkId,
    pub ops: [CollectCounter; ARITH_EQ_OP_NUM],
}

impl ArithEqCheckPoint {
    fn new(air_id: usize, chunk_id: ChunkId) -> Self {
        Self { air_id, chunk_id, ops: [CollectCounter::new(0, 0); ARITH_EQ_OP_NUM] }
    }

    pub fn count(&self) -> u32 {
        self.ops.iter().map(|c| c.count()).sum()
    }
}

// ============================================================================
// CounterInputGen — per-op counting + mem-input generation.
// ============================================================================

/// Counts each `ArithEq` sub-operation separately and drives `PrecompileMemInputs`. Used in all
/// three bus modes (`Counter`, `CounterAsm`, `InputGenerator`).
pub struct ArithEqCounterInputGen<F: PrimeField64> {
    /// Per-sub-op occurrence counts, indexed by `ArithEqOp::index`.
    pub counts: [u64; ARITH_EQ_OP_NUM],
    mode: BusDeviceMode,
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> ArithEqCounterInputGen<F> {
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { counts: [0; ARITH_EQ_OP_NUM], mode, _phantom: std::marker::PhantomData }
    }

    #[inline(always)]
    pub fn process_data<P: MemProcessor>(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        mem_processors: &mut P,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] as u32 != ZiskOperationType::ArithEq as u32 {
            return true;
        }

        let step_main = data[STEP];
        let addr_main = data[B] as u32;

        match self.mode {
            BusDeviceMode::Counter => {
                Metrics::measure(self, data);
                <ArithEqSM<F> as PrecompileMemInputs>::generate(
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
                if <ArithEqSM<F> as PrecompileMemInputs>::should_skip(
                    addr_main,
                    data,
                    mem_processors,
                ) {
                    return true;
                }
                <ArithEqSM<F> as PrecompileMemInputs>::generate(
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

impl<F: PrimeField64> Metrics for ArithEqCounterInputGen<F> {
    #[inline(always)]
    fn measure(&mut self, data: &[u64]) {
        if let Some(op) = ArithEqOp::from_opcode(data[OP] as u8) {
            self.counts[op.index()] += 1;
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<F: PrimeField64> BusDevice<u64> for ArithEqCounterInputGen<F> {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

// ============================================================================
// Collector — re-scans the bus, routing each sub-op via its CollectCounter.
// ============================================================================

/// Gathers this instance's `ArithEqInput`s for one chunk: for each sub-op occurrence, applies the
/// checkpoint's `CollectCounter` and pushes the converted input when it falls in the window.
pub struct ArithEqCollector {
    pub inputs: Vec<ArithEqInput>,
    pub air_id: usize,
    pub chunk_id: ChunkId,
    ops: [CollectCounter; ARITH_EQ_OP_NUM],
}

impl ArithEqCollector {
    pub fn new(checkpoint: &ArithEqCheckPoint) -> Self {
        let total: u32 = checkpoint.count();
        Self {
            inputs: Vec::with_capacity(total as usize),
            air_id: checkpoint.air_id,
            chunk_id: checkpoint.chunk_id,
            ops: checkpoint.ops,
        }
    }

    pub fn count(&self) -> u32 {
        self.ops.iter().map(|c| c.count()).sum()
    }

    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        // Gate on OP_TYPE first (like the counter/input-gen path), so a non-ArithEq bus entry can
        // never advance a sub-op window even if it reused an ArithEq opcode value.
        if data[OP_TYPE] as u32 != ZiskOperationType::ArithEq as u32 {
            return true;
        }
        let Some(op) = ArithEqOp::from_opcode(data[OP] as u8) else {
            return true;
        };
        // Advance this sub-op's window; skip unless we're inside the collect range.
        if self.ops[op.index()].should_skip() {
            return true;
        }

        let ext: ExtOperationData<u64> =
            data.try_into().expect("ArithEqCollector: failed to convert bus data");
        let input = match ext {
            ExtOperationData::OperationArith256Data(d) => {
                ArithEqInput::Arith256(Arith256Input::from(&d))
            }
            ExtOperationData::OperationArith256ModData(d) => {
                ArithEqInput::Arith256Mod(Arith256ModInput::from(&d))
            }
            ExtOperationData::OperationSecp256k1AddData(d) => {
                ArithEqInput::Secp256k1Add(Secp256k1AddInput::from(&d))
            }
            ExtOperationData::OperationSecp256k1DblData(d) => {
                ArithEqInput::Secp256k1Dbl(Secp256k1DblInput::from(&d))
            }
            ExtOperationData::OperationBn254CurveAddData(d) => {
                ArithEqInput::Bn254CurveAdd(Bn254CurveAddInput::from(&d))
            }
            ExtOperationData::OperationBn254CurveDblData(d) => {
                ArithEqInput::Bn254CurveDbl(Bn254CurveDblInput::from(&d))
            }
            ExtOperationData::OperationBn254ComplexAddData(d) => {
                ArithEqInput::Bn254ComplexAdd(Bn254ComplexAddInput::from(&d))
            }
            ExtOperationData::OperationBn254ComplexSubData(d) => {
                ArithEqInput::Bn254ComplexSub(Bn254ComplexSubInput::from(&d))
            }
            ExtOperationData::OperationBn254ComplexMulData(d) => {
                ArithEqInput::Bn254ComplexMul(Bn254ComplexMulInput::from(&d))
            }
            ExtOperationData::OperationSecp256r1AddData(d) => {
                ArithEqInput::Secp256r1Add(Secp256r1AddInput::from(&d))
            }
            ExtOperationData::OperationSecp256r1DblData(d) => {
                ArithEqInput::Secp256r1Dbl(Secp256r1DblInput::from(&d))
            }
            _ => return true,
        };
        self.inputs.push(input);
        true
    }
}

impl BusDevice<PayloadType> for ArithEqCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

// ============================================================================
// Planner — cost strategy + per-instance checkpoint filling.
// ============================================================================

/// Turns per-chunk per-op counters into per-air instance plans (one `Plan` per instance, carrying a
/// `HashMap<ChunkId, ArithEqCheckPoint>` as metadata).
pub struct ArithEqPlanner<F: PrimeField64> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> Default for ArithEqPlanner<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField64> ArithEqPlanner<F> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}

impl<F: PrimeField64> Planner for ArithEqPlanner<F> {
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        // Per-chunk per-op counts (in chunk order) + totals.
        let mut per_chunk: Vec<(ChunkId, [u64; ARITH_EQ_OP_NUM])> =
            Vec::with_capacity(counters.len());
        let mut totals = [0u64; ARITH_EQ_OP_NUM];
        for (chunk_id, counter) in counters.iter() {
            let cig = Metrics::as_any(&**counter)
                .downcast_ref::<ArithEqCounterInputGen<F>>()
                .expect("ArithEqPlanner: unexpected counter type");
            for (t, c) in totals.iter_mut().zip(cig.counts.iter()) {
                *t += *c;
            }
            per_chunk.push((*chunk_id, cig.counts));
        }

        if totals.iter().all(|&c| c == 0) {
            return Vec::new();
        }

        // Area-minimizing assignment: per air, how many of each op it proves. The universal air is
        // last, and a single op may be split (its full instances in a specialized air, its remainder
        // pooled into the universal air).
        let strategy = plan_air_strategy(&arith_eq_air_ids(), &totals);
        let metas = air_metas();

        // Global running offset per (chunk, op) across all airs/instances, so a split op's
        // specialized collect windows and its universal collect window never overlap. Specialized
        // airs (processed first) take the earliest occurrences; the universal air takes the rest.
        let mut offset: Vec<[u64; ARITH_EQ_OP_NUM]> =
            vec![[0u64; ARITH_EQ_OP_NUM]; per_chunk.len()];

        let mut plans = Vec::new();
        for air_plan in &strategy {
            let air_id = air_plan.air_id;
            let num_rows = metas.iter().find(|m| m.air_id == air_id).unwrap().num_rows as u64;
            // Operations per instance (uniform: every op consumes ARITH_EQ_ROWS_BY_OP rows).
            let cap = num_rows / ARITH_EQ_ROWS_BY_OP as u64;
            debug_assert!(cap > 0);

            let mut air_budget = air_plan.op_counts;
            let mut cur_fill = 0u64;
            let mut cur_cps: HashMap<ChunkId, ArithEqCheckPoint> = HashMap::new();
            let mut cur_chunks: Vec<ChunkId> = Vec::new();
            let mut segment = 0usize;

            for (chunk_idx, (chunk_id, counts)) in per_chunk.iter().enumerate() {
                for idx in 0..ARITH_EQ_OP_NUM {
                    if air_budget[idx] == 0 {
                        continue;
                    }
                    // Occurrences of this op in this chunk still free after earlier airs/instances.
                    let available = counts[idx].saturating_sub(offset[chunk_idx][idx]);
                    let mut take_total = air_budget[idx].min(available);
                    while take_total > 0 {
                        if cur_fill == cap {
                            Self::close_instance(
                                &mut plans,
                                air_id,
                                &mut segment,
                                &mut cur_cps,
                                &mut cur_chunks,
                            );
                            cur_fill = 0;
                        }
                        let take = take_total.min(cap - cur_fill);
                        if !cur_cps.contains_key(chunk_id) {
                            cur_chunks.push(*chunk_id);
                            cur_cps.insert(*chunk_id, ArithEqCheckPoint::new(air_id, *chunk_id));
                        }
                        cur_cps.get_mut(chunk_id).unwrap().ops[idx] =
                            CollectCounter::new(offset[chunk_idx][idx] as u32, take as u32);
                        offset[chunk_idx][idx] += take;
                        cur_fill += take;
                        air_budget[idx] -= take;
                        take_total -= take;
                    }
                }
            }
            Self::close_instance(&mut plans, air_id, &mut segment, &mut cur_cps, &mut cur_chunks);
        }
        plans
    }
}

impl<F: PrimeField64> ArithEqPlanner<F> {
    fn close_instance(
        plans: &mut Vec<Plan>,
        air_id: usize,
        segment: &mut usize,
        cur_cps: &mut HashMap<ChunkId, ArithEqCheckPoint>,
        cur_chunks: &mut Vec<ChunkId>,
    ) {
        if cur_cps.is_empty() {
            return;
        }
        let checkpoints = std::mem::take(cur_cps);
        let chunks = std::mem::take(cur_chunks);
        let plan = Plan::new(
            ZISK_AIRGROUP_ID,
            air_id,
            Some(SegmentId(*segment)),
            InstanceType::Instance,
            CheckPoint::Multiple(chunks),
            Some(Box::new(checkpoints)),
        );
        *segment += 1;
        plans.push(plan);
    }
}

// ============================================================================
// Instance — computes the witness for its air via the shared SM.
// ============================================================================

pub struct ArithEqInstance<F: PrimeField64> {
    ictx: InstanceCtx,
    checkpoint: HashMap<ChunkId, ArithEqCheckPoint>,
    arith_eq_sm: Arc<ArithEqSM<F>>,
}

impl<F: PrimeField64> ArithEqInstance<F> {
    pub fn new(arith_eq_sm: Arc<ArithEqSM<F>>, mut ictx: InstanceCtx) -> Self {
        let meta = ictx.plan.meta.take().expect("ArithEqInstance: expected metadata in plan.meta");
        let checkpoint = *meta
            .downcast::<HashMap<ChunkId, ArithEqCheckPoint>>()
            .expect("ArithEqInstance: failed to downcast plan.meta");
        Self { ictx, checkpoint, arith_eq_sm }
    }

    pub fn build_arith_eq_collector(&self, chunk_id: ChunkId) -> ArithEqCollector {
        ArithEqCollector::new(&self.checkpoint[&chunk_id])
    }
}

impl<F: PrimeField64> Instance<F> for ArithEqInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let inputs: Vec<Vec<ArithEqInput>> = collectors
            .into_iter()
            .map(|(_, collector)| collector.as_any().downcast::<ArithEqCollector>().unwrap().inputs)
            .collect();

        Ok(Some(self.arith_eq_sm.compute_witness_for_air(
            self.ictx.plan.air_id,
            sctx,
            &inputs,
            trace_buffer,
            packed,
        )?))
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
        Some(Box::new(ArithEqCollector::new(&self.checkpoint[&chunk_id])))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Manager — plan/build wiring.
// ============================================================================

pub struct ArithEqManager<F: PrimeField64> {
    arith_eq_sm: Arc<ArithEqSM<F>>,
}

impl<F: PrimeField64> ArithEqManager<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { arith_eq_sm: ArithEqSM::new(std) })
    }
}

impl<F: PrimeField64> ComponentPlanBuilder<F> for ArithEqManager<F> {
    type Counter = ArithEqCounterInputGen<F>;

    fn counter(is_asm_emulator: bool) -> Self::Counter {
        let mode = if is_asm_emulator { BusDeviceMode::CounterAsm } else { BusDeviceMode::Counter };
        ArithEqCounterInputGen::new(mode)
    }

    fn planner(_is_asm_emulator: bool) -> Box<dyn Planner> {
        Box::new(ArithEqPlanner::<F>::new())
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for ArithEqManager<F> {
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        Box::new(ArithEqInstance::new(self.arith_eq_sm.clone(), ictx))
    }
}
