use std::sync::Arc;

use fields::PrimeField64;
use rayon::prelude::*;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::DMA_ROM_ID;

use crate::{dma::dma_rom::DmaRom, dma_trace, DmaInput, DmaModule, DMA_ROM_WITHOUT_MEMCMP_SIZE};
use precompiles_helpers::DmaInfo;

#[cfg(feature = "packed")]
pub use zisk_pil::{
    DmaInputCpyTracePacked as DmaInputCpyTrace, DmaInputCpyTraceRowPacked as DmaInputCpyTraceRow,
};

#[cfg(not(feature = "packed"))]
pub use zisk_pil::{DmaInputCpyTrace, DmaInputCpyTraceRow};

/// The `DmaInputCpySM` struct encapsulates the logic of the Dma State Machine.
pub struct DmaInputCpySM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    pub rom_table_id: usize,
    pub range_7_bits_id: usize,
    pub range_22_bits_id: usize,
    pub range_24_bits_id: usize,
}

impl<F: PrimeField64> DmaInputCpySM<F> {
    /// Creates a new Dma State Machine instance.
    ///
    /// # Returns
    /// A new `DmaInputCpySM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self {
            std: std.clone(),
            rom_table_id: std.get_virtual_table_id(DMA_ROM_ID).expect("Failed to get dma rom ID"),
            range_7_bits_id: std
                .get_range_id(0, 0x07F, None)
                .expect("Failed to get 7-bits range ID"),
            range_22_bits_id: std
                .get_range_id(0, 0x3F_FFFF, None)
                .expect("Failed to get 22b range ID"),
            range_24_bits_id: std
                .get_range_id(0, 0xFF_FFFF, None)
                .expect("Failed to get 24b range ID"),
        })
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Dma trace.
    /// * `input` - The operation data to process.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn process_slice(
        &self,
        input: &DmaInput,
        trace: &mut DmaInputCpyTraceRow<F>,
        local_7_bits_multiplicities: &mut [u32],
        local_22_bits_values: &mut Vec<u32>,
        local_24_bits_values: &mut Vec<u32>,
        local_24_bits_low_values: &mut [u32],
        local_rom_multiplicities: &mut [u64],
    ) {
        let count = DmaInfo::get_count(input.encoded);
        let count_lt_256 = count < 256;
        let count_ge_256 = 1 - count_lt_256 as usize;
        let h_count = ((count >> 8) - count_ge_256) as u32;
        trace.set_count_lt_256(count_lt_256);
        trace.set_h_count(h_count);
        let l_count = (count & 0xFF) as u16 + 256 * count_ge_256 as u16;
        trace.set_l_count(l_count);

        // to increase performance because the 99.99% of count is < 64K => h_count < 256
        if h_count < 256 {
            local_24_bits_low_values[h_count as usize] += 1;
        } else {
            local_24_bits_values.push(h_count);
        }

        let h_dst64 = input.dst >> 10;
        let l_dst64 = (input.dst >> 3) as u8 & 0x7F;

        trace.set_h_dst64(h_dst64);
        trace.set_l_dst64(l_dst64);
        trace.set_dst_offset(input.dst as u8 & 0x07);

        local_22_bits_values.push(h_dst64);
        local_7_bits_multiplicities[l_dst64 as usize] += 1;

        let rom_index = DmaRom::get_row(input.dst & 0x07, input.src & 0x07, count, false, false);

        local_rom_multiplicities[rom_index] += 1;

        trace.set_main_step(input.step);

        let pre_count = DmaInfo::get_pre_count(input.encoded) as u8;
        let loop_count = DmaInfo::get_loop_count(input.encoded);
        let post_count = DmaInfo::get_post_count(input.encoded);
        trace.set_use_pre(pre_count > 0);
        trace.set_use_loop(loop_count > 0);
        trace.set_use_post(post_count > 0);

        trace.set_pre_count(pre_count);
        trace.set_l_count64((l_count - pre_count as u16 - post_count as u16) >> 3);

        trace.set_sel_inputcpy(true);
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the DmaInputCpy trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_empty_slice(&self, trace: &mut DmaInputCpyTraceRow<F>) {
        trace.set_count_lt_256(true);
    }
}

