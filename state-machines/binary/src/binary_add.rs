//! The `BinaryAddSM` module implements the logic for the Binary Add State Machine.
//!
//! This state machine processes binary-related operations.

use std::sync::Arc;

use log::info;
use p3_field::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use rayon::prelude::*;
use zisk_pil::{BinaryAddTrace, BinaryAddTraceRow};

const MASK_U32: u64 = 0x0000_0000_FFFF_FFFF;

/// The `BinaryAddSM` struct encapsulates the logic of the Binary Add State Machine.
pub struct BinaryAddSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,
    range_id: usize,
}

impl<F: PrimeField64> BinaryAddSM<F> {
    const MY_NAME: &'static str = "BinaryAdd";

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
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    ///
    /// # Returns
    /// A `BinaryAddTraceRow` representing the operation's result.
    #[inline(always)]
    pub fn process_slice(&self, input: &[u64; 2]) -> BinaryAddTraceRow<F> {
        // Create an empty trace
        let mut row: BinaryAddTraceRow<F> = Default::default();

        // Execute the opcode
        let mut a = input[0];
        let mut b = input[1];
        let mut cin = 0;

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
            self.std.range_check(c_chunks[0] as i64, 1, self.range_id);
            self.std.range_check(c_chunks[1] as i64, 1, self.range_id);
            a >>= 32;
            b >>= 32;
        }
        // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
        row.multiplicity = F::ONE;

        // Return
        row
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `operations` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness(&self, inputs: &[Vec<[u64; 2]>]) -> AirInstance<F> {
        let mut add_trace = BinaryAddTrace::new();

        let num_rows = add_trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        info!(
            "{}: ··· Creating BinaryAdd instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        // Split the add_e_trace.buffer into slices matching each inner vector’s length.
        let sizes: Vec<usize> = inputs.iter().map(|v| v.len()).collect();
        let mut slices = Vec::with_capacity(inputs.len());
        let mut rest = add_trace.buffer.as_mut_slice();
        for size in sizes {
            let (head, tail) = rest.split_at_mut(size);
            slices.push(head);
            rest = tail;
        }

        // Process each slice in parallel, and use the corresponding inner input from `inputs`.
        slices.into_par_iter().enumerate().for_each(|(i, slice)| {
            //let std = self.std.clone();
            slice.iter_mut().enumerate().for_each(|(j, trace_row)| {
                *trace_row = self.process_slice(&inputs[i][j]);
            });
        });
        // Note: We can choose any operation that trivially satisfies the constraints on padding
        // rows
        let padding_row = BinaryAddTraceRow::<F> { ..Default::default() };

        add_trace.buffer[total_inputs..num_rows].fill(padding_row);
        self.std.range_check(0, 4 * (num_rows - total_inputs) as u64, self.range_id);

        AirInstance::new_from_trace(FromTrace::new(&mut add_trace))
    }
}
