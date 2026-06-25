//! Per-AIR unit tests for the memory state machines.
//!
//! Currently only MemAlign. The mem-module SMs (Mem, RomData, InputData)
//! compute their witness through the offsets path, which needs the
//! planner-grade `MemModuleSegmentCheckPoint` (per-address first-row offsets,
//! padding/forward-fill semantics); the unit-test registrations only build a
//! simple per-input checkpoint, which plans but does not yet satisfy the
//! padding constraints. Add Mem/RomData/InputData tests once the
//! registrations derive the checkpoint via the production planner.

use zisk_prover_backend::{inputs::MemAlignInput, testing::with_prover, MemAlignSm};

/// An unaligned 4-byte read at byte offset 2 of one 8-byte word: the read
/// `value` must be the bytes the offset/width carve out of `mem_values[0]`.
#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn mem_align_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<MemAlignSm>(MemAlignInput {
                addr: 0xa000_0002, // byte address, offset 2 within the word
                is_write: false,
                width: 4,
                step: 1,
                value: 0x0304_0506, // bytes 2..6 of mem_values[0] (little-endian)
                mem_values: [0x0102_0304_0506_0708, 0],
            })
            .run()
            .expect("verification run failed");

        assert!(result.valid, "honest MemAlign input should satisfy all constraints");
    });
}
