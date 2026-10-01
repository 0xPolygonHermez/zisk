//! Hand-written precompile family for `ArithEq384`.
//!
//! Replaces the `zisk_precompile!` shell, whose planner sizes every air of a precompile as one
//! ladder proving the same operations. Here there are two ladders: the little-endian airs
//! (`ArithEq384`, `ArithEq384Large`) prove the little-endian opcodes and the `big_endian: 1` twins
//! (`ArithEq384Be`, `ArithEq384BeLarge`) prove the `OP_*_BE` ones, so the operations are counted,
//! planned and collected per endianness and each ladder is sized on its own with the shared
//! `select_sizes` / `plan_ladder` helpers, exactly as the macro does for one ladder. One shared
//! `ArithEq384SM` computes the witness of every air; the row type selects the air.
//!
//! The four public type names (`ArithEq384Manager`, `ArithEq384CounterInputGen`,
//! `ArithEq384Instance`, `ArithEq384Collector`) match what `executor`'s `register_precompiles!`
//! expects.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use pil2_std_lib::Std;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use zisk_common::{
    plan_ladder, select_sizes, AirChoice, BusDevice, BusDeviceMetrics, BusDeviceMode, BusId,
    CheckPoint, ChunkId, CollectSkipper, ComponentBuilder, ComponentPlanBuilder, ExtOperationData,
    InstCount, Instance, InstanceCtx, InstanceType, Metrics, PayloadType, Plan, Planner, StatsType,
    B, OP, OPERATION_BUS_ID, OP_TYPE, STEP,
};
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::ZiskOperationType;
use zisk_pil::{
    ArithEq384BeLargeTrace, ArithEq384BeLargeTraceRow, ArithEq384BeLargeTraceRowPacked,
    ArithEq384BeTrace, ArithEq384BeTraceRow, ArithEq384BeTraceRowPacked, ArithEq384LargeTrace,
    ArithEq384LargeTraceRow, ArithEq384LargeTraceRowPacked, ArithEq384Trace, ArithEq384TraceRow,
    ArithEq384TraceRowPacked, ARITH_EQ_384_BE_INSTANCE_COST, ARITH_EQ_384_BE_LARGE_INSTANCE_COST,
    ARITH_EQ_384_INSTANCE_COST, ARITH_EQ_384_LARGE_INSTANCE_COST, ZISK_AIRGROUP_ID,
};
use zisk_precomp_common::{MemProcessor, PrecompileMemInputs};

use crate::{
    arith_eq_384_ops_per_instance, Arith384ModInput, ArithEq384Input, ArithEq384SM,
    Bls12_381ComplexAddInput, Bls12_381ComplexMulInput, Bls12_381ComplexSubInput,
    Bls12_381CurveAddInput, Bls12_381CurveDblInput,
};

/// Endianness of an `ArithEq384` opcode: `Some(false)` for a little-endian op, `Some(true)` for its
/// big-endian twin (operands stored in memory as big-endian integers), `None` for any other opcode.
#[inline]
pub fn arith_eq_384_op_is_big_endian(op: u8) -> Option<bool> {
    match op {
        ZiskOp::ARITH384_MOD
        | ZiskOp::BLS12_381_CURVE_ADD
        | ZiskOp::BLS12_381_CURVE_DBL
        | ZiskOp::BLS12_381_COMPLEX_ADD
        | ZiskOp::BLS12_381_COMPLEX_SUB
        | ZiskOp::BLS12_381_COMPLEX_MUL => Some(false),
        ZiskOp::ARITH384_MOD_BE
        | ZiskOp::BLS12_381_CURVE_ADD_BE
        | ZiskOp::BLS12_381_CURVE_DBL_BE
        | ZiskOp::BLS12_381_COMPLEX_ADD_BE
        | ZiskOp::BLS12_381_COMPLEX_SUB_BE
        | ZiskOp::BLS12_381_COMPLEX_MUL_BE => Some(true),
        _ => None,
    }
}

/// Static description of one `ArithEq384` air.
#[derive(Clone, Copy, Debug)]
pub struct ArithEq384AirMeta {
    /// Air id in the ZisK air group (matches `<Alias>Trace::AIR_ID`).
    pub air_id: usize,
    /// Whether it is a `big_endian: 1` air.
    pub big_endian: bool,
    /// Rows per instance.
    pub num_rows: usize,
    /// Area of one instance of this air, full or not (MB of prover memory).
    pub cost: usize,
}