impl<F: PrimeField64> DmaModule<F> for DmaInputCpySM<F> {
    fn get_name(&self) -> &'static str {
        "dma_inputcpy"
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
        inputs: &[Vec<DmaInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = DmaInputCpyTrace::<F>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        dma_trace("DmaInputCpy", total_inputs, num_rows);

        timer_start_trace!(DMA_TRACE);

        // Split the dma_trace.buffer into slices matching each inner vector’s length.
        let flat_inputs: Vec<_> = inputs.iter().flatten().collect();
        let trace_rows = trace.buffer.as_mut_slice();

        // Calculate optimal chunk size
        let num_threads = rayon::current_num_threads();
        let chunk_size = std::cmp::max(1, flat_inputs.len() / num_threads);

        // TODO: add new interface with u32 to std to be used with global_rom_multiplicities
        // Split the add256_trace.buffer into slices matching each inner vector’s length.
        let (
            global_7_bits_multiplicities,
            global_22_bits_values,
            global_24_bits_values,
            global_24_bits_low_values,
            global_rom_multiplicities,
        ) = flat_inputs
            .par_chunks(chunk_size)
            .zip(trace_rows.par_chunks_mut(chunk_size))
            .map(|(input_chunk, trace_chunk)| {
                // Local array shared by this chunk
                let mut local_7_bits_multiplicities = vec![0u32; 1 << 14];
                let mut local_22_bits_values = Vec::<u32>::with_capacity(inputs.len() * 2);
                let mut local_24_bits_values = Vec::<u32>::new();
                let mut local_24_bits_low_values = vec![0u32; 256];
                let mut local_rom_multiplicities = vec![0u64; DMA_ROM_WITHOUT_MEMCMP_SIZE];
                // Sum all local arrays into a global one
                for (input, trace_row) in input_chunk.iter().zip(trace_chunk.iter_mut()) {
                    self.process_slice(
                        input,
                        trace_row,
                        &mut local_7_bits_multiplicities,
                        &mut local_22_bits_values,
                        &mut local_24_bits_values,
                        &mut local_24_bits_low_values,
                        &mut local_rom_multiplicities,
                    );
                }
                (
                    local_7_bits_multiplicities,
                    local_22_bits_values,
                    local_24_bits_values,
                    local_24_bits_low_values,
                    local_rom_multiplicities,
                )
            })
            .reduce(
                // Identity: create empty accumulators
                || {
                    (
                        vec![0u32; 1 << 14],
                        Vec::new(),
                        Vec::new(),
                        vec![0u32; 256],
                        vec![0u64; DMA_ROM_WITHOUT_MEMCMP_SIZE],
                    )
                },
                // Combine two results
                |mut acc, local| {
                    // Merge multiplicities (element-wise addition)
                    for (i, &val) in local.0.iter().enumerate() {
                        acc.0[i] += val;
                    }
                    // Concatenate value vectors
                    acc.1.extend(local.1);
                    acc.2.extend(local.2);
                    // Merge low values (element-wise addition)
                    for (i, &val) in local.3.iter().enumerate() {
                        acc.3[i] += val;
                    }
                    for (i, &val) in local.4.iter().enumerate() {
                        acc.4[i] += val;
                    }
                    acc
                },
            );

        self.std.range_checks(self.range_7_bits_id, global_7_bits_multiplicities);
        self.std.range_checks(self.range_24_bits_id, global_24_bits_low_values);
        self.std.inc_virtual_rows_ranged(self.rom_table_id, &global_rom_multiplicities);

        for value in global_22_bits_values {
            self.std.range_check(self.range_22_bits_id, value as i64, 1);
        }
        for value in global_24_bits_values {
            self.std.range_check(self.range_24_bits_id, value as i64, 1);
        }

        if total_inputs < num_rows {
            self.process_empty_slice(&mut trace_rows[total_inputs]);
            let empty_row = trace_rows[total_inputs];
            trace_rows[total_inputs + 1..].par_iter_mut().for_each(|row| {
                *row = empty_row;
            });
        }

        timer_stop_and_log_trace!(DMA_TRACE);
        let from_trace = FromTrace::new(&mut trace);
        Ok(AirInstance::new_from_trace(from_trace))
    }
}
