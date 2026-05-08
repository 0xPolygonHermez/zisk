mod input_data_sm;
mod mem;
mod mem_align_byte_instance;
mod mem_align_byte_sm;
mod mem_align_collector;
mod mem_align_instance;
mod mem_align_read_byte_instance;
mod mem_align_rom_sm;
mod mem_align_sm;
mod mem_align_write_byte_instance;
mod mem_counters_cursor;
mod mem_inputs;
mod mem_module;
mod mem_module_collector;
mod mem_module_instance;
mod mem_module_planner;
mod mem_planner;
mod mem_sm;
mod mem_test;
mod rom_data_sm;

use input_data_sm::*;
pub use mem::*;
pub use mem_align_byte_instance::*;
pub use mem_align_byte_sm::*;
pub use mem_align_collector::*;
pub use mem_align_instance::*;
pub use mem_align_read_byte_instance::*;
use mem_align_rom_sm::*;
pub use mem_align_sm::*;
pub use mem_align_write_byte_instance::*;
use mem_counters_cursor::*;
pub use mem_inputs::*;
use mem_module::*;
pub use mem_module_collector::*;
pub use mem_module_instance::*;
use mem_module_planner::*;
pub use mem_planner::*;
use mem_sm::*;
use rom_data_sm::*;

// =====================================================================
// Unit-test framework markers. Each marker's `manager` is the inner SM
// directly (not the `Mem` orchestrator). `MemAlignSm` keeps a manual
// helper for the per-input `used_rows` count; the rest fit the macro.
// =====================================================================

use mem_common::RAM_W_ADDR_INIT;
use zisk_common::{unit_test_sm, SegmentId};
use zisk_pil::{
    InputDataTrace, InputDataTraceRow, MemAlignTrace, MemAlignTraceRow, MemAlignTraceRowPacked,
    MemTrace, MemTraceRow, RomDataTrace, RomDataTraceRow, INPUT_DATA_AIR_IDS, MEM_AIR_IDS,
    MEM_ALIGN_AIR_IDS, ROM_DATA_AIR_IDS,
};

use crate::{
    input_data_sm::INPUT_DATA_W_ADDR_INIT, rom_data_sm::ROM_DATA_W_ADDR_INIT, MemModule,
    MemPreviousSegment,
};

/// Worst-case rows a single `MemAlignInput` consumes (write + cross-chunk).
/// Used to keep `chunk_size` conservative so the default uniform planner
/// never overflows the trace; the per-input variation is handled inside
/// `compute_witness` via `MemAlignSM::rows_per_input`.
const MEM_ALIGN_MAX_ROWS_PER_INPUT: usize = 5;

unit_test_sm! {
    MemAlignSm => {
        name: "MemAlign",
        air: MEM_ALIGN_AIR_IDS[0],
        input: MemAlignInput,
        row: MemAlignTraceRow<F>,
        manager: MemAlignSM<F>,
        chunk_size: |_| MemAlignTrace::<usize>::NUM_ROWS / MEM_ALIGN_MAX_ROWS_PER_INPUT,
        compute: |sm, _sctx, inputs, buf, packed| {
            let used_rows: usize = inputs.iter().map(MemAlignSM::<F>::rows_per_input).sum();
            let inputs = vec![inputs];
            if packed {
                sm.compute_witness::<MemAlignTraceRowPacked<F>>(&inputs, used_rows, buf)
            } else {
                sm.compute_witness::<MemAlignTraceRow<F>>(&inputs, used_rows, buf)
            }
        },
    }
}

/// Sort the same way the production `MemModuleInstance::prepare_inputs`
/// does before handing inputs to a `MemModule::compute_witness` call.
fn sorted_mem_inputs(mut inputs: Vec<MemInput>) -> Vec<MemInput> {
    inputs.sort_by_key(|input| (input.addr, input.step));
    inputs
}

/// Default segment continuation for unit-test single-segment runs:
/// segment 0, last segment, `(addr, 0, 0)` where `addr` is the module's
/// first writable address (the SM treats `segment_id == 0` as a special
/// case anyway).
fn default_prev_segment(addr: u32) -> MemPreviousSegment {
    MemPreviousSegment { addr, step: 0, value: 0 }
}

unit_test_sm! {
    MemSm => {
        name: "Mem",
        air: MEM_AIR_IDS[0],
        input: MemInput,
        row: MemTraceRow<F>,
        manager: MemSM<F>,
        chunk_size: |_| MemTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            let inputs = sorted_mem_inputs(inputs);
            let prev = default_prev_segment(RAM_W_ADDR_INIT);
            sm.compute_witness(&inputs, SegmentId(0), true, &prev, buf, packed)
        },
    }
}

unit_test_sm! {
    RomDataSm => {
        name: "RomData",
        air: ROM_DATA_AIR_IDS[0],
        input: MemInput,
        row: RomDataTraceRow<F>,
        manager: RomDataSM<F>,
        chunk_size: |_| RomDataTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            let inputs = sorted_mem_inputs(inputs);
            let prev = default_prev_segment(ROM_DATA_W_ADDR_INIT);
            sm.compute_witness(&inputs, SegmentId(0), true, &prev, buf, packed)
        },
    }
}

unit_test_sm! {
    InputDataSm => {
        name: "InputData",
        air: INPUT_DATA_AIR_IDS[0],
        input: MemInput,
        row: InputDataTraceRow<F>,
        manager: InputDataSM<F>,
        chunk_size: |_| InputDataTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            let inputs = sorted_mem_inputs(inputs);
            let prev = default_prev_segment(INPUT_DATA_W_ADDR_INIT);
            sm.compute_witness(&inputs, SegmentId(0), true, &prev, buf, packed)
        },
    }
}
