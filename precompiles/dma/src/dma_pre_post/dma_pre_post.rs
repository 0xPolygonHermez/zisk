use std::sync::Arc;

use fields::PrimeField64;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use rayon::{
    iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
    slice::{ParallelSlice, ParallelSliceMut},
};
use zisk_pil::{DMA_PRE_POST_TABLE_ID, DMA_PRE_POST_TABLE_SIZE, DUAL_RANGE_BYTE_ID};

#[cfg(feature = "packed")]
pub use zisk_pil::{DmaPrePostTracePacked, DmaPrePostTraceRowPacked};

#[cfg(not(feature = "packed"))]
pub use zisk_pil::{DmaPrePostTrace, DmaPrePostTraceRow};

#[cfg(feature = "packed")]
type DmaPrePostTraceRowType<F> = DmaPrePostTraceRowPacked<F>;
#[cfg(feature = "packed")]
type DmaPrePostTraceType<F> = DmaPrePostTracePacked<F>;

#[cfg(not(feature = "packed"))]
type DmaPrePostTraceRowType<F> = DmaPrePostTraceRow<F>;
#[cfg(not(feature = "packed"))]
type DmaPrePostTraceType<F> = DmaPrePostTrace<F>;

use crate::{DmaPrePostInput, DmaPrePostRom};
use precompiles_helpers::DmaInfo;

/// The `DmaPrePostSM` struct encapsulates the logic of the DmaPrePost State Machine.
pub struct DmaPrePostSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Range checks ID's
    pre_post_table_id: usize,

    /// Dual Byte Range checks
    dual_range_byte_id: usize,
}

