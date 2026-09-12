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
    ARITH_EQ_384_OP_TYPE_ID, ARITH_EQ_OP_TYPE_ID, BIG_INT_OP_TYPE_ID, BLAKE2_OP_TYPE_ID,
    KECCAK_OP_TYPE_ID, KOALA_POSEIDON2_OP_TYPE_ID, POSEIDON_OP_TYPE_ID, SHA256_OP_TYPE_ID,
};
use zisk_pil::{
    ADD_256_AIR_IDS, ARITH_EQ_384_AIR_IDS, BLAKE_2_BR_AIR_IDS, KECCAKF_AIR_IDS,
    KOALA_POSEIDON_2_AIR_IDS, POSEIDON_AIR_IDS, SHA_256_F_AIR_IDS,
};
use zisk_precomp_arith_eq::{
    ArithEqCollector, ArithEqCounterInputGen, ArithEqInstance, ArithEqManager,
    ARITH_EQ_CONFIG_AIR_IDS,
};
use zisk_precomp_arith_eq_384::{
    ArithEq384Collector, ArithEq384CounterInputGen, ArithEq384Instance, ArithEq384Manager,
};
use zisk_precomp_big_int::{Add256Collector, Add256CounterInputGen, Add256Instance, Add256Manager};
use zisk_precomp_blake2::{Blake2Collector, Blake2CounterInputGen, Blake2Instance, Blake2Manager};
use zisk_precomp_keccakf::{
    KeccakfCollector, KeccakfCounterInputGen, KeccakfInstance, KeccakfManager,
};
use zisk_precomp_koala_poseidon2::{
    KoalaPoseidon2Collector, KoalaPoseidon2CounterInputGen, KoalaPoseidon2Instance,
    KoalaPoseidon2Manager,
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
    KoalaPoseidon2 [
        op: KOALA_POSEIDON2_OP_TYPE_ID,
        air: KOALA_POSEIDON_2_AIR_IDS,
        rank_assign: false,
    ] => KoalaPoseidon2Manager<F>,
    Blake2 [
        op: BLAKE2_OP_TYPE_ID,
        air: BLAKE_2_BR_AIR_IDS,
        rank_assign: false,
    ] => Blake2Manager<F>,
    ArithEq [
        op: ARITH_EQ_OP_TYPE_ID,
        air: ARITH_EQ_CONFIG_AIR_IDS,
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

#[cfg(test)]
mod koala_poseidon2_tests {
    use std::collections::HashMap;

    use proofman_fields::Goldilocks;
    use zisk_common::{
        BusDeviceMetrics, ChunkId, CollectSkipper, OPERATION_BUS_ID,
        OPERATION_BUS_KOALA_POSEIDON2_DATA_SIZE,
    };
    use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
    use zisk_pil::KoalaPoseidon2Trace;

    use super::*;

    #[test]
    fn registration_dispatches_counters_and_capacity_boundaries() {
        let air_id = KoalaPoseidon2Trace::<()>::AIR_ID;
        let capacity =
            (KoalaPoseidon2Trace::<()>::NUM_ROWS / zisk_precomp_koala_poseidon2::CLOCKS) as u64;
        assert_eq!(KOALA_POSEIDON_2_AIR_IDS, &[air_id]);
        assert_eq!(PRECOMPILE_AIR_IDS.iter().filter(|id| **id == air_id).count(), 1);
        assert_eq!(PRECOMPILE_AIR_IDS.len(), PRECOMPILE_RANK_ASSIGN.len());

        for is_asm in [false, true] {
            for count in [0_u64, 1, capacity, capacity + 1] {
                let mut counters = PrecompileCounters::<Goldilocks>::build(is_asm);
                let mut payload = [0_u64; OPERATION_BUS_KOALA_POSEIDON2_DATA_SIZE];
                payload[..5].copy_from_slice(&[
                    u64::from(ZiskOp::KOALA_POSEIDON2),
                    u64::from(KOALA_POSEIDON2_OP_TYPE_ID),
                    0,
                    0xa000_1000,
                    0,
                ]);
                for step in 0..count {
                    payload[4] = step;
                    assert!(counters.dispatch_op(
                        KOALA_POSEIDON2_OP_TYPE_ID,
                        &OPERATION_BUS_ID,
                        &payload,
                        None,
                    ));
                }
                let counter = counters.koala_poseidon2.1;
                assert_eq!(counter.inst_count(ZiskOperationType::KoalaPoseidon2), Some(count));
                assert_eq!(counter.inst_count(ZiskOperationType::Poseidon), None);
                let plans = Precompiles::<Goldilocks>::planner_for_air_id(air_id, is_asm)
                    .plan(vec![(ChunkId(0), Box::new(counter))]);
                assert_eq!(plans.len(), count.div_ceil(capacity) as usize);
                let mut consumed = 0;
                for plan in plans {
                    assert_eq!(plan.air_id, air_id);
                    assert_eq!(plan.airgroup_id, KoalaPoseidon2Trace::<()>::AIRGROUP_ID);
                    let metadata = plan
                        .meta
                        .as_ref()
                        .unwrap()
                        .downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>()
                        .unwrap();
                    let (operations, skipper) = metadata[&ChunkId(0)];
                    assert_eq!(operations, (count - consumed).min(capacity));
                    assert_eq!(skipper.skip, consumed);
                    consumed += operations;
                }
                assert_eq!(consumed, count);
            }
        }
    }

    #[test]
    fn planner_keeps_calls_across_chunk_and_instance_boundaries() {
        let capacity =
            (KoalaPoseidon2Trace::<()>::NUM_ROWS / zisk_precomp_koala_poseidon2::CLOCKS) as u64;
        let mut counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> = Vec::new();
        for (chunk, count) in [3, capacity - 2].into_iter().enumerate() {
            let mut counter =
                <KoalaPoseidon2Manager<Goldilocks> as zisk_common::ComponentPlanBuilder<
                    Goldilocks,
                >>::counter(true);
            for _ in 0..count {
                zisk_common::Metrics::measure(&mut counter, &[]);
            }
            counters.push((ChunkId(chunk), Box::new(counter)));
        }
        let plans =
            Precompiles::<Goldilocks>::planner_for_air_id(KoalaPoseidon2Trace::<()>::AIR_ID, true)
                .plan(counters);
        assert_eq!(plans.len(), 2);
        let metadata = |index: usize| {
            plans[index]
                .meta
                .as_ref()
                .unwrap()
                .downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>()
                .unwrap()
        };
        assert_eq!(metadata(0)[&ChunkId(0)], (3, CollectSkipper::new(0)));
        assert_eq!(metadata(0)[&ChunkId(1)], (capacity - 3, CollectSkipper::new(0)));
        assert_eq!(metadata(1)[&ChunkId(1)], (1, CollectSkipper::new(capacity - 3)));
    }
}
