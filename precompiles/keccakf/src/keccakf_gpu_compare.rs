//! CPU/GPU comparison for the Keccakf witness.
//!
//! The kernel must produce the packed trace word for word, including the rows a
//! slot leaves empty. Everything generic — the operation counts worth trying,
//! driving the kernel, locating a mismatch — lives in `zisk-gpu-witness`. What is here is only what
//! is Keccakf's: how to make inputs, and how the CPU packs them.

use super::*;
use crate::keccakf_gpu::KeccakfKernel;
use proofman_fields::Goldilocks;
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

/// The packed CPU trace as one flat word array, filled slot by slot as
/// `compute_witness` does.
fn cpu_packed(inputs: &[KeccakfInput]) -> Vec<u64> {
    let sm = KeccakfSM::<Goldilocks>::new();
    let num_slots = inputs.len().div_ceil(OPS_PER_SLOT);
    let mut rows = vec![Row::default(); num_slots * CLOCKS];
    for (slot, slot_rows) in rows.chunks_mut(CLOCKS).enumerate() {
        let op = slot * OPS_PER_SLOT;
        sm.process_slot::<Row>(slot_rows, &inputs[op], inputs.get(op + 1));
    }
    rows.iter().flat_map(|row| row.packed).collect()
}

#[test]
fn gpu_matches_cpu_packed_trace() {
    if KeccakfKernel::available() {
        // A geometry drift in the .cu shows up here rather than as a wrong proof.
        assert_eq!(KeccakfKernel::row_words(), Row::PACKED_WORDS);
        assert_eq!(KeccakfKernel::out_words(OPS_PER_SLOT), CLOCKS * Row::PACKED_WORDS);
    }
    zisk_gpu_witness::assert_matches_cpu::<KeccakfKernel, _>(
        DEFAULT_COMPARE_COUNTS,
        random_inputs,
        cpu_packed,
    );
}
