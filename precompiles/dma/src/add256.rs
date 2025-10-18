use std::sync::Arc;

use fields::PrimeField64;
use rayon::prelude::*;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::{Add256Trace, Add256TraceRow};

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
        let num_availables = Add256Trace::<usize>::NUM_ROWS;

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
        trace: &mut Add256TraceRow<F>,
        multiplicities: &mut [u32],
    ) {
        trace.cin = F::from_bool(input.cin != 0);
        let mut cout_2 = input.cin as u32;

        for i in 0..4 {
            let al = input.a[i] as u32;
            let ah = (input.a[i] >> 32) as u32;

            let bl = input.b[i] as u32;
            let bh = (input.b[i] >> 32) as u32;

            trace.a[i][0] = F::from_u32(al);
            trace.a[i][1] = F::from_u32(ah);
            trace.b[i][0] = F::from_u32(bl);
            trace.b[i][1] = F::from_u32(bh);
            let cl = al as u64 + bl as u64 + cout_2 as u64;
            let cout_1 = cl >> 32;
            let ch = ah as u64 + bh as u64 + cout_1;
            cout_2 = (ch >> 32) as u32;

            let cll = cl as u16;
            let clh = (cl >> 16) as u16;
            let chl = ch as u16;
            let chh = (ch >> 16) as u16;

            trace.c_chunks[i][0] = F::from_u16(cll);
            trace.c_chunks[i][1] = F::from_u16(clh);
            trace.c_chunks[i][2] = F::from_u16(chl);
            trace.c_chunks[i][3] = F::from_u16(chh);

            trace.cout[i][0] = F::from_u8(cout_1 as u8);
            trace.cout[i][1] = F::from_u8(cout_2 as u8);

            multiplicities[cll as usize] += 1;
            multiplicities[clh as usize] += 1;
            multiplicities[chl as usize] += 1;
            multiplicities[chh as usize] += 1;
        }
        trace.addr_params = F::from_u32(input.addr_main);
        trace.addr_a = F::from_u32(input.addr_a);
        trace.addr_b = F::from_u32(input.addr_b);
        trace.addr_c = F::from_u32(input.addr_c);
        trace.step = F::from_u64(input.step_main);
        trace.sel = F::ONE;
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
        let mut trace = Add256Trace::new_from_vec(trace_buffer);

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
        let trace_rows = trace.row_slice_mut();

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

        trace.row_slice_mut()[total_inputs..num_rows]
            .par_iter_mut()
            .for_each(|slot| *slot = Add256TraceRow::<F> { ..Default::default() });

        AirInstance::<F>::new_from_trace(FromTrace::new(&mut trace))
    }
}
