//! Active `ArithEq` config airs for the current build.
//!
//! This stands in for the generated `zisk_pil::ARITH_EQ_AIR_IDS` while that constant still lists only
//! the full air (`&[0]`). It is the single list of air ids the planner/manager/registry consider
//! "present in the pilout" — kept in sync with the aliases in `pil/zisk.pil` and with
//! [`crate::air_metas`].
//!
//! Once the pilout exposes every alias in `ARITH_EQ_AIR_IDS`, replace uses of
//! [`ARITH_EQ_CONFIG_AIR_IDS`] with that generated constant and delete this module.

use crate::air_metas;
use zisk_pil::{
    Arith256Trace, Arith256XTrace, ArithBn254ComplexTrace, ArithBn254EcTrace, ArithEqTrace,
    ArithSecp256K1Trace,
};

/// Air ids of every `ArithEq` config air instantiated in `pil/zisk.pil`, from the trace `AIR_ID`
/// consts so it cannot drift. Current config: ArithEq(0), Arith256(1), Arith256X(2),
/// ArithSecp256K1(3), ArithBn254Ec(4), ArithBn254Complex(5).
pub const ARITH_EQ_CONFIG_AIR_IDS: &[usize] = &[
    ArithEqTrace::<()>::AIR_ID,
    Arith256Trace::<()>::AIR_ID,
    Arith256XTrace::<()>::AIR_ID,
    ArithSecp256K1Trace::<()>::AIR_ID,
    ArithBn254EcTrace::<()>::AIR_ID,
    ArithBn254ComplexTrace::<()>::AIR_ID,
];

/// Same list as a `Vec` for runtime consumers (planner). Kept consistent with
/// [`ARITH_EQ_CONFIG_AIR_IDS`] and [`crate::air_metas`].
pub fn arith_eq_air_ids() -> Vec<usize> {
    air_metas().into_iter().map(|m| m.air_id).collect()
}
