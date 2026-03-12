use std::sync::Arc;

use fields::PrimeField64;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use rayon::{
    iter::{IndexedParallelIterator, ParallelIterator},
    slice::{ParallelSlice, ParallelSliceMut},
};
use zisk_pil::{DMA_PRE_POST_TABLE_ID, DMA_PRE_POST_TABLE_SIZE, DUAL_RANGE_BYTE_ID};

#[cfg(feature = "packed")]
pub use zisk_pil::{
    DmaPrePostMemCpyTracePacked as DmaPrePostMemCpyTrace,
    DmaPrePostMemCpyTraceRowPacked as DmaPrePostMemCpyTraceRow,
};

#[cfg(not(feature = "packed"))]
pub use zisk_pil::{DmaPrePostMemCpyTrace, DmaPrePostMemCpyTraceRow};

use crate::{dma_trace, DmaPrePostInput, DmaPrePostModule, DmaPrePostRom};
use precompiles_helpers::DmaInfo;

/// The `DmaPrePostMemCpySM` struct encapsulates the logic of the DmaPrePost State Machine.
pub struct DmaPrePostMemCpySM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Range checks ID's
    pre_post_table_id: usize,

    /// Dual Byte Range checks
    dual_range_byte_id: usize,
}

impl<F: PrimeField64> DmaPrePostMemCpySM<F> {
    /// Creates a new Dma State Machine instance.
    ///
    /// # Returns
    /// A new `DmaPrePostMemCpySM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self {
            std: std.clone(),
            dual_range_byte_id: std
                .get_virtual_table_id(DUAL_RANGE_BYTE_ID)
                .expect("Failed to get table DUAL_RANGE_BYTE ID"),
            pre_post_table_id: std
                .get_virtual_table_id(DMA_PRE_POST_TABLE_ID)
                .expect("Failed to get table DMA_PRE_POST_TABLE_ID ID"),
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
        trace: &mut DmaPrePostMemCpyTraceRow<F>,
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
        trace.set_is_post(!is_pre);

        let count = if is_pre {
            DmaInfo::get_pre_count(input.encoded)
        } else {
            DmaInfo::get_post_count(input.encoded)
        };

        trace.set_count(count as u8);
        trace.set_sel_memcpy(true);
        // intermediate: trace.last_dst_byte(0);
        let second_read = (src_offset as usize + count) > 8;
        //println!("SECOND_READ: {second_read}");
        trace.set_enabled_second_read(second_read);

        let mut value = input.src_values[0];
        let mut rb = [0u8; 16];
        let mut pb = [0u8; 8];

        rb[0] = value as u8;
        rb[1] = (value >> 8) as u8;
        rb[2] = (value >> 16) as u8;
        rb[3] = (value >> 24) as u8;
        rb[4] = (value >> 32) as u8;
        rb[5] = (value >> 40) as u8;
        rb[6] = (value >> 48) as u8;
        rb[7] = (value >> 56) as u8;

