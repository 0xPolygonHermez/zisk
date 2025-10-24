use std::sync::Arc;

use fields::PrimeField64;
use rayon::prelude::*;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

#[cfg(not(feature = "packed"))]
use zisk_pil::{Add256Trace, Add256TraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{Add256TracePacked, Add256TraceRowPacked};

#[cfg(not(feature = "packed"))]
type Add256TraceRowType<F> = Add256TraceRow<F>;
#[cfg(feature = "packed")]
type Add256TraceRowType<F> = Add256TraceRowPacked<F>;

#[cfg(not(feature = "packed"))]
type Add256TraceType<F> = Add256Trace<F>;
#[cfg(feature = "packed")]
type Add256TraceType<F> = Add256TracePacked<F>;

use super::Add256Input;

/// The `Add256SM` struct encapsulates the logic of the Add256 State Machine.
pub struct Add256SM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Number of available add256s in the trace.
    pub num_availables: usize,

    /// Range checks ID's
    range_id: usize,
}

impl<F: PrimeField64> Add256SM<F> {
    /// Creates a new Add256 State Machine instance.
    ///
    /// # Returns
    /// A new `Add256SM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Compute some useful values
        let num_availables = Add256TraceType::<F>::NUM_ROWS;

        let range_id = std.get_range_id(0, (1 << 16) - 1, None);

        Arc::new(Self { std, num_availables, range_id })
    }

    /// Processes a slice of operation data, updating the trace.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Add256 trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_slice(
        &self,
        input: &Add256Input,
        trace: &mut Add256TraceRowType<F>,
        multiplicities: &mut [u32],
    ) {
        trace.set_cin(input.cin != 0);
        let mut cout_2 = input.cin as u32;

        for i in 0..4 {
            let al = input.a[i] as u32;
            let ah = (input.a[i] >> 32) as u32;

            let bl = input.b[i] as u32;
            let bh = (input.b[i] >> 32) as u32;

            trace.set_a(i, 0, al);
            trace.set_a(i, 1, ah);
            trace.set_b(i, 0, bl);
            trace.set_b(i, 1, bh);
            let cl = al as u64 + bl as u64 + cout_2 as u64;
            let cout_1 = cl >> 32;
            let ch = ah as u64 + bh as u64 + cout_1;
            cout_2 = (ch >> 32) as u32;

            let cll = cl as u16;
            let clh = (cl >> 16) as u16;
            let chl = ch as u16;
            let chh = (ch >> 16) as u16;

            trace.set_c_chunks(i, 0, cll);
            trace.set_c_chunks(i, 1, clh);
            trace.set_c_chunks(i, 2, chl);
            trace.set_c_chunks(i, 3, chh);

            trace.set_cout(i, 0, cout_1 != 0);
            trace.set_cout(i, 1, cout_2 != 0);

            multiplicities[cll as usize] += 1;
            multiplicities[clh as usize] += 1;
            multiplicities[chl as usize] += 1;
            multiplicities[chh as usize] += 1;
        }
        trace.set_addr_params(input.addr_main);
        trace.set_addr_a(input.addr_a);
        trace.set_addr_b(input.addr_b);
        trace.set_addr_c(input.addr_c);
        trace.set_step(input.step_main);
        trace.set_sel(true);
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
        inputs: &[Vec<Add256Input>],
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut trace = Add256TraceType::<F>::new_from_vec(trace_buffer);

        let num_rows = trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        tracing::info!(
            "··· Creating Add256 instance [{} / {} rows filled {:.2}%]",
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(ADD256_TRACE);

        // Split the add256_trace.buffer into slices matching each inner vector’s length.
        let flat_inputs: Vec<_> = inputs.iter().flatten().collect();
        let trace_rows = trace.buffer.as_mut_slice();

        // Determinar tamaño óptimo de chunks
        let num_threads = rayon::current_num_threads();
        let chunk_size = std::cmp::max(1, flat_inputs.len() / num_threads);

        // Procesar en chunks para compartir arrays locales de multiplicities
        let local_multiplicities_vec: Vec<Vec<u32>> = flat_inputs
            .par_chunks(chunk_size)
            .zip(trace_rows.par_chunks_mut(chunk_size))
            .map(|(input_chunk, trace_chunk)| {
                // Array local compartido por este chunk
                let mut local_multiplicities = vec![0u32; 1 << 16];

                // Procesar todos los inputs del chunk
                for (input, trace_row) in input_chunk.iter().zip(trace_chunk.iter_mut()) {
                    self.process_slice(input, trace_row, &mut local_multiplicities);
                }

                local_multiplicities
            })
            .collect();

        // Sumar todos los arrays locales en uno global
        let mut global_multiplicities = vec![0u32; 1 << 16];
        for local_multiplicities in local_multiplicities_vec {
            for (i, count) in local_multiplicities.iter().enumerate() {
                global_multiplicities[i] += count;
            }
        }

        // Enviar el resultado final al std
        self.std.range_checks(self.range_id, global_multiplicities);

        timer_stop_and_log_trace!(ADD256_TRACE);

        let padding_row = Add256TraceRowType::<F>::default();
        trace.buffer[total_inputs..num_rows].par_iter_mut().for_each(|slot| *slot = padding_row);

        AirInstance::<F>::new_from_trace(FromTrace::new(&mut trace))
    }
}
