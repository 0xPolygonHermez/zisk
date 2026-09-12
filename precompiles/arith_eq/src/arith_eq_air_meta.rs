//! Per-config air metadata for cost-based planning.
//!
//! Each `ArithEq` config air (one alias in `pil/zisk.pil`) covers a fixed set of sub-operations and
//! has a fixed width and capacity (`NUM_ROWS`). The planner counts each operation separately, then —
//! over only the airs actually present in the pilout — assigns each operation under the shared
//! criterion: fewest instances first, least memory to break a tie.
//!
//! A config comes in as many heights as `zisk.pil` gives it aliases, each committing exactly the
//! same columns over more rows: a plain air at `2**20` and a `Large` at `2**23` for the specialized
//! configs, and a third rung on the universal one, whose ladder is `2**20` / `2**22` / `2**23`. The
//! taller ones keep the instance count down; the shorter ones keep the memory down once the count is
//! settled. The ladders meet at the top: every specialized `Large` sits at the same height as the
//! universal `ArithEqHuge`, so a bulk of an operation a specialized config covers ties on instance
//! count with the universal air and is sent to the narrower specialized one by the memory
//! tie-break. The universal `2**22` rung exists for the leftovers that would waste three quarters of
//! a `Huge`.
//!
//! This table is the planner's static input; it is derived from the `equations` bitmask each alias
//! was instantiated with. `num_rows` is read from the trace types and the cost from
//! that air's `*_INSTANCE_COST` constant in [`zisk_pil::air_costs`].

use crate::{ArithEqInput, ArithEqOp, ArithEqSM};
use proofman_common::{AirInstance, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use zisk_pil::*;

/// Static description of one `ArithEq` config air.
#[derive(Clone, Copy)]
pub struct ArithEqAirMeta {
    /// Air id in the ZisK air group (matches `<Alias>Trace::AIR_ID`).
    pub air_id: usize,
    /// Sub-operations this air can prove.
    pub ops: &'static [ArithEqOp],
    /// Area of one instance of this air, full or not.
    pub cost: usize,
    /// Rows per instance (capacity).
    pub num_rows: usize,
}

impl ArithEqAirMeta {
    /// Whether this air covers `op`.
    #[inline]
    pub fn covers(&self, op: ArithEqOp) -> bool {
        self.ops.contains(&op)
    }
}

/// One entry per air alias instantiated in `pil/zisk.pil`, most specific first and, within a config,
/// shortest to tallest — the order the planner returns plans in, which is what orders a split
/// operation's collect windows.
///
/// NOTE: keep this in sync with the aliases in `pil/zisk.pil`. Airs whose id is absent from the
/// pilout at runtime (`ARITH_EQ_AIR_IDS`) are simply skipped by the planner.
pub fn air_metas() -> Vec<ArithEqAirMeta> {
    use ArithEqOp::*;

    /// One config's heights, shortest first, sharing the operations they cover. Each height is
    /// paired with its own `*_INSTANCE_COST` constant rather than a lookup by air id, since air ids
    /// are positional. A config takes as many heights as `zisk.pil` gives it aliases.
    macro_rules! config {
        ($ops:expr, $( $air:ident : $cost:ident ),+ $(,)?) => {
            [$(
                ArithEqAirMeta {
                    air_id: $air::<()>::AIR_ID,
                    ops: $ops,
                    cost: $cost,
                    num_rows: $air::<()>::NUM_ROWS,
                },
            )+]
        };
    }

    let mut metas = Vec::with_capacity(9);
    // arith256 + arith256_mod.
    metas.extend(config!(
        &[Arith256, Arith256Mod],
        Arith256XTrace: ARITH_256_X_INSTANCE_COST,
        Arith256XLargeTrace: ARITH_256_X_LARGE_INSTANCE_COST,
    ));
    // secp256k1 add/dbl.
    metas.extend(config!(
        &[Secp256k1Add, Secp256k1Dbl],
        ArithSecp256K1Trace: ARITH_SECP_256_K_1_INSTANCE_COST,
        ArithSecp256K1LargeTrace: ARITH_SECP_256_K_1_LARGE_INSTANCE_COST,
    ));
    // bn254 EC add/dbl and complex add/sub/mul.
    metas.extend(config!(
        &[Bn254CurveAdd, Bn254CurveDbl, Bn254ComplexAdd, Bn254ComplexSub, Bn254ComplexMul],
        ArithBn254Trace: ARITH_BN_254_INSTANCE_COST,
        ArithBn254LargeTrace: ARITH_BN_254_LARGE_INSTANCE_COST,
    ));
    // The full airs, which cover every operation and are the only home of the secp256r1 ones.
    metas.extend(config!(
        &ArithEqOp::ALL,
        ArithEqTrace: ARITH_EQ_INSTANCE_COST,
        ArithEqLargeTrace: ARITH_EQ_LARGE_INSTANCE_COST,
        ArithEqHugeTrace: ARITH_EQ_HUGE_INSTANCE_COST,
    ));
    metas
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
            ArithEqLargeTrace: ArithEqLargeTraceRow / ArithEqLargeTraceRowPacked,
            ArithEqHugeTrace: ArithEqHugeTraceRow / ArithEqHugeTraceRowPacked,
            Arith256XTrace: Arith256XTraceRow / Arith256XTraceRowPacked,
            Arith256XLargeTrace: Arith256XLargeTraceRow / Arith256XLargeTraceRowPacked,
            ArithSecp256K1Trace: ArithSecp256K1TraceRow / ArithSecp256K1TraceRowPacked,
            ArithSecp256K1LargeTrace:
                ArithSecp256K1LargeTraceRow / ArithSecp256K1LargeTraceRowPacked,
            ArithBn254Trace: ArithBn254TraceRow / ArithBn254TraceRowPacked,
            ArithBn254LargeTrace: ArithBn254LargeTraceRow / ArithBn254LargeTraceRowPacked,
        )
    }
}