        local_dual_range_byte_mul[(value & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 16) & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 32) & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 48) & 0xFFFF) as usize] += 1;

        // println!("DUAL_RANGE_BYTE_1({:08X})", (value & 0xFFFF));
        // println!("DUAL_RANGE_BYTE_1({:08X})", ((value >> 16) & 0xFFFF));
        // println!("DUAL_RANGE_BYTE_1({:08X})", ((value >> 32) & 0xFFFF));
        // println!("DUAL_RANGE_BYTE_1({:08X})", ((value >> 48) & 0xFFFF));

        if second_read {
            value = input.src_values[1];
            rb[8] = value as u8;
            rb[9] = (value >> 8) as u8;
            rb[10] = (value >> 16) as u8;
            rb[11] = (value >> 24) as u8;
            rb[12] = (value >> 32) as u8;
            rb[13] = (value >> 40) as u8;
            rb[14] = (value >> 48) as u8;
            rb[15] = (value >> 56) as u8;
            local_dual_range_byte_mul[(value & 0xFFFF) as usize] += 1;
            local_dual_range_byte_mul[((value >> 16) & 0xFFFF) as usize] += 1;
            local_dual_range_byte_mul[((value >> 32) & 0xFFFF) as usize] += 1;
            local_dual_range_byte_mul[((value >> 48) & 0xFFFF) as usize] += 1;
            // println!("DUAL_RANGE_BYTE_2({:08X})", (value & 0xFFFF));
            // println!("DUAL_RANGE_BYTE_2({:08X})", ((value >> 16) & 0xFFFF));
            // println!("DUAL_RANGE_BYTE_2({:08X})", ((value >> 32) & 0xFFFF));
            // println!("DUAL_RANGE_BYTE_2({:08X})", ((value >> 48) & 0xFFFF));
        } else {
            local_dual_range_byte_mul[0] += 4;
        }

        value = input.dst_pre_value;
        pb[0] = value as u8;
        pb[1] = (value >> 8) as u8;
        pb[2] = (value >> 16) as u8;
        pb[3] = (value >> 24) as u8;
        pb[4] = (value >> 32) as u8;
        pb[5] = (value >> 40) as u8;
        pb[6] = (value >> 48) as u8;
        pb[7] = (value >> 56) as u8;

        local_dual_range_byte_mul[(value & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 16) & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 32) & 0xFFFF) as usize] += 1;
        local_dual_range_byte_mul[((value >> 48) & 0xFFFF) as usize] += 1;

        // println!("DUAL_RANGE_BYTE_3({:08X})", (value & 0xFFFF));
        // println!("DUAL_RANGE_BYTE_3({:08X})", ((value >> 16) & 0xFFFF));
        // println!("DUAL_RANGE_BYTE_3({:08X})", ((value >> 32) & 0xFFFF));
        // println!("DUAL_RANGE_BYTE_3({:08X})", ((value >> 48) & 0xFFFF));

        let selr_value = if dst_offset > src_offset {
            trace.set_dst_offset_gt_src_offset(true);
            dst_offset - src_offset
        } else {
            trace.set_dst_offset_gt_src_offset(false);
            src_offset - dst_offset
        };

        let _mask = 0xFFFF_FFFF_FFFF_FFFFu64 << (dst_offset * 8);
        let mask = _mask ^ (_mask << (count * 8));

        trace.set_sb(0, (mask & 0x0000_0000_0000_00FF) != 0);
        trace.set_sb(1, (mask & 0x0000_0000_0000_FF00) != 0);
        trace.set_sb(2, (mask & 0x0000_0000_00FF_0000) != 0);
        trace.set_sb(3, (mask & 0x0000_0000_FF00_0000) != 0);
        trace.set_sb(4, (mask & 0x0000_00FF_0000_0000) != 0);
        trace.set_sb(5, (mask & 0x0000_FF00_0000_0000) != 0);
        trace.set_sb(6, (mask & 0x00FF_0000_0000_0000) != 0);
        trace.set_sb(7, (mask & 0xFF00_0000_0000_0000) != 0);

        for (index, byte) in rb.iter().enumerate() {
            // println!("PRE-POST bytes[{index}]: 0x{byte:02X}");
            trace.set_rb(index, *byte);
        }
        for (index, byte) in pb.iter().enumerate() {
            // println!("PRE-POST bytes[{index}]: 0x{byte:02X}");
            trace.set_pb(index, *byte);
        }

        trace.set_selr(0, selr_value == 0);
        trace.set_selr(1, selr_value == 1);
        trace.set_selr(2, selr_value == 2);
        trace.set_selr(3, selr_value == 3);
        trace.set_selr(4, selr_value == 4);
        trace.set_selr(5, selr_value == 5);
        trace.set_selr(6, selr_value == 6);

        // println!("PRE-POST write_value: 0x{write_value_01:016X} 0x{write_value_23:016X}");

        let table_row = DmaPrePostRom::get_row(
            dst_offset as usize,
            src_offset as usize,
            count,
            false,
            false,
            true,
        );
        // println!("PRE-POST-ROM [{table_row}] dst_offset: {dst_offset} src_offset: {src_offset} count: {count}");
        pre_post_table_mul[table_row] += 1;
    }
}
impl<F: PrimeField64> DmaPrePostModule<F> for DmaPrePostMemCpySM<F> {
    fn get_name(&self) -> &'static str {
        "dma_pre_post_memcpy"
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `sctx` - The setup context containing the setup data.
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    fn compute_witness(
        &self,
        inputs: &[Vec<DmaPrePostInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = DmaPrePostMemCpyTrace::<F>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|inputs| inputs.len()).sum();

        assert!(total_inputs <= num_rows);
        assert!(total_inputs > 0);

        dma_trace("DmaPrePostMemCpy", total_inputs, num_rows);

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

        /*
        if total_inputs < num_rows {
            self.process_empty_slice(&mut trace_rows[total_inputs]);
            let empty_row = trace_rows[total_inputs];
            trace_rows[total_inputs + 1..].par_iter_mut().for_each(|row| {
                *row = empty_row;
            });
        }*/
        let from_trace = FromTrace::new(&mut trace);
        timer_stop_and_log_trace!(DMA_PRE_POST_TRACE);
        Ok(AirInstance::new_from_trace(from_trace))
    }
}
