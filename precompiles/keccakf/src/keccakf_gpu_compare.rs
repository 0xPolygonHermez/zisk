//! CPU/GPU comparison harness for the Keccakf witness (spike).
//!
//! `gpu_matches_cpu_packed_trace` is the correctness gate: the kernel must
//! produce the packed trace word for word, including the rows a slot leaves
//! empty. `bench_cpu_vs_gpu` is the measurement; it is `#[ignore]`d so a normal
//! `cargo test` stays cheap.
//!
//!   cargo test -p zisk-precomp-keccakf --release -- --ignored --nocapture bench

use super::*;
use crate::keccakf_gpu::{self, GpuOp};
use proofman_fields::Goldilocks;
use std::time::Instant;

type Row = KeccakfTraceRowPacked<Goldilocks>;

fn random_inputs(count: usize, mut seed: u64) -> Vec<KeccakfInput> {
    (0..count)
        .map(|i| {
            let mut state = [0u64; 25];
            for lane in &mut state {
                seed ^= seed << 7;
                seed ^= seed >> 9;
                seed ^= seed << 8;
                *lane = seed;
            }
            KeccakfInput {
                step_main: 0x0000_00ab_cdef_0000 + i as u64,
                addr_main: 0x8000_0000 + (i as u32) * 200,
                state,
            }
        })
        .collect()
}

/// Fills the packed trace exactly as `compute_witness` does, on `threads`
/// rayon workers, and returns the wall time of the fill alone.
fn fill_cpu(
    sm: &KeccakfSM<Goldilocks>,
    inputs: &[KeccakfInput],
    rows: &mut [Row],
    threads: usize,
) -> f64 {
    rows.iter_mut().for_each(|row| *row = Row::default());
    let num_slots = inputs.len().div_ceil(OPS_PER_SLOT);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();

    let start = Instant::now();
    pool.install(|| {
        let slots_per_chunk = num_slots.div_ceil(threads).max(1);
        rows.par_chunks_mut(CLOCKS * slots_per_chunk).enumerate().for_each(
            |(chunk, chunk_rows)| {
                for (i, slot_rows) in chunk_rows.chunks_mut(CLOCKS).enumerate() {
                    let op = (chunk * slots_per_chunk + i) * OPS_PER_SLOT;
                    sm.process_slot::<Row>(slot_rows, &inputs[op], inputs.get(op + 1));
                }
            },
        );
    });
    start.elapsed().as_secs_f64() * 1e3
}

fn gpu_ops(inputs: &[KeccakfInput]) -> Vec<GpuOp> {
    inputs.iter().map(GpuOp::from).collect()
}

/// Reports the first differing word as (row, word), or `None` when identical.
fn first_mismatch(cpu: &[Row], gpu: &[u64]) -> Option<(usize, usize)> {
    let width = keccakf_gpu::row_words();
    for (r, row) in cpu.iter().enumerate() {
        for w in 0..width {
            if row.packed[w] != gpu[r * width + w] {
                return Some((r, w));
            }
        }
    }
    None
}

#[test]
fn gpu_matches_cpu_packed_trace() {
    if !keccakf_gpu::available() {
        eprintln!("no CUDA device — skipping");
        return;
    }
    assert_eq!(keccakf_gpu::row_words(), Row::PACKED_WORDS);
    assert_eq!(keccakf_gpu::clocks(), CLOCKS);

    let sm = KeccakfSM::<Goldilocks>::new();
    // Odd counts exercise the trailing op-B-absent slot; 31/33 straddle a warp.
    for &count in &[1usize, 2, 3, 4, 31, 32, 33, 64, 257] {
        let inputs = random_inputs(count, 0x9e37_79b9_7f4a_7c15 ^ count as u64);
        let mut rows = vec![Row::default(); inputs.len().div_ceil(OPS_PER_SLOT) * CLOCKS];
        fill_cpu(&sm, &inputs, &mut rows, 1);

        let mut gpu = vec![0u64; keccakf_gpu::out_words(inputs.len())];
        keccakf_gpu::run(&gpu_ops(&inputs), Some(&mut gpu), 1, 128).expect("kernel failed");

        if let Some((row, word)) = first_mismatch(&rows, &gpu) {
            panic!(
                "count={count}: row {row} (slot {}, clock {}) word {word}: cpu {:#018x} gpu {:#018x}",
                row / CLOCKS,
                row % CLOCKS,
                rows[row].packed[word],
                gpu[row * Row::PACKED_WORDS + word],
            );
        }
    }
}

#[test]
#[ignore = "benchmark"]
fn bench_cpu_vs_gpu() {
    if !keccakf_gpu::available() {
        eprintln!("no CUDA device — skipping");
        return;
    }
    // One full 2^20-row Keccakf instance holds this many operations.
    let full_instance = OPS_PER_SLOT * (zisk_pil::KeccakfTrace::<()>::NUM_ROWS / CLOCKS);
    let sm = KeccakfSM::<Goldilocks>::new();
    let iters = 20;

    println!(
        "\n{:>9} {:>10} {:>11} {:>11} {:>11} {:>11} {:>9} {:>8}",
        "ops", "MiB out", "cpu 4thr", "cpu 32thr", "gpu kern", "gpu +d2h", "GB/s", "speedup"
    );
    for &count in &[1024usize, 16384, full_instance] {
        let inputs = random_inputs(count, 0xd1b5_4a32_d192_ed03 ^ count as u64);
        let mut rows = vec![Row::default(); inputs.len().div_ceil(OPS_PER_SLOT) * CLOCKS];

        let cpu4 = fill_cpu(&sm, &inputs, &mut rows, 4);
        let cpu32 = fill_cpu(&sm, &inputs, &mut rows, rayon::current_num_threads());

        let ops = gpu_ops(&inputs);
        let mut gpu = vec![0u64; keccakf_gpu::out_words(inputs.len())];
        let t = keccakf_gpu::run(&ops, Some(&mut gpu), iters, 128).expect("kernel failed");
        assert_eq!(first_mismatch(&rows, &gpu), None, "count={count}");

        let mib = t.out_bytes as f64 / (1024.0 * 1024.0);
        let gbs = t.out_bytes as f64 / (t.kernel_ms * 1e-3) / 1e9;
        println!(
            "{count:>9} {mib:>10.1} {cpu4:>10.2}m {cpu32:>10.2}m {:>10.2}m {:>10.2}m {gbs:>9.1} {:>7.1}x",
            t.kernel_ms,
            t.kernel_ms + t.d2h_ms,
            cpu4 / t.kernel_ms,
        );
    }
    println!();
}
