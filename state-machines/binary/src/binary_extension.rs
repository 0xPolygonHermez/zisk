//! The `BinaryExtensionSM` module defines the Binary Extension State Machine.
//!
//! This state machine handles binary extension-related operations, computes traces, and manages
//! range checks and multiplicities for table rows based on the operations provided.

use std::sync::Arc;

use crate::{BinaryExtensionFrops, BinaryExtensionTableOp, BinaryExtensionTableSM, BinaryInput};

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use rayon::prelude::*;
use zisk_core::zisk_ops::ZiskOp;
#[cfg(not(feature = "packed"))]
use zisk_pil::{BinaryExtensionTrace, BinaryExtensionTraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{BinaryExtensionTracePacked, BinaryExtensionTraceRowPacked};

#[cfg(feature = "packed")]
type BinaryExtensionTraceRowType<F> = BinaryExtensionTraceRowPacked<F>;
#[cfg(feature = "packed")]
type BinaryExtensionTraceType<F> = BinaryExtensionTracePacked<F>;

#[cfg(not(feature = "packed"))]
type BinaryExtensionTraceRowType<F> = BinaryExtensionTraceRow<F>;
#[cfg(not(feature = "packed"))]
type BinaryExtensionTraceType<F> = BinaryExtensionTrace<F>;

// Constants for bit masks and operations.
const MASK_32: u64 = 0xFFFFFFFF;
const MASK_64: u64 = 0xFFFFFFFFFFFFFFFF;

const SE_MASK_32: u64 = 0xFFFFFFFF00000000;
const SE_MASK_16: u64 = 0xFFFFFFFFFFFF0000;
const SE_MASK_8: u64 = 0xFFFFFFFFFFFFFF00;

const SIGN_32_BIT: u64 = 0x80000000;
const SIGN_BYTE: u64 = 0x80;

const LS_5_BITS: u64 = 0x1F;
const LS_6_BITS: u64 = 0x3F;

const SE_W_OP: u8 = 0x39;

/// The `BinaryExtensionSM` struct defines the Binary Extension State Machine.
///
/// It processes binary extension-related operations and generates necessary traces and multiplicity
/// tables for the operations. It also manages range checks through the PIL2 standard library.
pub struct BinaryExtensionSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,

    /// The range check ID
    range_id: usize,

    /// The table ID for the Binary Basic State Machine
    table_id: usize,

    /// The table ID for the Binary Extension FROPS
    frops_table_id: usize,
}

impl<F: PrimeField64> BinaryExtensionSM<F> {
    /// Creates a new instance of the `BinaryExtensionSM`.
    ///
    /// # Arguments
    /// * `std` - An `Arc`-wrapped reference to the PIL2 standard library.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `BinaryExtensionSM`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Get the range check ID
        let range_id = std.get_range_id(0, 0xFFFFFF, None);

        // Get the table ID
        let table_id = std.get_virtual_table_id(BinaryExtensionTableSM::TABLE_ID);

        // Get the FROPS table ID
        let frops_table_id = std.get_virtual_table_id(BinaryExtensionFrops::TABLE_ID);

