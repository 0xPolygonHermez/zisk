//! Witness of the fused `CompactMem` air: one segment of each memory area, on the same rows.

use std::sync::Arc;

use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use zisk_common::SegmentId;
use zisk_pil::{
    CompactMemAirValues, CompactMemTrace, CompactMemTraceRow, CompactMemTraceRowPacked,
    InputDataAirValues, MemAirValues, RomDataAirValues,
};
use zisk_sm_mem_common::{
    CompactMemSegmentCheckPoint, InputDataLaneRow, MemLaneRow, RomDataLaneRow,
};

use crate::{InputDataSM, MemOps, MemPreviousSegment, MemSM, RomDataSM};

/// Everything one block of a `CompactMem` instance needs to be filled: the operations its
/// collectors gathered and what the previous segment of its area handed over.
pub struct CompactMemBlockInputs<'a> {
    pub mem_ops: MemOps<'a>,
    pub previous_segment: MemPreviousSegment,
}

/// The three blocks of one `CompactMem` instance.
pub struct CompactMemInputs<'a> {
    pub mem: CompactMemBlockInputs<'a>,
    pub input_data: CompactMemBlockInputs<'a>,
    pub rom_data: CompactMemBlockInputs<'a>,
}

/// Fills the `CompactMem` trace by running the three area fills over the same rows.
///
/// There is no logic of its own here: each block is filled by the state machine of the air it
/// replaces, through the row adapters of `zisk_sm_mem_common::mem_block_rows`, and each fill only
/// ever touches its own columns. What this type adds is the one thing the three cannot do
/// separately -- putting their results in a single `AirInstance`.
pub struct CompactMemSM<F: PrimeField64> {
    /// The `Mem` fill has no state of its own (`MemSM::fill_trace` is an associated function), so
    /// only the other two machines are held.
    input_data_sm: Arc<InputDataSM<F>>,
    rom_data_sm: Arc<RomDataSM<F>>,
}

impl<F: PrimeField64> CompactMemSM<F> {
    /// Shares the three state machines with the standalone instances rather than building its own,
    /// so the fused air fills each block with exactly the code its standalone air runs.
    pub fn new(input_data_sm: Arc<InputDataSM<F>>, rom_data_sm: Arc<RomDataSM<F>>) -> Arc<Self> {
        Arc::new(Self { input_data_sm, rom_data_sm })
    }