/// One entry per air alias instantiated in `pil/zisk.pil`: each endianness at both heights.
/// `num_rows` is read from the trace types and the cost from that air's `*_INSTANCE_COST` constant.
pub fn air_metas() -> [ArithEq384AirMeta; 4] {
    macro_rules! meta {
        ($trace:ident, $big_endian:expr, $cost:ident) => {
            ArithEq384AirMeta {
                air_id: $trace::<()>::AIR_ID,
                big_endian: $big_endian,
                num_rows: $trace::<()>::NUM_ROWS,
                cost: $cost,
            }
        };
    }
    [
        meta!(ArithEq384Trace, false, ARITH_EQ_384_INSTANCE_COST),
        meta!(ArithEq384LargeTrace, false, ARITH_EQ_384_LARGE_INSTANCE_COST),
        meta!(ArithEq384BeTrace, true, ARITH_EQ_384_BE_INSTANCE_COST),
        meta!(ArithEq384BeLargeTrace, true, ARITH_EQ_384_BE_LARGE_INSTANCE_COST),
    ]
}

/// Air ids of every `ArithEq384` air instantiated in `pil/zisk.pil`, from the trace `AIR_ID` consts
/// so it cannot drift: the executor registers the family on them.
pub const ARITH_EQ_384_CONFIG_AIR_IDS: &[usize] = &[
    ArithEq384Trace::<()>::AIR_ID,
    ArithEq384LargeTrace::<()>::AIR_ID,
    ArithEq384BeTrace::<()>::AIR_ID,
    ArithEq384BeLargeTrace::<()>::AIR_ID,
];

/// Per-instance metadata: which endianness it proves and, per chunk, how many of its operations
/// this instance collects after skipping how many (the `plan_ladder` windows).
#[derive(Debug, Clone)]
pub struct ArithEq384CollectInfo {
    pub big_endian: bool,
    pub windows: HashMap<ChunkId, (u64, CollectSkipper)>,
}

// ============================================================================
// CounterInputGen — per-endianness counting + mem-input generation.
// ============================================================================

/// Counts the little-endian and the big-endian `ArithEq384` operations separately and drives
/// `PrecompileMemInputs`. Used in all three bus modes (`Counter`, `CounterAsm`, `InputGenerator`).
pub struct ArithEq384CounterInputGen<F: PrimeField64> {
    /// Occurrences: `counts[0]` little-endian ops, `counts[1]` big-endian ops.
    pub counts: [u64; 2],
    mode: BusDeviceMode,
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> ArithEq384CounterInputGen<F> {
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { counts: [0; 2], mode, _phantom: std::marker::PhantomData }
    }

    /// Operations of the given endianness counted so far.
    #[inline]
    pub fn count(&self, big_endian: bool) -> u64 {
        self.counts[big_endian as usize]
    }

    #[inline(always)]
    pub fn process_data<P: MemProcessor>(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        mem_processors: &mut P,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] as u32 != ZiskOperationType::ArithEq384 as u32 {
            return true;
        }

        let step_main = data[STEP];
        let addr_main = data[B] as u32;

