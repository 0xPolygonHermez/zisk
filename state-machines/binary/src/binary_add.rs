//! The `BinaryAddSM` module implements the logic for the Binary Add State Machine.
//!
//! This state machine processes binary-related operations.

use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use rayon::prelude::*;
use zisk_pil::{BinaryAddTrace, BinaryAddTraceRow, BinaryAddTraceSplit};

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
    /// # Arguments
    /// * `std` - An `Arc`-wrapped reference to the PIL2 standard library.
    ///   Machine.
    ///
    /// # Returns
    /// A new `BinaryAddSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let range_id = std.get_range(0, 0xFFFF, None);

        // Create the BinaryAdd state machine
        Arc::new(Self { std, range_id })
    }

    /// Processes a slice of operation data, generating a trace row and updating multiplicities.
    ///
    /// # Arguments
    /// * `operation` - The operation data to process.
    ///
    /// # Returns
    /// A `BinaryAddTraceRow` representing the operation's result.
    #[inline(always)]
    pub fn process_slice(input: &[u64; 2]) -> (BinaryAddTraceRow<F>, [u64; 4]) {
        // Create an empty trace
        let mut row: BinaryAddTraceRow<F> = Default::default();

        // Execute the opcode
        let mut a = input[0];
        let mut b = input[1];
        let mut cin = 0;

        let mut range_checks = [0u64; 4];
        for i in 0..2 {
            let _a = a & 0xFFFF_FFFF;
            let _b = b & 0xFFFF_FFFF;
            let c = _a + _b + cin;
            let _c = c & 0xFFFF_FFFF;
            row.a[i] = F::from_u64(_a);
            row.b[i] = F::from_u64(_b);
            let c_chunks = [_c & 0xFFFF, _c >> 16];
            row.c_chunks[i * 2] = F::from_u64(c_chunks[0]);
            row.c_chunks[i * 2 + 1] = F::from_u64(c_chunks[1]);
            if c > MASK_U32 {
                row.cout[i] = F::ONE;
                cin = 1
            } else {
                row.cout[i] = F::ZERO;
                cin = 0
            };
            range_checks[i * 2] = c_chunks[0];
            range_checks[i * 2 + 1] = c_chunks[1];
            a >>= 32;
            b >>= 32;
        }
        // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
        row.multiplicity = F::ONE;

        // Return
        (row, range_checks)
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `trace_split` - A `BinaryAddTraceSplit` containing the binary add trace data.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness(&self, trace_split: BinaryAddTraceSplit<F>) -> AirInstance<F> {
        let padding_size = trace_split.leftover_size();

        let mut trace = BinaryAddTrace::<F>::from_split_struct(trace_split);

        let num_rows = trace.num_rows();

        let total_inputs = num_rows - padding_size;
        assert!(total_inputs <= num_rows);

        tracing::info!(
            "··· Creating BinaryAdd instance [{} / {} rows filled {:.2}%]",
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        // Split the add_e_trace.buffer into slices matching each inner vector’s length.
        self.std.range_check(0, 4 * (num_rows - total_inputs) as u64, self.range_id);

        // Note: We can choose any operation that trivially satisfies the constraints on padding
        // rows
        let padding_row = BinaryAddTraceRow::<F> { ..Default::default() };
        trace.row_slice_mut()[total_inputs..num_rows]
            .par_iter_mut()
            .for_each(|slot| *slot = padding_row);

        AirInstance::new_from_trace(FromTrace::new(&mut trace))
    }
}
