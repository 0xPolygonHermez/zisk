//! The `BinaryAddSM` module implements the logic for the Binary Add State Machine.
//!
//! This state machine processes binary-related operations.

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use rayon::prelude::*;
use std::sync::Arc;
use zisk_pil::{BinaryAddAirValues, BinaryAddTrace, BinaryAddTraceRowOps};

const MASK_U32: u64 = 0x0000_0000_FFFF_FFFF;

/// The `BinaryAddSM` struct encapsulates the logic of the Binary Add State Machine.
pub struct BinaryAddSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,
    range_id: usize,
}

impl<F: PrimeField64> BinaryAddSM<F> {
    /// Creates a new BinaryAdd State Machine instance.
    ///
    /// # Arguments/// * `std` - An `Arc`-wrapped reference to the PIL2 standard library.
    ///   Machine.
    ///
    /// # Returns
    /// A new `BinaryAddSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let range_id = std.get_range_id(0, 0xFFFF, None).expect("Failed to get range ID");

        // Create the BinaryAdd state machine
        Arc::new(Self { std, range_id })
    }

    /// Processes a slice of operation data, generating a trace row and updating multiplicities.
    ///
    /// # Arguments
    /// * `operation` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    ///
    /// # Returns
    /// A `BinaryAddTraceRow` representing the operation's result.
    #[inline(always)]
    pub fn process_slice<R: BinaryAddTraceRowOps<F>>(
        &self,
        row: &mut R,
        input: &[u64; 2],
    ) -> [u64; 4] {
        // Execute the opcode
        let a = input[0];
        let b = input[1];
        let mut cin = 0u64;

        // Compute all values first
        let mut a_values = [0u32; 2];
        let mut b_values = [0u32; 2];
        let mut c_chunks_values = [0u16; 4];
        let mut cout_values = [false; 2];
        let mut range_checks = [0u64; 4];

        for i in 0..2 {
            // Extract the appropriate 32-bit chunk for this iteration
            let _a = if i == 0 { a & 0xFFFF_FFFF } else { a >> 32 };
            let _b = if i == 0 { b & 0xFFFF_FFFF } else { b >> 32 };
            let c = _a + _b + cin;
            let _c = c & 0xFFFF_FFFF;

            a_values[i] = _a as u32;
            b_values[i] = _b as u32;

            // Split result into two 16-bit chunks (indices: i=0 -> 0,1; i=1 -> 2,3)
            c_chunks_values[i * 2] = (_c & 0xFFFF) as u16;
            c_chunks_values[i * 2 + 1] = (_c >> 16) as u16;

            // Update carry for next iteration
            cin = if c > MASK_U32 { 1 } else { 0 };
            cout_values[i] = cin != 0;

            range_checks[i * 2] = c_chunks_values[i * 2] as u64;
            range_checks[i * 2 + 1] = c_chunks_values[i * 2 + 1] as u64;
        }

        // Set all values at once using bulk setters
        row.set_all_a(&a_values);
        row.set_all_b(&b_values);
        row.set_all_c_chunks(&c_chunks_values);
        row.set_all_cout(&cout_values);

        // Return
        range_checks
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `operations` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness<R: BinaryAddTraceRowOps<F>>(
        &self,
        inputs: &[Vec<[u64; 2]>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut add_trace = BinaryAddTrace::<R>::new_from_vec(trace_buffer)?;

        let num_rows = add_trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        debug_assert!(total_inputs <= num_rows);

        tracing::debug!(
            "··· Creating BinaryAdd instance [{} / {} rows filled {:.2}%]",
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        // Split the add_e_trace.buffer into slices matching each inner vector’s length.
        let flat_inputs: Vec<_> = inputs.iter().flatten().collect();
        let mut range_checks: Vec<[u64; 4]> = vec![[0u64; 4]; flat_inputs.len()];

        // Process each slice in parallel, and use the corresponding inner input from `inputs`.
        flat_inputs
            .into_par_iter()
            .zip(add_trace.buffer.par_iter_mut())
            .zip(range_checks.par_iter_mut())
            .for_each(|((input, trace_row), range_check)| {
                let checks = self.process_slice::<R>(trace_row, input);
                *range_check = checks;
            });

        let mut multiplicities = vec![0u32; 0xFFFF + 1];
        for range_check in range_checks {
            multiplicities[range_check[0] as usize] += 1;
            multiplicities[range_check[1] as usize] += 1;
            multiplicities[range_check[2] as usize] += 1;
            multiplicities[range_check[3] as usize] += 1;
        }
        multiplicities[0] += 4 * (num_rows - total_inputs) as u32;

        self.std.range_checks(self.range_id, multiplicities);

        // Set 0 + 0 as the padding row
        let padding_size = num_rows - total_inputs;
        if padding_size > 0 {
            let padding_row = R::default();
            add_trace.buffer[total_inputs..num_rows]
                .par_iter_mut()
                .for_each(|slot| *slot = padding_row);
        }

        let mut air_values = BinaryAddAirValues::<F>::new();
        air_values.padding_size = F::from_usize(padding_size);
        Ok(AirInstance::new_from_trace(
            FromTrace::new(&mut add_trace).with_air_values(&mut air_values),
        ))
    }
}
