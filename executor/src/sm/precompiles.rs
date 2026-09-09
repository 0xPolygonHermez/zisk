//! Precompile registry — the declarative source of truth for every
//! precompile available to the executor.
//!
//! The `register_precompiles!` macro emits `Precompiles<F>` (with its
//! `all` constructor), `PrecompileCounters<F>`, `PrecompileCollectors<F>`,
//! plus the `PRECOMPILE_AIR_IDS` / `PRECOMPILE_RANK_ASSIGN` const slices
//! from the one-liner-per-precompile list below. Per-precompile types
//! (`*Manager`, `*CounterInputGen`, `*Instance`, `*Collector`) are
//! derived from the variant name via `paste!` and must be in scope —
//! hence the per-crate `use` lines above the invocation.
//!
//! Adding a precompile: ONE line in `register_precompiles!`, plus the
//! matching per-crate import.

use zisk_common::ComponentBuilder;
use zisk_core::{
    ARITH_EQ_384_OP_TYPE_ID, ARITH_EQ_OP_TYPE_ID, BABYJUBJUB_OP_TYPE_ID, BIG_INT_OP_TYPE_ID,
    BLAKE2_OP_TYPE_ID, BLAKE3_OP_TYPE_ID, KECCAK_OP_TYPE_ID, POSEIDON_OP_TYPE_ID,
    SHA256_OP_TYPE_ID,
};
use zisk_pil::{
    ADD_256_AIR_IDS, ARITH_EQ_384_AIR_IDS, ARITH_EQ_384_LARGE_AIR_IDS, BABY_JUB_JUB_AIR_IDS,
    BLAKE_2_BR_AIR_IDS, BLAKE_3_F_AIR_IDS, KECCAKF_AIR_IDS, POSEIDON_AIR_IDS, SHA_256_F_AIR_IDS,
};

/// Every height of the `ArithEq384` air, which the planner sizes as one ladder. Keep in step with
/// the `traces` ladder in `zisk_precomp_arith_eq_384`: an alias missing here has no state machine to
/// build its instances, and the executor fails with `StateMachineNotFound` on the air id.
const ARITH_EQ_384_CONFIG_AIR_IDS: &[usize] =
    &[ARITH_EQ_384_AIR_IDS[0], ARITH_EQ_384_LARGE_AIR_IDS[0]];
use zisk_precomp_arith_eq::{
    ArithEqCollector, ArithEqCounterInputGen, ArithEqInstance, ArithEqManager,
    ARITH_EQ_CONFIG_AIR_IDS,
};
use zisk_precomp_arith_eq_384::{
    ArithEq384Collector, ArithEq384CounterInputGen, ArithEq384Instance, ArithEq384Manager,
};
use zisk_precomp_babyjubjub::{
    BabyJubJubCollector, BabyJubJubCounterInputGen, BabyJubJubInstance, BabyJubJubManager,
};
use zisk_precomp_big_int::{Add256Collector, Add256CounterInputGen, Add256Instance, Add256Manager};
use zisk_precomp_blake2::{Blake2Collector, Blake2CounterInputGen, Blake2Instance, Blake2Manager};
use zisk_precomp_blake3::{Blake3Collector, Blake3CounterInputGen, Blake3Instance, Blake3Manager};
use zisk_precomp_keccakf::{
    KeccakfCollector, KeccakfCounterInputGen, KeccakfInstance, KeccakfManager,
};
use zisk_precomp_poseidon::{
    PoseidonCollector, PoseidonCounterInputGen, PoseidonInstance, PoseidonManager,
};
use zisk_precomp_sha256f::{
    Sha256fCollector, Sha256fCounterInputGen, Sha256fInstance, Sha256fManager,
};

crate::register_precompiles! {
    Keccakf [
        op: KECCAK_OP_TYPE_ID,
        air: KECCAKF_AIR_IDS,
        rank_assign: true,
    ] => KeccakfManager<F>,
    Sha256f [
        op: SHA256_OP_TYPE_ID,
        air: SHA_256_F_AIR_IDS,
        rank_assign: false,
    ] => Sha256fManager<F>,
    Poseidon [
        op: POSEIDON_OP_TYPE_ID,
        air: POSEIDON_AIR_IDS,
        rank_assign: false,
    ] => PoseidonManager<F>,
    Blake2 [
        op: BLAKE2_OP_TYPE_ID,
        air: BLAKE_2_BR_AIR_IDS,
        rank_assign: false,
    ] => Blake2Manager<F>,
    Blake3 [
        op: BLAKE3_OP_TYPE_ID,
        air: BLAKE_3_F_AIR_IDS,
        rank_assign: false,
    ] => Blake3Manager<F>,
    ArithEq [
        op: ARITH_EQ_OP_TYPE_ID,
        air: ARITH_EQ_CONFIG_AIR_IDS,
        rank_assign: false,
    ] => ArithEqManager<F>,
    ArithEq384 [
        op: ARITH_EQ_384_OP_TYPE_ID,
        air: ARITH_EQ_384_CONFIG_AIR_IDS,
        rank_assign: false,
    ] => ArithEq384Manager<F>,
    Add256 [
        op: BIG_INT_OP_TYPE_ID,
        air: ADD_256_AIR_IDS,
        rank_assign: false,
    ] => Add256Manager<F>,
    BabyJubJub [
        op: BABYJUBJUB_OP_TYPE_ID,
        air: BABY_JUB_JUB_AIR_IDS,
        rank_assign: false,
    ] => BabyJubJubManager<F>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_pil::AIR_NAMES;

    /// Air ids of the pilout airs whose name `keep` accepts.
    fn family(keep: impl Fn(&str) -> bool) -> Vec<usize> {
        AIR_NAMES.iter().filter(|(_, _, name)| keep(name)).map(|&(_, air_id, _)| air_id).collect()
    }

    /// A ladder registered here short of one of its heights compiles fine — the list is a plain
    /// `&[usize]` — and only fails at run time, when the executor finds no state machine for the
    /// missing air and aborts the whole proof with `StateMachineNotFound`. So pin each ladder to
    /// the pilout: adding an alias in `zisk.pil` without registering it fails here instead.
    ///
    /// The `ArithEq` family is every `Arith*` air except the standalone `Arith` state machine (a
    /// builtin, not a precompile) and the `ArithEq384` ones, which are their own precompile: its
    /// aliases are named after the equations they cover (`Arith256X`, `ArithSecp256K1`,
    /// `ArithBn254`), not after the config air.
    #[test]
    fn every_arith_eq_air_in_the_pilout_has_a_state_machine() {
        let mut registered = ARITH_EQ_CONFIG_AIR_IDS.to_vec();
        registered.sort_unstable();
        assert_eq!(
            registered,
            family(|n| n.starts_with("Arith") && n != "Arith" && !n.starts_with("ArithEq384"))
        );
    }

    #[test]
    fn every_arith_eq_384_air_in_the_pilout_has_a_state_machine() {
        let mut registered = ARITH_EQ_384_CONFIG_AIR_IDS.to_vec();
        registered.sort_unstable();
        assert_eq!(registered, family(|n| n.starts_with("ArithEq384")));
    }
}
