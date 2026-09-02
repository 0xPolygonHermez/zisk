//! Per-config air metadata for cost-based planning.
//!
//! Each `ArithEq` config air (one alias in `pil/zisk.pil`) covers a fixed set of sub-operations and
//! has a fixed per-row column cost (`ROW_SIZE`) and capacity (`NUM_ROWS`). The planner counts each
//! operation separately, then — over only the airs actually present in the pilout — assigns each
//! operation to the cheapest air that covers it (mem_align-style: most efficient first).
//!
//! This table is the planner's static input; it is derived from the generated column layout in
//! `zisk_pil` and the `equations` bitmask each alias was instantiated with. `row_size` / `num_rows`
//! are read from the trace types so cost stays in sync with the PIL automatically.

use crate::{ArithEqInput, ArithEqOp, ArithEqSM};
use proofman_common::trace::TraceRow;
use proofman_common::{AirInstance, ProofmanResult, SetupCtx};
use proofman_fields::{Goldilocks, PrimeField64};
use zisk_pil::*;

/// Static description of one `ArithEq` config air.
#[derive(Clone, Copy)]
pub struct ArithEqAirMeta {
    /// Air id in the ZisK air group (matches `<Alias>Trace::AIR_ID`).
    pub air_id: usize,
    /// Sub-operations this air can prove.
    pub ops: &'static [ArithEqOp],
    /// Committed columns per row (cost driver).
    pub row_size: usize,
    /// Rows per instance (capacity).
    pub num_rows: usize,
}

impl ArithEqAirMeta {
    /// Cost of one full instance of this air, in committed field elements.
    #[inline]
    pub fn instance_cost(&self) -> usize {
        self.row_size * self.num_rows
    }

    /// Whether this air covers `op`.
    #[inline]
    pub fn covers(&self, op: ArithEqOp) -> bool {
        self.ops.contains(&op)
    }
}

/// One entry per air alias instantiated in `pil/zisk.pil`, cheapest/most-specific first so the
/// planner can greedily prefer efficient airs. `row_size`/`num_rows` come from the trace types.
///
/// NOTE: keep this in sync with the aliases in `pil/zisk.pil`. Airs whose id is absent from the
/// pilout at runtime (`ARITH_EQ_AIR_IDS`) are simply skipped by the planner.
pub fn air_metas() -> Vec<ArithEqAirMeta> {
    use ArithEqOp::*;
    vec![
        // AIR_ID 1 — arith256 only.
        ArithEqAirMeta {
            air_id: Arith256Trace::<()>::AIR_ID,
            ops: &[Arith256],
            row_size: Arith256TraceRow::<Goldilocks>::ROW_SIZE,
            num_rows: Arith256Trace::<()>::NUM_ROWS,
        },
        // AIR_ID 2 — arith256 + arith256_mod.
        ArithEqAirMeta {
            air_id: Arith256XTrace::<()>::AIR_ID,
            ops: &[Arith256, Arith256Mod],
            row_size: Arith256XTraceRow::<Goldilocks>::ROW_SIZE,
            num_rows: Arith256XTrace::<()>::NUM_ROWS,
        },
        // AIR_ID 3 — secp256k1 add/dbl.
        ArithEqAirMeta {
            air_id: ArithSecp256K1Trace::<()>::AIR_ID,
            ops: &[Secp256k1Add, Secp256k1Dbl],
            row_size: ArithSecp256K1TraceRow::<Goldilocks>::ROW_SIZE,
            num_rows: ArithSecp256K1Trace::<()>::NUM_ROWS,
        },
        // AIR_ID 4 — bn254 EC add/dbl.
        ArithEqAirMeta {
            air_id: ArithBn254EcTrace::<()>::AIR_ID,
            ops: &[Bn254CurveAdd, Bn254CurveDbl],
            row_size: ArithBn254EcTraceRow::<Goldilocks>::ROW_SIZE,
            num_rows: ArithBn254EcTrace::<()>::NUM_ROWS,
        },
        // AIR_ID 5 — bn254 complex add/sub/mul.
        ArithEqAirMeta {
            air_id: ArithBn254ComplexTrace::<()>::AIR_ID,
            ops: &[Bn254ComplexAdd, Bn254ComplexSub, Bn254ComplexMul],
            row_size: ArithBn254ComplexTraceRow::<Goldilocks>::ROW_SIZE,
            num_rows: ArithBn254ComplexTrace::<()>::NUM_ROWS,
        },
        // AIR_ID 0 — full air (fallback: covers every operation).
        ArithEqAirMeta {
            air_id: ArithEqTrace::<()>::AIR_ID,
            ops: &ArithEqOp::ALL,
            row_size: ArithEqTraceRow::<Goldilocks>::ROW_SIZE,
            num_rows: ArithEqTrace::<()>::NUM_ROWS,
        },
    ]
}

impl<F: PrimeField64> ArithEqSM<F> {
    /// Air-id ↔ instance correspondence: dispatch the witness computation to the config air
    /// identified by `air_id`, picking the packed or unpacked row layout. This is the single place
    /// that maps a runtime `air_id` to its concrete trace row type; the generic `compute_witness`
    /// does the rest. Only airs instantiated in `pil/zisk.pil` are handled.
    pub fn compute_witness_for_air(
        &self,
        air_id: usize,
        sctx: &SetupCtx<F>,
        inputs: &[Vec<ArithEqInput>],
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        macro_rules! dispatch {
            ( $( $alias:ident : $row:ident / $row_packed:ident ),+ $(,)? ) => {
                match air_id {
                    $(
                        id if id == $alias::<()>::AIR_ID => {
                            if packed {
                                self.compute_witness::<$row_packed<F>>(sctx, inputs, trace_buffer)
                            } else {
                                self.compute_witness::<$row<F>>(sctx, inputs, trace_buffer)
                            }
                        }
                    )+
                    _ => panic!("ArithEqSM::compute_witness_for_air: unsupported air_id {air_id}"),
                }
            };
        }
        dispatch!(
            ArithEqTrace: ArithEqTraceRow / ArithEqTraceRowPacked,
            Arith256Trace: Arith256TraceRow / Arith256TraceRowPacked,
            Arith256XTrace: Arith256XTraceRow / Arith256XTraceRowPacked,
            ArithSecp256K1Trace: ArithSecp256K1TraceRow / ArithSecp256K1TraceRowPacked,
            ArithBn254EcTrace: ArithBn254EcTraceRow / ArithBn254EcTraceRowPacked,
            ArithBn254ComplexTrace: ArithBn254ComplexTraceRow / ArithBn254ComplexTraceRowPacked,
        )
    }
}
