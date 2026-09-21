//! CPU/GPU comparison for the Keccakf witness.
//!
//! `gpu_matches_cpu_packed_trace` is the correctness gate: the kernel must
//! produce the packed trace word for word, including the rows a slot leaves
//! empty. `bench_cpu_vs_gpu` is the measurement; it is `#[ignore]`d so a normal
//! `cargo test` stays cheap.
//!
//!   cargo test -p zisk-precomp-keccakf --release -- --ignored --nocapture bench
//!
//! Everything generic — the operation counts worth trying, driving the kernel,
//! locating a mismatch — lives in `zisk-gpu-witness`. What is here is only what
//! is Keccakf's: how to make inputs, and how the CPU packs them.

use super::*;
use crate::keccakf_gpu::{self, GpuOp, KeccakfKernel};
use proofman_fields::Goldilocks;
use std::time::Instant;
use zisk_gpu_witness::{GpuWitnessKernel, DEFAULT_COMPARE_COUNTS};

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
/// rayon workers, and returns the rows together with the wall time of the fill.
fn fill_cpu(
    sm: &KeccakfSM<Goldilocks>,
    inputs: &[KeccakfInput],
    threads: usize,
) -> (Vec<Row>, f64) {
    let num_slots = inputs.len().div_ceil(OPS_PER_SLOT);
    let mut rows = vec![Row::default(); num_slots * CLOCKS];
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
    (rows, start.elapsed().as_secs_f64() * 1e3)
}

/// The packed CPU trace as one flat word array, which is how the shared
/// comparison consumes it.
fn cpu_packed(inputs: &[KeccakfInput]) -> Vec<u64> {
    let sm = KeccakfSM::<Goldilocks>::new();
    fill_cpu(&sm, inputs, 1).0.iter().flat_map(|row| row.packed).collect()
}

#[test]
fn gpu_matches_cpu_packed_trace() {
    if KeccakfKernel::available() {
        // A geometry drift in the .cu shows up here rather than as a wrong proof.
        assert_eq!(KeccakfKernel::row_words(), Row::PACKED_WORDS);
        assert_eq!(keccakf_gpu::clocks(), CLOCKS);
    }
    zisk_gpu_witness::assert_matches_cpu::<KeccakfKernel, _>(
        DEFAULT_COMPARE_COUNTS,
        random_inputs,
        cpu_packed,
    );
}

#[test]
#[ignore = "benchmark"]
fn bench_cpu_vs_gpu() {
    if !KeccakfKernel::available() {
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
        let (rows, cpu4) = fill_cpu(&sm, &inputs, 4);
        let (_, cpu32) = fill_cpu(&sm, &inputs, rayon::current_num_threads());

        let ops: Vec<GpuOp> = inputs.iter().map(GpuOp::from).collect();
        let mut gpu = vec![0u64; KeccakfKernel::out_words(ops.len())];
        let t = keccakf_gpu::run(&ops, Some(&mut gpu), iters, 128).expect("kernel failed");

        let cpu: Vec<u64> = rows.iter().flat_map(|row| row.packed).collect();
        assert_eq!(cpu.iter().zip(&gpu).position(|(c, g)| c != g), None, "count={count}");

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