impl<F: PrimeField64> DmaPrePostSM<F> {
    /// Creates a new Dma State Machine instance.
    ///
    /// # Returns
    /// A new `DmaPrePostSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self {
            std: std.clone(),
            dual_range_byte_id: std
                .get_virtual_table_id(DUAL_RANGE_BYTE_ID)
                .expect("Failed to get tabl eDUAL_RANGE_BYTE ID ID"),
            pre_post_table_id: std
                .get_virtual_table_id(DMA_PRE_POST_TABLE_ID)
                .expect("Failed to get table DMA_PRE_POST_TABLE_ID ID ID"),
        })
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Dma trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_slice(
        &self,
        input: &DmaPrePostInput,
        trace: &mut DmaPrePostTraceRowType<F>,
        pre_post_table_mul: &mut [u64],
        local_dual_range_byte_mul: &mut [u64],
    ) {
        let dst_offset = input.dst & 0x07;
        let src_offset = input.src & 0x07;
        let is_pre = dst_offset > 0;

        let dst64 = input.dst >> 3;
        let src64 = input.src >> 3;

        trace.set_main_step(input.step);
        trace.set_dst64(dst64);
        trace.set_src64(src64);
        trace.set_dst_offset(dst_offset as u8);
        trace.set_src_offset(src_offset as u8);

        let count = if is_pre {
            DmaInfo::get_pre_count(input.encoded)
        } else {
            DmaInfo::get_post_count(input.encoded)
        };

        trace.set_count(count as u8);
        trace.set_enabled(true);
        let second_read = (src_offset as usize + count) > 8;
        //println!("SECOND_READ: {second_read}");
        trace.set_enabled_second_read(second_read);

        let mut value = input.src_values[0];
        let mut bytes = [0u8; 24];

        bytes[0] = value as u8;
        bytes[1] = (value >> 8) as u8;
        bytes[2] = (value >> 16) as u8;
        bytes[3] = (value >> 24) as u8;
        bytes[4] = (value >> 32) as u8;
        bytes[5] = (value >> 40) as u8;
        bytes[6] = (value >> 48) as u8;
        bytes[7] = (value >> 56) as u8;

        local_dual_range_byte_mul[(value & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 16) & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 32) & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 48) & 0xFFFF) as usize] += 1;

        if second_read {
            value = input.src_values[1];
            bytes[8] = value as u8;
            bytes[9] = (value >> 8) as u8;
            bytes[10] = (value >> 16) as u8;
            bytes[11] = (value >> 24) as u8;
            bytes[12] = (value >> 32) as u8;
            bytes[13] = (value >> 40) as u8;
            bytes[14] = (value >> 48) as u8;
            bytes[15] = (value >> 56) as u8;
            local_dual_range_byte_mul[(value & 0xFFFF) as usize] += 1;
            local_dual_range_byte_mul[((value >> 16) & 0xFFFF) as usize] += 1;
            local_dual_range_byte_mul[((value >> 32) & 0xFFFF) as usize] += 1;
            local_dual_range_byte_mul[((value >> 48) & 0xFFFF) as usize] += 1;
        } else {
            local_dual_range_byte_mul[0] += 4;
        }

        value = input.dst_pre_value;
        bytes[16] = value as u8;
        bytes[17] = (value >> 8) as u8;
        bytes[18] = (value >> 16) as u8;
        bytes[19] = (value >> 24) as u8;
        bytes[20] = (value >> 32) as u8;
        bytes[21] = (value >> 40) as u8;
        bytes[22] = (value >> 48) as u8;
        bytes[23] = (value >> 56) as u8;

        local_dual_range_byte_mul[(value & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 16) & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 32) & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 48) & 0xFFFF) as usize] += 1;

        let selr_value = if dst_offset > src_offset {
            trace.set_dst_offset_gt_src_offset(true);
            dst_offset - src_offset
        } else {
            trace.set_dst_offset_gt_src_offset(false);
            src_offset - dst_offset
        };

        let read_value_23 =
            if selr_value > 0 { input.src_values[0] << (selr_value * 8) } else { 0 };
        let read_value_01 = (input.src_values[0] >> (selr_value * 8))
            | if selr_value > 0 { input.src_values[1] << (64 - selr_value * 8) } else { 0 };

        let _mask = 0xFFFF_FFFF_FFFF_FFFFu64 << (dst_offset * 8);
        let mask = _mask ^ (_mask << (count * 8));

        let write_value_01 = (read_value_01 & mask) | (input.dst_pre_value & !mask);
        let write_value_23 = (read_value_23 & mask) | (input.dst_pre_value & !mask);

        trace.set_write_value(0, write_value_01 as u32);
        trace.set_write_value(1, (write_value_01 >> 32) as u32);
        trace.set_write_value(2, write_value_23 as u32);
        trace.set_write_value(3, (write_value_23 >> 32) as u32);

        trace.set_selb(0, (mask & 0x0000_0000_0000_00FF) != 0);
        trace.set_selb(1, (mask & 0x0000_0000_0000_FF00) != 0);
        trace.set_selb(2, (mask & 0x0000_0000_00FF_0000) != 0);
        trace.set_selb(3, (mask & 0x0000_0000_FF00_0000) != 0);
        trace.set_selb(4, (mask & 0x0000_00FF_0000_0000) != 0);
        trace.set_selb(5, (mask & 0x0000_FF00_0000_0000) != 0);
        trace.set_selb(6, (mask & 0x00FF_0000_0000_0000) != 0);
        trace.set_selb(7, (mask & 0xFF00_0000_0000_0000) != 0);

        for (index, byte) in bytes.iter().enumerate() {
            // println!("PRE-POST bytes[{index}]: 0x{byte:02X}");
            trace.set_bytes(index, *byte);
        }

        trace.set_selread(0, selr_value == 0);
        trace.set_selread(1, selr_value == 1);
        trace.set_selread(2, selr_value == 2);
        trace.set_selread(3, selr_value == 3);
        trace.set_selread(4, selr_value == 4);
        trace.set_selread(5, selr_value == 5);
        trace.set_selread(6, selr_value == 6);

        // println!("PRE-POST write_value: 0x{write_value_01:016X} 0x{write_value_23:016X}");

        let table_row = DmaPrePostRom::get_row(dst_offset as usize, src_offset as usize, count);
        // println!("PRE-POST-ROM [{table_row}] dst_offset: {dst_offset} src_offset: {src_offset} count: {count}");
        pre_post_table_mul[table_row] += 1;

        // println!("DMA_PRE_POST: bytes={bytes:?} selr_value={selr_value} mask=0x{mask:016X}");
        // println!(
        //     "DMA_PRE_POST: read_value_01=0x{read_value_01:016X} read_value_23=0x{read_value_23:016X}"
        // );
        // println!("DMA_PRE_POST: write_value_xx=[0x{write_value_01:016X},0x{write_value_23:016X}] dst_offset={dst_offset} src_offset={src_offset}");
        // println!(
        //     "DMA_PRE_POST: selb={:?}",
        //     [
        //         ((mask & 0x0000_0000_0000_00FF) != 0) as u8,
        //         ((mask & 0x0000_0000_0000_FF00) != 0) as u8,
        //         ((mask & 0x0000_0000_00FF_0000) != 0) as u8,
        //         ((mask & 0x0000_0000_FF00_0000) != 0) as u8,
        //         ((mask & 0x0000_00FF_0000_0000) != 0) as u8,
        //         ((mask & 0x0000_FF00_0000_0000) != 0) as u8,
        //         ((mask & 0x00FF_0000_0000_0000) != 0) as u8,
        //         ((mask & 0xFF00_0000_0000_0000) != 0) as u8
        //     ]
        // );
        // println!(
        //     "DMA_PRE_POST: selread={:?}",
        //     [
        //         (selr_value == 0) as u8,
        //         (selr_value == 1) as u8,
        //         (selr_value == 2) as u8,
        //         (selr_value == 3) as u8,
        //         (selr_value == 4) as u8,
        //         (selr_value == 5) as u8,
        //         (selr_value == 6) as u8,
        //         (selr_value == 7) as u8
        //     ]
        // );
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Dma trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_empty_slice(&self, trace: &mut DmaPrePostTraceRowType<F>) {
        trace.set_main_step(0);
        trace.set_dst64(0);
        trace.set_src64(0);
        trace.set_dst_offset(0);
        trace.set_src_offset(0);
        for index in 0..7 {
            trace.set_selread(index, false);
        }

        trace.set_dst_offset_gt_src_offset(false);
        trace.set_count(0);
        trace.set_enabled(false);
        trace.set_enabled_second_read(false);

        for index in 0..24 {
            trace.set_bytes(index, 0);
        }
        for index in 0..8 {
            trace.set_selb(index, false);
        }
        trace.set_write_value(0, 0);
        trace.set_write_value(1, 0);
        trace.set_write_value(2, 0);
        trace.set_write_value(3, 0);
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
        inputs: &[Vec<DmaPrePostInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = DmaPrePostTraceType::<F>::new_from_vec(trace_buffer)?;
        let num_rows = trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|inputs| inputs.len()).sum();

        assert!(total_inputs <= num_rows);
        assert!(total_inputs > 0);

        tracing::debug!(
            "··· Creating DmaPrePost instance [{total_inputs} / {num_rows} rows filled {:.2}%]",
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(DMA_PRE_POST_TRACE);

        // Split the dma_trace.buffer into slices matching each inner vector’s length.
        let flat_inputs: Vec<_> = inputs.iter().flatten().collect();
        let trace_rows = trace.buffer.as_mut_slice();

        // Calculate optimal chunk size
        let num_threads = rayon::current_num_threads();
        let chunk_size = std::cmp::max(1, flat_inputs.len() / num_threads);

        // Process in chunks to allow per-chunk local multiplicities arrays
        let (global_pre_post_table_mul, global_dual_range_byte_mul): (
            Vec<Vec<u64>>,
            Vec<Vec<u64>>,
        ) = flat_inputs
            .par_chunks(chunk_size)
            .zip(trace_rows.par_chunks_mut(chunk_size))
            .map(|(input_chunk, trace_chunk)| {
                // Local array shared by this chunk
                let mut local_pre_post_table_mul = vec![0u64; DMA_PRE_POST_TABLE_SIZE];
                let mut local_dual_range_byte_mul = vec![0u64; 1 << 16];

                // Sum all local arrays into a global one
                for (input, trace_row) in input_chunk.iter().zip(trace_chunk.iter_mut()) {
                    self.process_slice(
                        input,
                        trace_row,
                        &mut local_pre_post_table_mul,
                        &mut local_dual_range_byte_mul,
                    );
                }

                (local_pre_post_table_mul, local_dual_range_byte_mul)
            })
            .collect();

        for pre_post_table_mul in global_pre_post_table_mul.iter() {
            // println!("PRE_POST_TABLE_MUL {:?}", pre_post_table_mul);
            self.std.inc_virtual_rows_ranged(self.pre_post_table_id, pre_post_table_mul);
        }

        for dual_range_byte_mul in global_dual_range_byte_mul.iter() {
            self.std.inc_virtual_rows_ranged(self.dual_range_byte_id, dual_range_byte_mul);
        }

        if total_inputs < num_rows {
            self.process_empty_slice(&mut trace_rows[total_inputs]);
            let empty_row = trace_rows[total_inputs];
            trace_rows[total_inputs + 1..].par_iter_mut().for_each(|row| {
                *row = empty_row;
            });
        }
        let from_trace = FromTrace::new(&mut trace);
        timer_stop_and_log_trace!(DMA_PRE_POST_TRACE);
        Ok(AirInstance::new_from_trace(from_trace))
    }
}