        match self.mode {
            BusDeviceMode::Counter => {
                Metrics::measure(self, data);
                <ArithEq384SM<F> as PrecompileMemInputs>::generate(
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
                if <ArithEq384SM<F> as PrecompileMemInputs>::should_skip(
                    addr_main,
                    data,
                    mem_processors,
                ) {
                    return true;
                }
                <ArithEq384SM<F> as PrecompileMemInputs>::generate(
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

impl<F: PrimeField64> Metrics for ArithEq384CounterInputGen<F> {
    #[inline(always)]
    fn measure(&mut self, data: &[u64]) {
        if let Some(big_endian) = arith_eq_384_op_is_big_endian(data[OP] as u8) {
            self.counts[big_endian as usize] += 1;
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<F: PrimeField64> BusDevice<u64> for ArithEq384CounterInputGen<F> {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

// ============================================================================
// Collector — gathers one instance's inputs of its endianness for one chunk.
// ============================================================================

pub struct ArithEq384Collector {
    pub inputs: Vec<ArithEq384Input>,
    num_operations: u64,
    collect_skipper: CollectSkipper,
    big_endian: bool,
}

impl ArithEq384Collector {
    pub fn new(num_operations: u64, collect_skipper: CollectSkipper, big_endian: bool) -> Self {
        Self {
            inputs: Vec::with_capacity(num_operations as usize),
            num_operations,
            collect_skipper,
            big_endian,
        }
    }

    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[PayloadType]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return false;
        }

        if data[OP_TYPE] as u32 != ZiskOperationType::ArithEq384 as u32 {
            return true;
        }

        // Ops of the other endianness belong to the other ladder: they neither count towards this
        // instance's window nor advance its skipper (the planner counted them apart).
        let Some(big_endian) = arith_eq_384_op_is_big_endian(data[OP] as u8) else {
            return true;
        };
        if big_endian != self.big_endian {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        let ext: ExtOperationData<u64> =
            data.try_into().expect("ArithEq384Collector: failed to convert bus data");
        // A big-endian op shares the bus layout and the witness path of its little-endian twin;
        // `from_bus` converts its memory-image operands to the limbs the executors work on.
        let input = match ext {
            ExtOperationData::OperationArith384ModData(d) => {
                ArithEq384Input::Arith384Mod(Arith384ModInput::from_bus(&d, big_endian))
            }
            ExtOperationData::OperationBls12_381CurveAddData(d) => {
                ArithEq384Input::Bls12_381CurveAdd(Bls12_381CurveAddInput::from_bus(&d, big_endian))
            }
            ExtOperationData::OperationBls12_381CurveDblData(d) => {
                ArithEq384Input::Bls12_381CurveDbl(Bls12_381CurveDblInput::from_bus(&d, big_endian))
            }
            ExtOperationData::OperationBls12_381ComplexAddData(d) => {
                ArithEq384Input::Bls12_381ComplexAdd(Bls12_381ComplexAddInput::from_bus(
                    &d, big_endian,
                ))
            }
            ExtOperationData::OperationBls12_381ComplexSubData(d) => {
                ArithEq384Input::Bls12_381ComplexSub(Bls12_381ComplexSubInput::from_bus(
                    &d, big_endian,
                ))
            }
            ExtOperationData::OperationBls12_381ComplexMulData(d) => {
                ArithEq384Input::Bls12_381ComplexMul(Bls12_381ComplexMulInput::from_bus(
                    &d, big_endian,
                ))
            }
            _ => panic!("ArithEq384Collector: unexpected ExtOperationData variant"),
        };
        self.inputs.push(input);

        self.inputs.len() < self.num_operations as usize
    }
}

impl BusDevice<PayloadType> for ArithEq384Collector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

// ============================================================================
// Planner — one ladder per endianness.
// ============================================================================

pub struct ArithEq384Planner<F: PrimeField64> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> Default for ArithEq384Planner<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField64> ArithEq384Planner<F> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }

    /// Plans the operations of one endianness on that endianness' ladder: the same sizing the
    /// generic precompile planner applies to a single-ladder precompile.
    fn plan_ladder_of(big_endian: bool, counts: &[InstCount], plans: &mut Vec<Plan>) {
        let total: u64 = counts.iter().map(|c| c.inst_count).sum();
        if total == 0 {
            return;
        }

        // Capacity and memory of each air of the ladder, so the sizing can trade one against the
        // other.
        let ladder: Vec<AirChoice> = air_metas()
            .iter()
            .filter(|m| m.big_endian == big_endian)
            .map(|m| AirChoice {
                airgroup_id: ZISK_AIRGROUP_ID,
                air_id: m.air_id,
                rows: arith_eq_384_ops_per_instance(m.num_rows) as u64,
                memory: m.cost as u64,
            })
            .collect();

        let granted = select_sizes(total, &ladder);

        // Fill the roomiest instances first, so the tail is what lands in the small one the sizing
        // demoted.
        let mut order: Vec<usize> = (0..ladder.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(ladder[i].rows));
        let mut instance_air: Vec<usize> = Vec::new();
        let mut capacities: Vec<u64> = Vec::new();
        for air in order {
            for _ in 0..granted[air] {
                instance_air.push(air);
                capacities.push(ladder[air].rows);
            }
        }

        plans.extend(plan_ladder(counts, &capacities).into_iter().map(
            |(instance, check_point, windows)| {
                let air = ladder[instance_air[instance]];
                Plan::new(
                    air.airgroup_id,
                    air.air_id,
                    None,
                    InstanceType::Instance,
                    check_point,
                    Some(Box::new(ArithEq384CollectInfo { big_endian, windows })),
                )
            },
        ));
    }
}

impl<F: PrimeField64> Planner for ArithEq384Planner<F> {
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        let mut plans = Vec::new();
        for big_endian in [false, true] {
            let counts: Vec<InstCount> = counters
                .iter()
                .map(|(chunk_id, counter)| {
                    let cig = Metrics::as_any(&**counter)
                        .downcast_ref::<ArithEq384CounterInputGen<F>>()
                        .expect("ArithEq384Planner: unexpected counter type");
                    InstCount::new(*chunk_id, cig.count(big_endian))
                })
                .collect();
            Self::plan_ladder_of(big_endian, &counts, &mut plans);
        }
        plans
    }
}

// ============================================================================
// Instance — computes the witness for its air via the shared SM.
// ============================================================================

pub struct ArithEq384Instance<F: PrimeField64> {
    ictx: InstanceCtx,
    arith_eq_384_sm: Arc<ArithEq384SM<F>>,
}

impl<F: PrimeField64> ArithEq384Instance<F> {
    pub fn new(arith_eq_384_sm: Arc<ArithEq384SM<F>>, ictx: InstanceCtx) -> Self {
        Self { ictx, arith_eq_384_sm }
    }

    fn collect_info(&self) -> &ArithEq384CollectInfo {
        self.ictx
            .plan
            .meta
            .as_ref()
            .expect("ArithEq384Instance: expected metadata in plan.meta")
            .downcast_ref::<ArithEq384CollectInfo>()
            .expect("ArithEq384Instance: failed to downcast plan.meta")
    }

    pub fn build_arith_eq384_collector(&self, chunk_id: ChunkId) -> ArithEq384Collector {
        let info = self.collect_info();
        let (num_ops, collect_skipper) = info.windows[&chunk_id];
        ArithEq384Collector::new(num_ops, collect_skipper, info.big_endian)
    }
}

impl<F: PrimeField64> Instance<F> for ArithEq384Instance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let inputs: Vec<Vec<ArithEq384Input>> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<ArithEq384Collector>().unwrap().inputs
            })
            .collect();

        // Air-id ↔ row type: the single place that maps a runtime `air_id` to its concrete trace
        // row (and so to its height and endianness); the generic `compute_witness` does the rest.
        macro_rules! dispatch {
            ( $( $alias:ident : $row:ident / $row_packed:ident ),+ $(,)? ) => {
                match self.ictx.plan.air_id {
                    $(
                        id if id == $alias::<()>::AIR_ID => {
                            if packed {
                                self.arith_eq_384_sm
                                    .compute_witness::<$row_packed<F>>(sctx, &inputs, trace_buffer)
                            } else {
                                self.arith_eq_384_sm
                                    .compute_witness::<$row<F>>(sctx, &inputs, trace_buffer)
                            }
                        }
                    )+
                    id => panic!("ArithEq384Instance: unsupported air_id {id}"),
                }
            };
        }
        let instance = dispatch!(
            ArithEq384Trace: ArithEq384TraceRow / ArithEq384TraceRowPacked,
            ArithEq384LargeTrace: ArithEq384LargeTraceRow / ArithEq384LargeTraceRowPacked,
            ArithEq384BeTrace: ArithEq384BeTraceRow / ArithEq384BeTraceRowPacked,
            ArithEq384BeLargeTrace: ArithEq384BeLargeTraceRow / ArithEq384BeLargeTraceRowPacked,
        )?;
        Ok(Some(instance))
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
        Some(Box::new(self.build_arith_eq384_collector(chunk_id)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Manager — plan/build wiring.
// ============================================================================

pub struct ArithEq384Manager<F: PrimeField64> {
    arith_eq_384_sm: Arc<ArithEq384SM<F>>,
}

impl<F: PrimeField64> ArithEq384Manager<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { arith_eq_384_sm: ArithEq384SM::new(std) })
    }
}

impl<F: PrimeField64> ComponentPlanBuilder<F> for ArithEq384Manager<F> {
    type Counter = ArithEq384CounterInputGen<F>;

    fn counter(is_asm_emulator: bool) -> Self::Counter {
        let mode = if is_asm_emulator { BusDeviceMode::CounterAsm } else { BusDeviceMode::Counter };
        ArithEq384CounterInputGen::new(mode)
    }

    fn planner(_is_asm_emulator: bool) -> Box<dyn Planner> {
        Box::new(ArithEq384Planner::<F>::new())
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for ArithEq384Manager<F> {
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        assert!(
            ARITH_EQ_384_CONFIG_AIR_IDS.contains(&ictx.plan.air_id),
            "ArithEq384Manager::build_instance() Unsupported air_id: {:?}",
            ictx.plan.air_id
        );
        Box::new(ArithEq384Instance::new(self.arith_eq_384_sm.clone(), ictx))
    }
}
