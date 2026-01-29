use std::sync::Arc;

use fields::PrimeField64;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator as _};

use crate::DmaUnalignedInput;
use pil_std_lib::Std;
use precompiles_helpers::DmaInfo;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::SegmentId;
use zisk_pil::{DmaUnalignedAirValues, DUAL_RANGE_BYTE_ID};

#[cfg(feature = "packed")]
pub use zisk_pil::{DmaUnalignedTracePacked, DmaUnalignedTraceRowPacked};

#[cfg(not(feature = "packed"))]
pub use zisk_pil::{DmaUnalignedTrace, DmaUnalignedTraceRow};

#[cfg(feature = "packed")]
type DmaUnalignedTraceRowType<F> = DmaUnalignedTraceRowPacked<F>;
#[cfg(feature = "packed")]
type DmaUnalignedTraceType<F> = DmaUnalignedTracePacked<F>;

#[cfg(not(feature = "packed"))]
type DmaUnalignedTraceRowType<F> = DmaUnalignedTraceRow<F>;
#[cfg(not(feature = "packed"))]
type DmaUnalignedTraceType<F> = DmaUnalignedTrace<F>;

pub struct DmaUnalignedPrevSegment {
    pub seq_end: bool,
    pub dst64: u32,
    pub src64: u32,
    pub src_offset: u8,
    pub main_step: u64,
    pub count: u32,
    pub is_mem_eq: bool,
}

/// The `DmaUnalignedSM` struct encapsulates the logic of the DmaUnaligned State Machine.
pub struct DmaUnalignedSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Range checks ID's
    range_16_bits_id: usize,
    dual_range_byte_id: usize,
}

