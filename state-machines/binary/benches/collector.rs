//! Microbenchmark for `BinaryBasicCollector::process_data`.
//!
//! This isolates the per-operation cost of the collector's bus-data handling
//! (payload decode + filtering + input accumulation) with a no-op virtual-table
//! sink, so no `ProofCtx`/`SetupCtx`/`Std` is required. It is the before/after
//! harness for the typed-args decode optimization.

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use std::hint::black_box;
use std::sync::Arc;

use sm_binary::BinaryBasicCollector;
use zisk_common::{
    CollectSkipper, NoopRangeChecker, A, B, OP, OPERATION_BUS_DATA_SIZE, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

/// Builds a deterministic stream of binary operation-bus payloads with varied
/// opcodes and operands, exercising the decode + filter + push paths.
fn make_payloads(n: usize) -> Vec<[u64; OPERATION_BUS_DATA_SIZE]> {
    let ops = [ZiskOp::Sub, ZiskOp::And, ZiskOp::Or, ZiskOp::Xor];
    let binary = ZiskOperationType::Binary as u64;
    (0..n)
        .map(|i| {
            let mut payload = [0u64; OPERATION_BUS_DATA_SIZE];
            payload[OP] = ops[i % ops.len()].code() as u64;
            payload[OP_TYPE] = binary;
            payload[A] = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            payload[B] = (i as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ 0xDEAD_BEEF;
            payload
        })
        .collect()
}

fn bench_process_data(c: &mut Criterion) {
    const N: usize = 100_000;
    let payloads = make_payloads(N);
    let witness = Arc::new(NoopRangeChecker);

    c.bench_function("binary_basic_collector_process_data", |b| {
        b.iter_batched(
            // `with_adds = true`, `force_execute_to_end = true` so every payload
            // is processed (no early stop on completion).
            || BinaryBasicCollector::new(N, CollectSkipper::new(0), true, true, witness.clone()),
            |mut collector| {
                for payload in &payloads {
                    black_box(collector.process_data(&OPERATION_BUS_ID, black_box(&payload[..])));
                }
                black_box(collector.inputs.len())
            },
            BatchSize::PerIteration,
        );
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(20);
    targets = bench_process_data
}
criterion_main!(benches);
