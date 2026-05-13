//! The `BinaryExtensionSM` module defines the Binary Extension State Machine.
//!
//! This state machine handles binary extension-related operations, computes traces, and manages
//! range checks and multiplicities for table rows based on the operations provided.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{binary_constants::*, BinaryExtensionTableOp, BinaryExtensionTableSM, BinaryInput};

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use rayon::prelude::*;
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{BinaryExtensionAirValues, BinaryExtensionTrace, BinaryExtensionTraceRowOps};

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

const TABLE_ROW_SPAN: usize = BinaryExtensionTableSM::MAX_TABLE_ROW as usize + 1;

thread_local! {
    static TL_COUNTS: RefCell<Vec<u64>> = RefCell::new(vec![0u64; TABLE_ROW_SPAN]);
    static TL_DIRTY:  RefCell<Vec<u32>> = RefCell::new(Vec::with_capacity(32768));
}

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
        let range_id = std.get_range_id(0, 0xFFFFFF, None).expect("Failed to get range ID");

        // Get the table ID
        let table_id = std
            .get_virtual_table_id(BinaryExtensionTableSM::TABLE_ID)
            .expect("Failed to get table ID");

        Arc::new(Self { std, range_id, table_id })
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
    pub fn process_slice<R: BinaryExtensionTraceRowOps<F>>(
        &self,
        input: &BinaryInput,
        counts: &mut Vec<u64>,
        dirty: &mut Vec<u32>,
    ) -> R {
        // Get a ZiskOp from the code
        let opcode = ZiskOp::try_from_code(input.op).expect("Invalid ZiskOp opcode");

        // Create an empty trace
        let mut row: R = Default::default();
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
        row.set_all_free_in_a(&a_bytes);

        // Store b low part into in2_low
        let in2_low: u64 = if op_is_shift { b_val & 0xFF } else { 0 };
        row.set_free_in_b(in2_low as u8);

        // Store b lower bits when shifting, depending on operation size
        let b_low = if op_is_shift_word { b_val & LS_5_BITS } else { b_val & LS_6_BITS };

        // Store b into in2
        let in2_0: u32 = if op_is_shift {
            ((b_val >> 8) & 0xFFFFFF) as u32
        } else {
            (b_val & 0xFFFFFFFF) as u32
        };
        let in2_1: u32 = ((b_val >> 32) & 0xFFFFFFFF) as u32;

        row.set_all_b(&[in2_0, in2_1]);

        // Calculate the trace output
        let mut t_out: [[u32; 2]; 8] = [[0; 2]; 8];

        // Calculate output based on opcode
        let binary_extension_table_op: BinaryExtensionTableOp;
        match opcode {
            ZiskOp::Sll => {
                binary_extension_table_op = BinaryExtensionTableOp::Sll;
                for j in 0..8 {
                    let bits_to_shift = b_low + 8 * j as u64;
                    let out =
                        if bits_to_shift < 64 { (a_bytes[j] as u64) << bits_to_shift } else { 0 };
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::Srl => {
                binary_extension_table_op = BinaryExtensionTableOp::Srl;
                for j in 0..8 {
                    let out = ((a_bytes[j] as u64) << (8 * j as u64)) >> b_low;
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
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
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
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
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
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
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
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
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::SignExtendB => {
                binary_extension_table_op = BinaryExtensionTableOp::SextB;
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
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::SignExtendH => {
                binary_extension_table_op = BinaryExtensionTableOp::SextH;
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
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::SignExtendW => {
                binary_extension_table_op = BinaryExtensionTableOp::SextW;
                for j in 0..4 {
                    let mut out = (a_bytes[j] as u64) << (8 * j as u64);
                    if j == 3 && ((a_bytes[j] as u64) & SIGN_BYTE) != 0 {
                        out |= SE_MASK_32;
                    }

                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            _ => panic!("BinaryExtensionSM::process_slice() found invalid opcode={}", input.op),
        }

        // Convert the trace output to field elements
        row.set_all_free_in_c(&t_out);

        for (i, a_byte) in a_bytes.iter().enumerate() {
            let row = BinaryExtensionTableSM::calculate_table_row(
                binary_extension_table_op,
                i as u64,
                *a_byte as u64,
                in2_low,
            );
            let offset = row as usize;
            debug_assert!(offset < TABLE_ROW_SPAN);
            if counts[offset] == 0 {
                dirty.push(offset as u32);
            }
            counts[offset] += 1;
        }

        row
    }

    /// Computes the witness for the given set of operations.
    ///
    /// # Arguments
    /// * `operations` - The list of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` representing the computed witness.
    pub fn compute_witness<R: BinaryExtensionTraceRowOps<F>>(
        &self,
        inputs: &[Vec<BinaryInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut binary_e_trace = BinaryExtensionTrace::<R>::new_from_vec(trace_buffer)?;

        let num_rows = binary_e_trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        debug_assert!(total_inputs <= num_rows, "{} <= {}", total_inputs, num_rows);

        tracing::debug!(
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

        // Phase 1 (parallel): each worker accumulates row multiplicities into its thread-local
        // Vec, then returns compact (offset, count) pairs.
        let chunk_results: Vec<Vec<(u32, u64)>> = slices
            .into_par_iter()
            .enumerate()
            .map(|(i, slice)| {
                TL_COUNTS.with(|counts_cell| {
                    TL_DIRTY.with(|dirty_cell| {
                        let mut counts = counts_cell.borrow_mut();
                        let mut dirty = dirty_cell.borrow_mut();

                        slice.iter_mut().enumerate().for_each(|(j, trace_row)| {
                            *trace_row =
                                self.process_slice::<R>(&inputs[i][j], &mut counts, &mut dirty);
                        });

                        let result: Vec<(u32, u64)> =
                            dirty.iter().map(|&o| (o, counts[o as usize])).collect();
                        for &o in dirty.iter() {
                            counts[o as usize] = 0;
                        }
                        dirty.clear();
                        result
                    })
                })
            })
            .collect();

        // Phase 2 (single-threaded): merge all chunk results and call inc_virtual_row once
        // per globally unique row.
        TL_COUNTS.with(|counts_cell| {
            TL_DIRTY.with(|dirty_cell| {
                let mut counts = counts_cell.borrow_mut();
                let mut dirty = dirty_cell.borrow_mut();

                for chunk in &chunk_results {
                    for &(offset, count) in chunk {
                        if counts[offset as usize] == 0 {
                            dirty.push(offset);
                        }
                        counts[offset as usize] += count;
                    }
                }

                for &offset in dirty.iter() {
                    self.std.inc_virtual_row(self.table_id, offset as u64, counts[offset as usize]);
                    counts[offset as usize] = 0;
                }
                dirty.clear();
            });
        });

        // Accumulate range check values, then emit one call per unique value.
        // Sparse in practice (profiling shows 1-2 unique values), but correct for any workload.
        let mut rc_counts: HashMap<u64, u64> = HashMap::new();
        for row in inputs.iter() {
            for input in row.iter() {
                let opcode = ZiskOp::try_from_code(input.op).expect("Invalid ZiskOp opcode");
                if Self::opcode_is_shift(opcode) {
                    let val = (input.b >> 8) & 0xFFFFFF;
                    *rc_counts.entry(val).or_insert(0) += 1;
                }
            }
        }
        for (val, count) in &rc_counts {
            self.std.range_check(self.range_id, *val as i64, *count);
        }

        // Set SEXT_B(0) as the padding row
        let mut padding_row: R = Default::default();
        padding_row.set_op(SEXT_B_OP);

        binary_e_trace.buffer[total_inputs..num_rows]
            .par_iter_mut()
            .for_each(|slot| *slot = padding_row);

        let padding_size = num_rows - total_inputs;
        for i in 0..8 {
            let multiplicity = padding_size as u64;
            let row =
                BinaryExtensionTableSM::calculate_table_row(BinaryExtensionTableOp::SextB, i, 0, 0);
            self.std.inc_virtual_row(self.table_id, row, multiplicity);
        }

        let mut air_values = BinaryExtensionAirValues::<F>::new();
        air_values.padding_size = F::from_usize(padding_size);
        Ok(AirInstance::new_from_trace(
            FromTrace::new(&mut binary_e_trace).with_air_values(&mut air_values),
        ))
    }
}
