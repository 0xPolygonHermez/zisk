use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use goldilocks::Goldilocks;
use proofman::trace;

trace!(BinaryTrace {
    opcode: Goldilocks,
    a: [Goldilocks; 8],
    b: [Goldilocks; 8],
    c: [Goldilocks; 8],
    freein_a: [Goldilocks; 2],
    freein_b: [Goldilocks; 2],
    freein_c: [Goldilocks; 2],
    cin: Goldilocks,
    cmiddle: Goldilocks,
    cout: Goldilocks,
    l_cout: Goldilocks,
    l_opcode: Goldilocks,
    previous_are_lt4: Goldilocks,
    use_previous_are_lt4: Goldilocks,
    reset4: Goldilocks,
    use_carry: Goldilocks,
    result_bin_op: Goldilocks,
    result_valid_range: Goldilocks,
});

pub(crate) fn trace_pol_bench(c: &mut Criterion) {
    c.bench_function("trace_pol_bench", |b| {
        b.iter_batched(
            || (BinaryTrace::new(1 << 23)),
            |binary_trace| {
                for i in 0..(1 << 17) {
                    for j in 0..(1 << 3) {
                        black_box(binary_trace.a[j][i]);
                    }
                }
            },
            criterion::BatchSize::NumIterations((1 << 23) * 8),
        )
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    trace_pol_bench(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