    pub fn compute_witness(
        &self,
        inputs: CompactMemInputs<'_>,
        check_point: &CompactMemSegmentCheckPoint,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.compute_witness_inner::<CompactMemTraceRowPacked<F>>(
                inputs,
                check_point,
                trace_buffer,
            )
        } else {
            self.compute_witness_inner::<CompactMemTraceRow<F>>(inputs, check_point, trace_buffer)
        }
    }

    fn compute_witness_inner<R>(
        &self,
        inputs: CompactMemInputs<'_>,
        check_point: &CompactMemSegmentCheckPoint,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>>
    where
        R: MemLaneRow<F> + InputDataLaneRow<F> + RomDataLaneRow<F>,
    {
        // Zeroed, as the standalone `Mem` trace is: the three fills write every column of their
        // own block, but the `<==` columns are the prover's and a recycled buffer is not trusted
        // for them.
        let mut trace = CompactMemTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let mut air_values = CompactMemAirValues::<F>::new();

        // `CompactMem` only ever proves the FIRST segment of each area -- that is the whole point
        // of the air (see `pil/zisk.pil`), and it is what the planner builds it from.
        let segment_id = SegmentId(0);

        // The `Mem` fill is the only parallel one; it splits the rows it owns across the threads
        // of the pool this witness computation is already running in.
        let n_ranges = rayon::current_num_threads();

        // Order matters only for the logs: the three fills write disjoint columns.
        let mem_out = MemSM::<F>::fill_trace::<R>(
            &mut trace.buffer,
            inputs.mem.mem_ops,
            &inputs.mem.previous_segment,
            segment_id,
            check_point.mem.is_last_segment,
            &check_point.mem,
            n_ranges,
        );
        let input_out = self.input_data_sm.fill_trace::<R>(
            &mut trace.buffer,
            inputs.input_data.mem_ops,
            &inputs.input_data.previous_segment,
            check_point.input_data.is_last_segment,
            &check_point.input_data,
        );
        let rom_out = self.rom_data_sm.fill_trace::<R>(
            &mut trace.buffer,
            inputs.rom_data.mem_ops,
            segment_id,
            check_point.rom_data.is_last_segment,
            &check_point.rom_data,
        );

        // The air values are built by the same code as the standalone airs' and then transcribed:
        // the fused air declares the very same values, prefixed by the block that owns them.
        let mut mem_av = MemAirValues::<F>::new();
        MemSM::set_air_values(
            &mut mem_av,
            segment_id,
            check_point.mem.is_last_segment,
            &inputs.mem.previous_segment,
            &mem_out,
        );
        air_values.mem_segment_id = mem_av.segment_id;
        air_values.mem_is_first_segment = mem_av.is_first_segment;
        air_values.mem_is_last_segment = mem_av.is_last_segment;
        air_values.mem_previous_segment_value = mem_av.previous_segment_value;
        air_values.mem_previous_segment_step = mem_av.previous_segment_step;
        air_values.mem_previous_segment_addr = mem_av.previous_segment_addr;
        air_values.mem_segment_last_value = mem_av.segment_last_value;
        air_values.mem_segment_last_step = mem_av.segment_last_step;
        air_values.mem_segment_last_addr = mem_av.segment_last_addr;
        air_values.mem_distance_base = mem_av.distance_base;
        air_values.mem_distance_end = mem_av.distance_end;
        air_values.mem_last_step_chunks = mem_av.last_step_chunks;
        air_values.mem_padding_size_chunks = mem_av.padding_size_chunks;
        air_values.mem_padding_size_to_max_chunks = mem_av.padding_size_to_max_chunks;

        let mut input_av = InputDataAirValues::<F>::new();
        InputDataSM::set_air_values(
            &mut input_av,
            segment_id,
            check_point.input_data.is_last_segment,
            &inputs.input_data.previous_segment,
            &input_out,
        );
        air_values.input_segment_id = input_av.segment_id;
        air_values.input_is_first_segment = input_av.is_first_segment;
        air_values.input_is_last_segment = input_av.is_last_segment;
        air_values.input_previous_segment_value = input_av.previous_segment_value;
        air_values.input_previous_segment_step = input_av.previous_segment_step;
        air_values.input_previous_segment_addr = input_av.previous_segment_addr;
        air_values.input_segment_last_value = input_av.segment_last_value;
        air_values.input_segment_last_step = input_av.segment_last_step;
        air_values.input_segment_last_addr = input_av.segment_last_addr;
        air_values.input_distance_base = input_av.distance_base;
        air_values.input_distance_end = input_av.distance_end;

        let mut rom_av = RomDataAirValues::<F>::new();
        RomDataSM::set_air_values(
            &mut rom_av,
            segment_id,
            check_point.rom_data.is_last_segment,
            &inputs.rom_data.previous_segment,
            &rom_out,
        );
        air_values.rom_segment_id = rom_av.segment_id;
        air_values.rom_is_first_segment = rom_av.is_first_segment;
        air_values.rom_is_last_segment = rom_av.is_last_segment;
        air_values.rom_previous_segment_value = rom_av.previous_segment_value;
        air_values.rom_previous_segment_addr = rom_av.previous_segment_addr;
        air_values.rom_segment_last_value = rom_av.segment_last_value;
        air_values.rom_segment_last_addr = rom_av.segment_last_addr;
        air_values.rom_padding_size = rom_av.padding_size;

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }
}

#[cfg(test)]
#[path = "compact_mem_fill_tests.rs"]
mod fill_tests;
