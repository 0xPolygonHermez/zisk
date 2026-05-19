//! Precompile registry — the declarative source of truth for every
//! precompile available to the executor.
//!
//! The `register_precompiles!` macro emits `Precompiles<F>`,
//! `PrecompileCounters<F>`, and `PrecompileCollectors<F>` from the
//! one-liner-per-precompile list below. Per-precompile types
//! (`*Manager`, `*CounterInputGen`, `*Instance`, `*Collector`) are
//! derived from the variant name via `paste!` and must be in scope —
//! hence the per-crate `use` lines above the invocation.
//!
//! Adding a precompile: ONE line in `register_precompiles!`, ONE line
//! in `Precompiles::all`, plus the matching per-crate import.

use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use precomp_arith_eq::{ArithEqCollector, ArithEqCounterInputGen, ArithEqInstance, ArithEqManager};
use precomp_arith_eq_384::{
    ArithEq384Collector, ArithEq384CounterInputGen, ArithEq384Instance, ArithEq384Manager,
};
use precomp_big_int::{Add256Collector, Add256CounterInputGen, Add256Instance, Add256Manager};
use precomp_blake2::{Blake2Collector, Blake2CounterInputGen, Blake2Instance, Blake2Manager};
use precomp_keccakf::{KeccakfCollector, KeccakfCounterInputGen, KeccakfInstance, KeccakfManager};
use precomp_poseidon2::{
    Poseidon2Collector, Poseidon2CounterInputGen, Poseidon2Instance, Poseidon2Manager,
};
use precomp_sha256f::{Sha256fCollector, Sha256fCounterInputGen, Sha256fInstance, Sha256fManager};
use zisk_common::ComponentBuilder;
use zisk_pil::{
    ADD_256_AIR_IDS, ARITH_EQ_384_AIR_IDS, ARITH_EQ_AIR_IDS, BLAKE_2_BR_AIR_IDS, KECCAKF_AIR_IDS,
    POSEIDON_2_AIR_IDS, SHA_256_F_AIR_IDS,
};

crate::register_precompiles! {
    Keccakf    [air: KECCAKF_AIR_IDS]      => KeccakfManager<F>,
    Sha256f    [air: SHA_256_F_AIR_IDS]    => Sha256fManager<F>,
    Poseidon2  [air: POSEIDON_2_AIR_IDS]   => Poseidon2Manager<F>,
    Blake2     [air: BLAKE_2_BR_AIR_IDS]   => Blake2Manager<F>,
    ArithEq    [air: ARITH_EQ_AIR_IDS]     => ArithEqManager<F>,
    ArithEq384 [air: ARITH_EQ_384_AIR_IDS] => ArithEq384Manager<F>,
    Add256     [air: ADD_256_AIR_IDS]      => Add256Manager<F>,
}

impl<F: PrimeField64> Precompiles<F> {
    /// Canonical default precompile set — one entry per registered
    /// variant. Mirrors `BuiltinSMs::all` on the built-in side.
    pub(crate) fn all(std: Arc<Std<F>>) -> Vec<(usize, Self)> {
        vec![
            (KECCAKF_AIR_IDS[0], Self::Keccakf(KeccakfManager::new(std.clone()))),
            (SHA_256_F_AIR_IDS[0], Self::Sha256f(Sha256fManager::new(std.clone()))),
            (POSEIDON_2_AIR_IDS[0], Self::Poseidon2(Poseidon2Manager::new(std.clone()))),
            (BLAKE_2_BR_AIR_IDS[0], Self::Blake2(Blake2Manager::new(std.clone()))),
            (ARITH_EQ_AIR_IDS[0], Self::ArithEq(ArithEqManager::new(std.clone()))),
            (ARITH_EQ_384_AIR_IDS[0], Self::ArithEq384(ArithEq384Manager::new(std.clone()))),
            (ADD_256_AIR_IDS[0], Self::Add256(Add256Manager::new(std))),
        ]
    }
}