impl<F: PrimeField64> DmaUnalignedSM<F> {
    /// Creates a new Dma State Machine instance.
    ///
    /// # Returns
    /// A new `DmaUnalignedSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self {
            std: std.clone(),
            dual_range_byte_id: std
                .get_virtual_table_id(DUAL_RANGE_BYTE_ID)
                .expect("Failed to get tabl eDUAL_RANGE_BYTE ID ID"),
            range_16_bits_id: std
                .get_range_id(0, 0xFFFF, None)
                .expect("Failed to get 16b table ID"),
        })
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Dma trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_input(
        &self,
        input: &DmaUnalignedInput,
        trace: &mut [DmaUnalignedTraceRowType<F>],
        local_dual_byte_table: &mut [u64],
        air_values: &mut DmaUnalignedAirValues<F>,
    ) -> usize {
        let rows = input.count as usize;
        let initial_count = DmaInfo::get_loop_count(input.encoded) - input.skip as usize;
        let mut count = initial_count;
        let src_offset = DmaInfo::get_loop_src_offset(input.encoded);
        let mut dst64 = (input.dst >> 3) + input.skip;
        let mut src64 = (input.src >> 3) + input.skip;

        let mut src_values_index = 0;
        let mut seq_end = false;
        let mut next_value = 0;
        // println!(
        //     "DMA_UNALIGNED INPUT {input:?} count:{count} rows:{rows} dma_info:{}",
        //     DmaInfo::to_string(input.encoded)
        // );
        assert!(rows > 0);
        for (irow, row) in trace.iter_mut().enumerate().take(rows) {
            row.set_main_step(input.step);
            row.set_is_mem_eq(false);
            row.set_no_last_no_seq_end(count != 0);
            row.set_previous_seq_end(input.skip == 0 && irow == 0);

            row.set_dst64(dst64);
            row.set_src64(src64);
            dst64 += 1;
            src64 += 1;

            row.set_offset_2(src_offset == 2);
            row.set_offset_3(src_offset == 3);
            row.set_offset_4(src_offset == 4);
            row.set_offset_5(src_offset == 5);
            row.set_offset_6(src_offset == 6);
            row.set_offset_7(src_offset == 7);

            row.set_count(count as u32);
            // println!("DMA_UNALIGNED: trace[{irow}] count:{count}");
            row.set_seq_end(count == 0);

            let value = input.src_values[src_values_index];
            src_values_index += 1;
            let write_value = if count == 0 {
                seq_end = true;
                next_value = 0;
                match src_offset {
                    1 => value >> 8,
                    2 => value >> 16,
                    3 => value >> 24,
                    4 => value >> 32,
                    5 => value >> 40,
                    6 => value >> 48,
                    7 => value >> 56,
                    _ => panic!("invalid src_offset {src_offset} on DmaUnaligned"),
                }
            } else {
                count -= 1;
                if src_values_index >= input.src_values.len() {
                    println!(
                        "DMA_UNALIGNED INPUT src_values_index out of bounds {} / {} count:{count} irow:{irow} INPUT:{:?}",
                        src_values_index,
                        input.src_values.len(),
                        input
                    );
                }
                next_value = input.src_values[src_values_index];
                match src_offset {
                    1 => (value >> 8) | (next_value << 56),
                    2 => (value >> 16) | (next_value << 48),
                    3 => (value >> 24) | (next_value << 40),
                    4 => (value >> 32) | (next_value << 32),
                    5 => (value >> 40) | (next_value << 24),
                    6 => (value >> 48) | (next_value << 16),
                    7 => (value >> 56) | (next_value << 8),
                    _ => panic!("invalid src_offset {src_offset} on DmaUnaligned"),
                }
            };

            row.set_read_bytes(0, value as u8);
            row.set_read_bytes(1, (value >> 8) as u8);
            row.set_read_bytes(2, (value >> 16) as u8);
            row.set_read_bytes(3, (value >> 24) as u8);
            row.set_read_bytes(4, (value >> 32) as u8);
            row.set_read_bytes(5, (value >> 40) as u8);
            row.set_read_bytes(6, (value >> 48) as u8);
            row.set_read_bytes(7, (value >> 56) as u8);

            row.set_write_value(0, write_value as u32);
            row.set_write_value(1, (write_value >> 32) as u32);

            let value = value as usize;
            local_dual_byte_table[value & 0xFFFF] += 1;
            local_dual_byte_table[(value >> 16) & 0xFFFF] += 1;
            local_dual_byte_table[(value >> 32) & 0xFFFF] += 1;
            local_dual_byte_table[(value >> 48) & 0xFFFF] += 1;
        }

        if input.is_last_instance_input {
            if seq_end {
                air_values.segment_last_seq_end = F::ONE;
                air_values.segment_last_src64 = F::ZERO;
                air_values.segment_last_dst64 = F::ZERO;
                air_values.segment_last_main_step = F::ZERO;
                air_values.segment_last_count = F::ZERO;
                air_values.segment_last_count = F::ZERO;
                air_values.segment_last_offset = F::ZERO;
                air_values.last_count_chunk[0] = F::ZERO;
                air_values.last_count_chunk[1] = F::ZERO;
                air_values.segment_last_is_mem_eq = F::ZERO;
                air_values.segment_next_bytes = [F::ZERO; 8];
            } else {
                let last_row = rows - 1;
                air_values.segment_last_seq_end = F::ZERO;
                air_values.segment_last_src64 = F::from_u32(trace[last_row].get_src64());
                air_values.segment_last_dst64 = F::from_u32(trace[last_row].get_dst64());
                air_values.segment_last_main_step = F::from_u64(trace[last_row].get_main_step());
                air_values.segment_last_count = F::from_u32(trace[last_row].get_count());
                air_values.segment_last_offset = F::from_u8(src_offset);
                let count = trace[last_row].get_count();
                air_values.last_count_chunk[0] = F::from_u16(count as u16);
                air_values.last_count_chunk[1] = F::from_u16((count >> 16) as u16);
                air_values.segment_last_is_mem_eq = F::from_bool(trace[last_row].get_is_mem_eq());
                for (index, byte) in air_values.segment_next_bytes.iter_mut().enumerate() {
                    *byte = F::from_u8((next_value >> (index * 8)) as u8);
                }
            }
        }
        rows
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Dma trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_empty_slice(&self, trace: &mut DmaUnalignedTraceRowType<F>) {
        trace.set_main_step(0);
        trace.set_is_mem_eq(false);
        trace.set_no_last_no_seq_end(false);
        trace.set_previous_seq_end(true);

        trace.set_dst64(0);
        trace.set_src64(0);

        trace.set_offset_2(false);
        trace.set_offset_3(false);
        trace.set_offset_4(false);
        trace.set_offset_5(false);
        trace.set_offset_6(false);
        trace.set_offset_7(false);

        trace.set_count(0);
        trace.set_seq_end(true);

        trace.set_read_bytes(0, 0);
        trace.set_read_bytes(1, 0);
        trace.set_read_bytes(2, 0);
        trace.set_read_bytes(3, 0);
        trace.set_read_bytes(4, 0);
        trace.set_read_bytes(5, 0);
        trace.set_read_bytes(6, 0);
        trace.set_read_bytes(7, 0);

        trace.set_write_value(0, 0);
        trace.set_write_value(1, 0);
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `sctx` - The setup context containing the setup data.
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness(
        &self,
        inputs: &[Vec<DmaUnalignedInput>],
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = DmaUnalignedTraceType::<F>::new_from_vec(trace_buffer)?;
        let num_rows = trace.num_rows();

        let total_inputs: usize = inputs
            .iter()
            .map(|inputs| inputs.iter().map(|input| input.count as usize).sum::<usize>())
            .sum();

        assert!(total_inputs <= num_rows);
        assert!(total_inputs > 0);

        tracing::debug!(
            "··· Creating DmaUnaligned instance [{total_inputs} / {num_rows} rows filled {:.2}%]",
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(DMA_UNALIGNED_TRACE);

        // Split the dma_trace.buffer into slices matching each inner vector’s length.
        let flat_inputs: Vec<_> = inputs.iter().flatten().collect();
        let trace_rows = trace.buffer.as_mut_slice();

        // TODO: add std method to used short table, no sense with instances around 2^22 use 64 bits, need more space.
        let mut local_dual_byte_table = vec![0u64; 1 << 16];
        let mut air_values = DmaUnalignedAirValues::<F>::new();
        let mut row_offset = 0;
        for input in flat_inputs.iter() {
            let rows_used = self.process_input(
                input,
                &mut trace_rows[row_offset..],
                &mut local_dual_byte_table,
                &mut air_values,
            );
            row_offset += rows_used;
        }

        let padding_rows = num_rows - row_offset;
        let last_count = if padding_rows == 0 && !trace_rows[num_rows - 1].get_seq_end() {
            trace_rows[num_rows - 1].get_count()
        } else {
            0
        };
        self.std.range_check(self.range_16_bits_id, (last_count & 0xFFFF) as i64, 1);
        self.std.range_check(self.range_16_bits_id, ((last_count >> 16) & 0xFFFF) as i64, 1);

        local_dual_byte_table[0] += (padding_rows * 4) as u64;
        self.std.inc_virtual_rows_ranged(self.dual_range_byte_id, &local_dual_byte_table);

        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_last_segment = F::from_bool(is_last_segment);

        let first_input = flat_inputs.first().unwrap();
        if first_input.skip == 0 {
            air_values.segment_previous_seq_end = F::ONE;
            air_values.segment_previous_dst64 = F::ZERO;
            air_values.segment_previous_src64 = F::ZERO;
            air_values.segment_previous_main_step = F::ZERO;
            air_values.segment_previous_count = F::ZERO;
            air_values.segment_previous_is_mem_eq = F::ZERO;
            air_values.segment_previous_offset = F::ZERO;
            air_values.segment_first_bytes = [F::ZERO; 8];
        } else {
            air_values.segment_previous_seq_end = F::ZERO;
            air_values.segment_previous_dst64 = F::from_u32(trace_rows[0].get_dst64() - 1);
            air_values.segment_previous_src64 = F::from_u32(trace_rows[0].get_src64() - 1);
            air_values.segment_previous_main_step = F::from_u64(trace_rows[0].get_main_step());
            air_values.segment_previous_count = F::from_u32(trace_rows[0].get_count() + 1);
            air_values.segment_previous_is_mem_eq = F::from_bool(trace_rows[0].get_is_mem_eq());
            air_values.segment_previous_offset =
                F::from_u8(DmaInfo::get_loop_src_offset(first_input.encoded));
            for (index, byte) in air_values.segment_first_bytes.iter_mut().enumerate() {
                *byte = F::from_u8(trace_rows[0].get_read_bytes(index));
            }
        }

        // padding
        if padding_rows > 0 {
            air_values.padding_rows = F::from_u32(padding_rows as u32);
            self.process_empty_slice(&mut trace_rows[row_offset]);
            let empty_row = trace_rows[row_offset];
            trace_rows[row_offset + 1..].par_iter_mut().for_each(|row| {
                *row = empty_row;
            });
            air_values.segment_last_seq_end = F::ONE;
            air_values.segment_last_src64 = F::ZERO;
            air_values.segment_last_dst64 = F::ZERO;
            air_values.segment_last_main_step = F::ZERO;
            air_values.segment_last_count = F::ZERO;
            air_values.segment_last_is_mem_eq = F::ZERO;
            air_values.segment_next_bytes = [F::ZERO; 8];
        } else {
            trace[num_rows - 1].set_no_last_no_seq_end(false);
        }
        #[cfg(feature = "debug_dma")]
        {
            println!("TRACE DmaUnalignedSM @{segment_id} [0] {:?}", trace[0]);
            println!(
                "TRACE DmaUnalignedSM @{segment_id} [{}] {:?}",
                num_rows - 1,
                trace[num_rows - 1]
            );
            println!("TRACE DmaUnalignedSM AIR_VALUES {:?}", air_values);
        }
        timer_stop_and_log_trace!(DMA_UNALIGNED_TRACE);
        let from_trace = FromTrace::new(&mut trace).with_air_values(&mut air_values);
        Ok(AirInstance::new_from_trace(from_trace))
    }
}
