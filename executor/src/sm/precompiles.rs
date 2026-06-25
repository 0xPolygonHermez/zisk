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

use precomp_arith_eq::{ArithEqCollector, ArithEqCounterInputGen, ArithEqInstance, ArithEqManager};
use precomp_arith_eq_384::{
    ArithEq384Collector, ArithEq384CounterInputGen, ArithEq384Instance, ArithEq384Manager,
};
use precomp_big_int::{Add256Collector, Add256CounterInputGen, Add256Instance, Add256Manager};
use precomp_blake2::{Blake2Collector, Blake2CounterInputGen, Blake2Instance, Blake2Manager};
use precomp_keccakf::{KeccakfCollector, KeccakfCounterInputGen, KeccakfInstance, KeccakfManager};
use precomp_poseidon::{
    PoseidonCollector, PoseidonCounterInputGen, PoseidonInstance, PoseidonManager,
};
use precomp_sha256f::{Sha256fCollector, Sha256fCounterInputGen, Sha256fInstance, Sha256fManager};
use zisk_common::ComponentBuilder;
use zisk_core::{
    ARITH_EQ_384_OP_TYPE_ID, ARITH_EQ_OP_TYPE_ID, BIG_INT_OP_TYPE_ID, BLAKE2_OP_TYPE_ID,
    KECCAK_OP_TYPE_ID, POSEIDON_OP_TYPE_ID, SHA256_OP_TYPE_ID,
};
use zisk_pil::{
    ADD_256_AIR_IDS, ARITH_EQ_384_AIR_IDS, ARITH_EQ_AIR_IDS, BLAKE_2_BR_AIR_IDS, KECCAKF_AIR_IDS,
    POSEIDON_AIR_IDS, SHA_256_F_AIR_IDS,
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
    ArithEq [
        op: ARITH_EQ_OP_TYPE_ID,
        air: ARITH_EQ_AIR_IDS,
        rank_assign: false,
    ] => ArithEqManager<F>,
    ArithEq384 [
        op: ARITH_EQ_384_OP_TYPE_ID,
        air: ARITH_EQ_384_AIR_IDS,
        rank_assign: false,
    ] => ArithEq384Manager<F>,
    Add256 [
        op: BIG_INT_OP_TYPE_ID,
        air: ADD_256_AIR_IDS,
        rank_assign: false,
    ] => Add256Manager<F>,
}
