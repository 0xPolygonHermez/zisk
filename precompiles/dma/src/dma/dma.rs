use std::sync::Arc;

use fields::PrimeField64;
use rayon::prelude::*;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::{DMA_ROM_ID, DUAL_RANGE_7_BITS_ID};

use crate::{dma::dma_rom::DmaRom, DmaInput};
use precompiles_helpers::DmaInfo;

#[cfg(feature = "packed")]
pub use zisk_pil::{DmaTracePacked, DmaTraceRowPacked};

#[cfg(not(feature = "packed"))]
pub use zisk_pil::{DmaTrace, DmaTraceRow};

#[cfg(feature = "packed")]
type DmaTraceRowType<F> = DmaTraceRowPacked<F>;
#[cfg(feature = "packed")]
type DmaTraceType<F> = DmaTracePacked<F>;

#[cfg(not(feature = "packed"))]
type DmaTraceRowType<F> = DmaTraceRow<F>;
#[cfg(not(feature = "packed"))]
type DmaTraceType<F> = DmaTrace<F>;

/// The `DmaSM` struct encapsulates the logic of the Dma State Machine.
pub struct DmaSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    pub rom_table_id: usize,
    pub dual_range_7_bits_id: usize,
    pub range_22_bits_id: usize,
    pub range_24_bits_id: usize,
}

impl<F: PrimeField64> DmaSM<F> {
    /// Creates a new Dma State Machine instance.
    ///
    /// # Returns
    /// A new `DmaSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self {
            std: std.clone(),
            rom_table_id: std.get_virtual_table_id(DMA_ROM_ID).expect("Failed to get dma rom ID"),
            dual_range_7_bits_id: std
                .get_virtual_table_id(DUAL_RANGE_7_BITS_ID)
                .expect("Failed to get dual 7-bits table ID"),
            range_22_bits_id: std
                .get_range_id(0, 0x3F_FFFF, None)
                .expect("Failed to get 22b table ID"),
            range_24_bits_id: std
                .get_range_id(0, 0xFF_FFFF, None)
                .expect("Failed to get 24b table ID"),
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
        trace: &mut DmaTraceRowType<F>,
        local_dual_7_bits_multiplicities: &mut [u64],
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

        let h_src64 = input.src >> 10;
        let h_dst64 = input.dst >> 10;
        let l_src64 = (input.src >> 3) as u8 & 0x7F;
        let l_dst64 = (input.dst >> 3) as u8 & 0x7F;

        trace.set_h_src64(h_src64);
        trace.set_l_src64(l_src64);
        let src_offset = input.src as u8 & 0x07;
        trace.set_src_offset(src_offset);

        trace.set_h_dst64(h_dst64);
        trace.set_l_dst64(l_dst64);
        trace.set_dst_offset(input.dst as u8 & 0x07);

        local_22_bits_values.push(h_src64);
        local_22_bits_values.push(h_dst64);
        let dual_7_bits_row = ((l_src64 as usize) << 7) | l_dst64 as usize;
        local_dual_7_bits_multiplicities[dual_7_bits_row] += 1;
        // println!(
        //     "local_dual_7_bits_multiplicities[{dual_7_bits_row} ({l_src64}|{l_dst64})] = {}",
        //     local_dual_7_bits_multiplicities[dual_7_bits_row]
        // );

        local_rom_multiplicities[DmaRom::get_row(input.dst & 0x07, input.src & 0x07, count)] += 1;

        trace.set_main_step(input.step);

        let pre_count = DmaInfo::get_pre_count(input.encoded) as u8;
        let loop_count = DmaInfo::get_loop_count(input.encoded);
        let post_count = DmaInfo::get_post_count(input.encoded);
        trace.set_use_pre(pre_count > 0);
        trace.set_use_memcpy(loop_count > 0);
        trace.set_use_post(post_count > 0);

        trace.set_src64_inc_by_pre(DmaInfo::get_src64_inc_by_pre(input.encoded) > 0);

        trace.set_pre_count(pre_count);
        trace.set_l_count64((l_count - pre_count as u16 - post_count as u16) >> 3);
        trace.set_src_offset_after_pre((src_offset + pre_count) % 8);

        trace.set_sel(true);
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Dma trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_empty_slice(&self, trace: &mut DmaTraceRowType<F>) {
        trace.set_count_lt_256(true);
        trace.set_h_count(0);
        trace.set_l_count(0);

        // to increase performance because the 99.99% of count is < 64K => h_count < 256
        trace.set_h_src64(0);
        trace.set_l_src64(0);
        trace.set_src_offset(0);

        trace.set_h_dst64(0);
        trace.set_l_dst64(0);
        trace.set_dst_offset(0);

        trace.set_main_step(0);

        trace.set_use_pre(false);
        trace.set_use_memcpy(false);
        trace.set_use_post(false);

        trace.set_src64_inc_by_pre(false);

        trace.set_pre_count(0);
        trace.set_l_count64(0);
        trace.set_src_offset_after_pre(0);

        trace.set_sel(false);
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
        inputs: &[Vec<DmaInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = DmaTraceType::<F>::new_from_vec(trace_buffer)?;
        let num_rows = trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        tracing::debug!(
            "··· Creating Dma instance [{total_inputs} / {num_rows} rows filled {:.2}%]",
            total_inputs as f64 / num_rows as f64 * 100.0
        );

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
            global_dual_7_bits_multiplicities,
            global_22_bits_values,
            global_24_bits_values,
            global_24_bits_low_values,
            global_rom_multiplicities,
        ) = flat_inputs
            .par_chunks(chunk_size)
            .zip(trace_rows.par_chunks_mut(chunk_size))
            .map(|(input_chunk, trace_chunk)| {
                // Local array shared by this chunk
                let mut local_dual_7_bits_multiplicities = vec![0u64; 1 << 14];
                let mut local_22_bits_values = Vec::<u32>::with_capacity(inputs.len() * 2);
                let mut local_24_bits_values = Vec::<u32>::new();
                let mut local_24_bits_low_values = vec![0u32; 256];
                let mut local_rom_multiplicities = vec![0u64; 1 << 15];
                // Sum all local arrays into a global one
                for (input, trace_row) in input_chunk.iter().zip(trace_chunk.iter_mut()) {
                    self.process_slice(
                        input,
                        trace_row,
                        &mut local_dual_7_bits_multiplicities,
                        &mut local_22_bits_values,
                        &mut local_24_bits_values,
                        &mut local_24_bits_low_values,
                        &mut local_rom_multiplicities,
                    );
                }
                (
                    local_dual_7_bits_multiplicities,
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
                        vec![0u64; 1 << 14],
                        Vec::new(),
                        Vec::new(),
                        vec![0u32; 256],
                        vec![0u64; 1 << 15],
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

        // for (index, value) in global_dual_7_bits_multiplicities.iter().enumerate() {
        //     if *value != 0 {
        //         println!("DUAL_7_BITS[{index}]={value}")
        //     }
        // }
        self.std
            .inc_virtual_rows_ranged(self.dual_range_7_bits_id, &global_dual_7_bits_multiplicities);
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