        Arc::new(Self { std, range_id, table_id, frops_table_id })
    }

    /// Determines if the given opcode represents a shift operation.
    fn opcode_is_shift(opcode: ZiskOp) -> bool {
        match opcode {
            ZiskOp::Sll
            | ZiskOp::Srl
            | ZiskOp::Sra
            | ZiskOp::SllW
            | ZiskOp::SrlW
            | ZiskOp::SraW => true,

            ZiskOp::SignExtendB | ZiskOp::SignExtendH | ZiskOp::SignExtendW => false,

            _ => panic!("BinaryExtensionSM::opcode_is_shift() got invalid opcode={opcode:?}"),
        }
    }

    /// Determines if the given opcode represents a shift word operation.
    fn opcode_is_shift_word(opcode: ZiskOp) -> bool {
        match opcode {
            ZiskOp::SllW | ZiskOp::SrlW | ZiskOp::SraW => true,

            ZiskOp::Sll
            | ZiskOp::Srl
            | ZiskOp::Sra
            | ZiskOp::SignExtendB
            | ZiskOp::SignExtendH
            | ZiskOp::SignExtendW => false,

            _ => panic!("BinaryExtensionSM::opcode_is_shift() got invalid opcode={opcode:?}"),
        }
    }

    /// Processes a single operation and generates the corresponding trace row.
    ///
    /// # Arguments
    /// * `operation` - The operation to process.
    /// * `multiplicity` - A mutable reference to the multiplicity table to update.
    /// * `range_check` - A mutable reference to the range check table to update.
    ///
    /// # Returns
    /// A `BinaryExtensionTraceRow` representing the processed trace.
    pub fn process_slice(&self, input: &BinaryInput) -> BinaryExtensionTraceRowType<F> {
        // Get a ZiskOp from the code
        let opcode = ZiskOp::try_from_code(input.op).expect("Invalid ZiskOp opcode");

        // Create an empty trace
        let mut row = BinaryExtensionTraceRowType::default();
        row.set_op(input.op);

        // Set if the opcode is a shift operation
        let op_is_shift = Self::opcode_is_shift(opcode);
        row.set_op_is_shift(op_is_shift);

        // Set if the opcode is a shift word operation
        let op_is_shift_word = Self::opcode_is_shift_word(opcode);

        // Detect if this is a sign extend operation
        let a_val = if op_is_shift { input.a } else { input.b };
        let b_val = if op_is_shift { input.b } else { input.a };

        // Split a in bytes and store them in in1
        let a_bytes: [u8; 8] = a_val.to_le_bytes();
        for (i, value) in a_bytes.iter().enumerate() {
            row.set_in1(i, *value);
        }

        // Store b low part into in2_low
        let in2_low: u64 = if op_is_shift { b_val & 0xFF } else { 0 };
        row.set_in2_low(in2_low as u8);

        // Store b lower bits when shifting, depending on operation size
        let b_low = if op_is_shift_word { b_val & LS_5_BITS } else { b_val & LS_6_BITS };

        // Store b into in2
        let in2_0: u32 = if op_is_shift {
            ((b_val >> 8) & 0xFFFFFF) as u32
        } else {
            (b_val & 0xFFFFFFFF) as u32
        };
        let in2_1: u32 = ((b_val >> 32) & 0xFFFFFFFF) as u32;

        row.set_in2(0, in2_0);
        row.set_in2(1, in2_1);

        // Calculate the trace output
        let mut t_out: [[u64; 2]; 8] = [[0; 2]; 8];

        // Calculate output based on opcode
        let binary_extension_table_op: BinaryExtensionTableOp;
        match opcode {
            ZiskOp::Sll => {
                binary_extension_table_op = BinaryExtensionTableOp::Sll;
                for j in 0..8 {
                    let bits_to_shift = b_low + 8 * j as u64;
                    let out =
                        if bits_to_shift < 64 { (a_bytes[j] as u64) << bits_to_shift } else { 0 };
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::Srl => {
                binary_extension_table_op = BinaryExtensionTableOp::Srl;
                for j in 0..8 {
                    let out = ((a_bytes[j] as u64) << (8 * j as u64)) >> b_low;
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::Sra => {
                binary_extension_table_op = BinaryExtensionTableOp::Sra;
                for j in 0..8 {
                    let mut out = ((a_bytes[j] as u64) << (8 * j as u64)) >> b_low;
                    if j == 7 {
                        // most significant bit of most significant byte define if negative or not
                        // if negative then add b bits one on the left
                        if ((a_bytes[j] as u64) & SIGN_BYTE) != 0 && (b_low != 0) {
                            out |= MASK_64 << (64 - b_low);
                        }
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SllW => {
                binary_extension_table_op = BinaryExtensionTableOp::SllW;
                for j in 0..8 {
                    let mut out: u64;
                    if j >= 4 {
                        out = 0;
                    } else {
                        out = (((a_bytes[j] as u64) << b_low) << (8 * j as u64)) & MASK_32;
                        if (out & SIGN_32_BIT) != 0 {
                            out |= SE_MASK_32;
                        }
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SrlW => {
                binary_extension_table_op = BinaryExtensionTableOp::SrlW;
                for j in 0..8 {
                    let mut out: u64;
                    if j >= 4 {
                        out = 0;
                    } else {
                        out = (((a_bytes[j] as u64) << (8 * j as u64)) >> b_low) & MASK_32;
                        if (out & SIGN_32_BIT) != 0 {
                            out |= SE_MASK_32;
                        }
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SraW => {
                binary_extension_table_op = BinaryExtensionTableOp::SraW;
                for j in 0..8 {
                    let mut out: u64;
                    if j >= 4 {
                        out = 0;
                    } else {
                        out = ((a_bytes[j] as u64) << (8 * j as u64)) >> b_low;
                        if j == 3 && ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                            out |= MASK_64 << (32 - b_low);
                        }
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SignExtendB => {
                binary_extension_table_op = BinaryExtensionTableOp::SignExtendB;
                for j in 0..8 {
                    let out: u64;
                    if j == 0 {
                        if ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                            out = (a_bytes[j] as u64) | SE_MASK_8;
                        } else {
                            out = a_bytes[j] as u64;
                        }
                    } else {
                        out = 0;
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SignExtendH => {
                binary_extension_table_op = BinaryExtensionTableOp::SignExtendH;
                for j in 0..8 {
                    let out: u64;
                    if j == 0 {
                        out = a_bytes[j] as u64;
                    } else if j == 1 {
                        if ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                            out = ((a_bytes[j] as u64) << 8) | SE_MASK_16;
                        } else {
                            out = (a_bytes[j] as u64) << 8;
                        }
                    } else {
                        out = 0;
                    }
                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            ZiskOp::SignExtendW => {
                binary_extension_table_op = BinaryExtensionTableOp::SignExtendW;
                for j in 0..4 {
                    let mut out = (a_bytes[j] as u64) << (8 * j as u64);
                    if j == 3 && ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                        out |= SE_MASK_32;
                    }

                    t_out[j][0] = out & 0xffffffff;
                    t_out[j][1] = (out >> 32) & 0xffffffff;
                }
            }
            _ => panic!("BinaryExtensionSM::process_slice() found invalid opcode={}", input.op),
        }

        // Convert the trace output to field elements
        for (j, out) in t_out.iter().enumerate() {
            row.set_out(j, 0, out[0] as u32);
            row.set_out(j, 1, out[1] as u32);
        }

        // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
        row.set_multiplicity(true);

        for (i, a_byte) in a_bytes.iter().enumerate() {
            let row = BinaryExtensionTableSM::calculate_table_row(
                binary_extension_table_op,
                i as u64,
                *a_byte as u64,
                in2_low,
            );
            self.std.inc_virtual_row(self.table_id, row, 1);
        }

        // Return successfully
        row
    }

    /// Computes the witness for the given set of operations.
    ///
    /// # Arguments
    /// * `operations` - The list of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` representing the computed witness.
    pub fn compute_witness(
        &self,
        inputs: &[Vec<BinaryInput>],
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut binary_e_trace = BinaryExtensionTraceType::new_from_vec(trace_buffer);

        let num_rows = binary_e_trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(
            total_inputs <= num_rows,
            "{} <= {} ({})",
            total_inputs,
            num_rows,
            BinaryExtensionTraceType::<F>::NUM_ROWS
        );

        tracing::info!(
            "··· Creating Binary Extension instance [{} / {} rows filled {:.2}%]",
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        // Split the binary_e_trace.buffer into slices matching each inner vector’s length.
        let sizes: Vec<usize> = inputs.iter().map(|v| v.len()).collect();
        let mut slices = Vec::with_capacity(inputs.len());
        let mut rest = &mut binary_e_trace.buffer[..];
        for size in sizes {
            let (head, tail) = rest.split_at_mut(size);
            slices.push(head);
            rest = tail;
        }

        // Process each slice in parallel, and use the corresponding inner input from `inputs`.
        slices.into_par_iter().enumerate().for_each(|(i, slice)| {
            slice.iter_mut().enumerate().for_each(|(j, trace_row)| {
                *trace_row = self.process_slice(&inputs[i][j]);
            });
        });

        // Iterate over all inputs and check opcode
        // to update multiplicity for the corresponding table row.
        for row in inputs.iter() {
            for input in row.iter() {
                let opcode = ZiskOp::try_from_code(input.op).expect("Invalid ZiskOp opcode");
                let op_is_shift = Self::opcode_is_shift(opcode);
                if op_is_shift {
                    let row = (input.b >> 8) & 0xFFFFFF;
                    self.std.range_check(self.range_id, row as i64, 1);
                }
            }
        }

        // Note: We can choose any operation that trivially satisfies the constraints on padding
        // rows
        let mut padding_row = BinaryExtensionTraceRowType::default();
        padding_row.set_op(SE_W_OP);

        binary_e_trace.buffer[total_inputs..num_rows]
            .par_iter_mut()
            .for_each(|slot| *slot = padding_row);

        let padding_size = num_rows - total_inputs;
        for i in 0..8 {
            let multiplicity = padding_size as u64;
            let row = BinaryExtensionTableSM::calculate_table_row(
                BinaryExtensionTableOp::SignExtendW,
                i,
                0,
                0,
            );
            self.std.inc_virtual_row(self.table_id, row, multiplicity);
        }

        AirInstance::new_from_trace(FromTrace::new(&mut binary_e_trace))
    }
    pub fn compute_frops(&self, frops_inputs: &Vec<u32>) {
        for row in frops_inputs {
            self.std.inc_virtual_row(self.frops_table_id, *row as u64, 1);
        }
    }
}
