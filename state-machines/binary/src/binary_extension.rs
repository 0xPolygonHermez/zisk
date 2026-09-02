//! The `BinaryExtensionSM` module defines the Binary Extension State Machine.
//!
//! This state machine handles binary extension-related operations, computes traces, and manages
//! range checks and multiplicities for table rows based on the operations provided.

use std::sync::Arc;

use crate::{
    extension_requires_full, opcode_is_chain, opcode_is_chain_rev, opcode_is_combine,
    opcode_is_shift, opcode_is_shift_word, BinaryExtensionTableOp, BinaryExtensionTableSM,
    BinaryInput,
};

use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{
    BinaryExtensionAirValues, BinaryExtensionFullAirValues, BinaryExtensionFullTrace,
    BinaryExtensionFullTraceRowOps, BinaryExtensionTrace, BinaryExtensionTraceRowOps,
};

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

/// Abstracts the two extension airs so one witness computation serves both.
///
/// `BinaryExtension` (reduced) and `BinaryExtensionFull` share every column except the ones the
/// full air needs to carry the "dirty" parts of the operands (`free_in_b_bit6`, `free_in_b_bit7`,
/// `b[2]`) and the byte-chain selectors. `Self` is the row type and `T` its trace type.
///
/// See [`crate::extension_requires_full`] for which operations each air can prove.
pub trait BinaryExtensionRow<F: PrimeField64, T>: Default + Copy + Send + Sync {
    /// Sets the columns present in both airs.
    fn set_shared_fields(
        &mut self,
        op: u8,
        free_in_a: &[u8; 8],
        free_in_b: u8,
        free_in_c: &[[u32; 2]; 8],
        op_is_shift: bool,
        op_is_combine: bool,
    );

    /// Sets the columns only the full air owns. A no-op on the reduced air, whose PIL pins these
    /// to the values its restricted operand shapes imply.
    fn set_full_only_fields(
        &mut self,
        free_in_b_bit6: bool,
        free_in_b_bit7: bool,
        op_is_chain: bool,
        op_is_chain_rev: bool,
        b: &[u32; 2],
    );

    /// `true` for the air that owns the full-only columns.
    fn is_full() -> bool;

    fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<T>;
    fn trace_num_rows(trace: &T) -> usize;
    fn trace_buffer_mut(trace: &mut T) -> &mut [Self];

    /// Fills the padding rows and wraps the trace into an `AirInstance`.
    fn into_air_instance(trace: &mut T, padding_row: Self, total_inputs: usize) -> AirInstance<F>;
}

impl<F: PrimeField64, R: BinaryExtensionTraceRowOps<F>>
    BinaryExtensionRow<F, BinaryExtensionTrace<R>> for R
{
    #[inline(always)]
    fn set_shared_fields(
        &mut self,
        op: u8,
        free_in_a: &[u8; 8],
        free_in_b: u8,
        free_in_c: &[[u32; 2]; 8],
        op_is_shift: bool,
        op_is_combine: bool,
    ) {
        self.set_op(op);
        self.set_all_free_in_a(free_in_a);
        self.set_free_in_b(free_in_b);
        self.set_all_free_in_c(free_in_c);
        self.set_op_is_shift(op_is_shift);
        self.set_op_is_combine(op_is_combine);
    }

    #[inline(always)]
    fn set_full_only_fields(&mut self, _: bool, _: bool, _: bool, _: bool, _: &[u32; 2]) {}

    #[inline(always)]
    fn is_full() -> bool {
        false
    }

    fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<BinaryExtensionTrace<R>> {
        BinaryExtensionTrace::<R>::new_from_vec(trace_buffer)
    }

    fn trace_num_rows(trace: &BinaryExtensionTrace<R>) -> usize {
        trace.num_rows()
    }

    fn trace_buffer_mut(trace: &mut BinaryExtensionTrace<R>) -> &mut [Self] {
        &mut trace.buffer
    }

    fn into_air_instance(
        trace: &mut BinaryExtensionTrace<R>,
        padding_row: Self,
        total_inputs: usize,
    ) -> AirInstance<F> {
        let num_rows = trace.num_rows();
        trace.buffer[total_inputs..num_rows].par_iter_mut().for_each(|slot| *slot = padding_row);

        let mut air_values = BinaryExtensionAirValues::<F>::new();
        air_values.padding_size = F::from_usize(num_rows - total_inputs);
        AirInstance::new_from_trace(FromTrace::new(trace).with_air_values(&mut air_values))
    }
}

impl<F: PrimeField64, R: BinaryExtensionFullTraceRowOps<F>>
    BinaryExtensionRow<F, BinaryExtensionFullTrace<R>> for R
{
    #[inline(always)]
    fn set_shared_fields(
        &mut self,
        op: u8,
        free_in_a: &[u8; 8],
        free_in_b: u8,
        free_in_c: &[[u32; 2]; 8],
        op_is_shift: bool,
        op_is_combine: bool,
    ) {
        self.set_op(op);
        self.set_all_free_in_a(free_in_a);
        self.set_free_in_b(free_in_b);
        self.set_all_free_in_c(free_in_c);
        self.set_op_is_shift(op_is_shift);
        self.set_op_is_combine(op_is_combine);
    }

    #[inline(always)]
    fn set_full_only_fields(
        &mut self,
        free_in_b_bit6: bool,
        free_in_b_bit7: bool,
        op_is_chain: bool,
        op_is_chain_rev: bool,
        b: &[u32; 2],
    ) {
        self.set_free_in_b_bit6(free_in_b_bit6);
        self.set_free_in_b_bit7(free_in_b_bit7);
        self.set_op_is_chain(op_is_chain);
        self.set_op_is_chain_rev(op_is_chain_rev);
        self.set_all_b(b);
    }

    #[inline(always)]
    fn is_full() -> bool {
        true
    }

    fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<BinaryExtensionFullTrace<R>> {
        BinaryExtensionFullTrace::<R>::new_from_vec(trace_buffer)
    }

    fn trace_num_rows(trace: &BinaryExtensionFullTrace<R>) -> usize {
        trace.num_rows()
    }

    fn trace_buffer_mut(trace: &mut BinaryExtensionFullTrace<R>) -> &mut [Self] {
        &mut trace.buffer
    }

    fn into_air_instance(
        trace: &mut BinaryExtensionFullTrace<R>,
        padding_row: Self,
        total_inputs: usize,
    ) -> AirInstance<F> {
        let num_rows = trace.num_rows();
        trace.buffer[total_inputs..num_rows].par_iter_mut().for_each(|slot| *slot = padding_row);

        let mut air_values = BinaryExtensionFullAirValues::<F>::new();
        air_values.padding_size = F::from_usize(num_rows - total_inputs);
        AirInstance::new_from_trace(FromTrace::new(trace).with_air_values(&mut air_values))
    }
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

    /// Processes a single operation and generates the corresponding trace row.
    ///
    /// # Arguments
    /// * `operation` - The operation to process.
    /// * `multiplicity` - A mutable reference to the multiplicity table to update.
    /// * `range_check` - A mutable reference to the range check table to update.
    ///
    /// # Returns
    /// A row of the air selected by `R` (reduced or full) representing the processed trace.
    ///
    /// # Panics
    /// In debug mode, panics if `R` is the reduced air but the operation needs the full one; the
    /// counter, planner and collectors are expected to keep those apart.
    pub fn process_slice<T, R: BinaryExtensionRow<F, T>>(&self, input: &BinaryInput) -> R {
        // Get a ZiskOp from the code
        let opcode = ZiskOp::try_from_code(input.op).expect("Invalid ZiskOp opcode");

        debug_assert!(
            R::is_full() || !extension_requires_full(input.op, input.a, input.b),
            "BinaryExtensionSM: op={:#04x} a={:#x} b={:#x} needs BinaryExtensionFull",
            input.op,
            input.a,
            input.b
        );

        // Set if the opcode is a shift operation
        let op_is_shift = opcode_is_shift(opcode);

        // Set if the opcode is a byte-chain operation (forward: ctz family, reverse: clz family)
        let op_is_chain = opcode_is_chain(opcode);
        let op_is_chain_rev = opcode_is_chain_rev(opcode);

        // Set if the opcode is a pack (combine) operation
        let op_is_combine = opcode_is_combine(opcode);

        // Set if the opcode is a shift word operation
        let op_is_shift_word = opcode_is_shift_word(opcode);

        // Select the value that is byte-decomposed into free_in_a:
        //  - shift:   the value being shifted (input.a)
        //  - combine: the two low halves interleaved, input.a[31:0] | input.b[31:0] << 32
        //  - other:   the single operand (input.b)
        let a_val = if op_is_shift {
            input.a
        } else if op_is_combine {
            (input.a & 0xFFFFFFFF) | ((input.b & 0xFFFFFFFF) << 32)
        } else {
            input.b
        };
        let b_val = if op_is_shift { input.b } else { input.a };

        // Split a in bytes and store them in in1
        let a_bytes: [u8; 8] = a_val.to_le_bytes();

        // Store b low part into in2_low (only shifts use it; 0 otherwise). The table lookup only
        // consumes the low 6 bits (free_in_b, since the shift amount is masked with LS_6_BITS /
        // LS_5_BITS); the two remaining bits are carried separately in free_in_b_bit6 /
        // free_in_b_bit7 so the full shift-amount low byte can be rebuilt on the operation bus.
        // The reduced air has no bit6/bit7 columns because it only admits amounts below 64.
        let in2_low: u64 = if op_is_shift { b_val & 0xFF } else { 0 };

        // Store b lower bits when shifting, depending on operation size
        let b_low = if op_is_shift_word { b_val & LS_5_BITS } else { b_val & LS_6_BITS };

        // Store the b[] witness columns (full air only; the reduced one pins them all to 0):
        //  - shift:   the shift amount (bits 8..63 of input.b; low byte is in free_in_b)
        //  - combine: the two high halves, b[0] = input.a[63:32], b[1] = input.b[63:32]
        //  - other:   the operand high/low (0 for single-source ops)
        let (in2_0, in2_1): (u32, u32) = if op_is_shift {
            (((b_val >> 8) & 0xFFFFFF) as u32, ((b_val >> 32) & 0xFFFFFFFF) as u32)
        } else if op_is_combine {
            ((input.a >> 32) as u32, (input.b >> 32) as u32)
        } else {
            ((b_val & 0xFFFFFFFF) as u32, ((b_val >> 32) & 0xFFFFFFFF) as u32)
        };

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
            ZiskOp::SllUW => {
                // slli.uw: rd = zext32(a) << (b & 63). Like Sll, but only the low 4 bytes of a
                // take part (that is the zero extension) and the result keeps its full 64 bits,
                // unlike SllW which truncates to 32 and sign-extends.
                //
                // The instruction sets m32, so the bus already zeroed the high half of a and those
                // bytes are zero; skipping them makes the zero extension explicit.
                binary_extension_table_op = BinaryExtensionTableOp::SllUw;
                for j in 0..4 {
                    let bits_to_shift = b_low + 8 * j as u64;
                    let out =
                        if bits_to_shift < 64 { (a_bytes[j] as u64) << bits_to_shift } else { 0 };
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
            ZiskOp::Rev8 => {
                // Byte-reverse the 64-bit input: byte j moves to position 7 - j.
                // Single input (op_is_shift = 0), so `a_bytes` holds the operand.
                binary_extension_table_op = BinaryExtensionTableOp::Rev8;
                for j in 0..8 {
                    let out = (a_bytes[j] as u64) << (8 * (7 - j) as u64);
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::OrcB => {
                // OR-combine bits within each byte, in place: output byte j is 0xFF
                // if input byte j has any bit set, else 0x00. Single input.
                binary_extension_table_op = BinaryExtensionTableOp::OrcB;
                for j in 0..8 {
                    let out = if a_bytes[j] != 0 { 0xFFu64 << (8 * j as u64) } else { 0 };
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::Rol => {
                // Rotate left the full 64-bit value by `b_low` (mod 64), per byte.
                binary_extension_table_op = BinaryExtensionTableOp::Rol;
                for j in 0..8 {
                    let a_pos = (a_bytes[j] as u64) << (8 * j as u64);
                    let out = a_pos.rotate_left(b_low as u32);
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::Ror => {
                // Rotate right the full 64-bit value by `b_low` (mod 64), per byte.
                binary_extension_table_op = BinaryExtensionTableOp::Ror;
                for j in 0..8 {
                    let a_pos = (a_bytes[j] as u64) << (8 * j as u64);
                    let out = a_pos.rotate_right(b_low as u32);
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::RolW => {
                // Rotate left the low 32 bits by `b_low` (mod 32), sign-extended.
                binary_extension_table_op = BinaryExtensionTableOp::RolW;
                for j in 0..8 {
                    let out = if j >= 4 {
                        0u64
                    } else {
                        let lo = ((a_bytes[j] as u64) << (8 * j as u64)) as u32;
                        let r = lo.rotate_left(b_low as u32) as u64;
                        if r & SIGN_32_BIT != 0 {
                            r | SE_MASK_32
                        } else {
                            r
                        }
                    };
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::RorW => {
                // Rotate right the low 32 bits by `b_low` (mod 32), sign-extended.
                binary_extension_table_op = BinaryExtensionTableOp::RorW;
                for j in 0..8 {
                    let out = if j >= 4 {
                        0u64
                    } else {
                        let lo = ((a_bytes[j] as u64) << (8 * j as u64)) as u32;
                        let r = lo.rotate_right(b_low as u32) as u64;
                        if r & SIGN_32_BIT != 0 {
                            r | SE_MASK_32
                        } else {
                            r
                        }
                    };
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::Cpop => {
                // Population count: each byte contributes its own set-bit count (0..8),
                // position-independent; the 8 contributions sum to the 64-bit popcount.
                binary_extension_table_op = BinaryExtensionTableOp::Cpop;
                for j in 0..8 {
                    t_out[j][0] = a_bytes[j].count_ones();
                }
            }
            ZiskOp::CpopW => {
                // Population count of the low 32 bits: only the low 4 bytes contribute.
                binary_extension_table_op = BinaryExtensionTableOp::CpopW;
                for j in 0..4 {
                    t_out[j][0] = a_bytes[j].count_ones();
                }
            }
            ZiskOp::Ctz => {
                // Count trailing zeros, byte-chained. `free_in_c[j][0]` = per-byte increment
                // (summed = ctz), `free_in_c[j][1]` = acc_in entering byte j. A byte is still in
                // the trailing-zero run iff acc_in == 8*j, in which case it adds its own trailing
                // zeros (8 for a zero byte, else 0..7); otherwise it adds 0.
                binary_extension_table_op = BinaryExtensionTableOp::Ctz;
                let mut acc: u64 = 0;
                for j in 0..8 {
                    let acc_in = acc;
                    let incr =
                        if acc_in == 8 * j as u64 { a_bytes[j].trailing_zeros() as u64 } else { 0 };
                    t_out[j][0] = incr as u32;
                    t_out[j][1] = acc_in as u32;
                    acc = acc_in + incr;
                }
            }
            ZiskOp::CtzW => {
                // Count trailing zeros of the low 32 bits. Same chain as Ctz but only the low 4
                // bytes participate; bytes at offset >= 4 add nothing.
                binary_extension_table_op = BinaryExtensionTableOp::CtzW;
                let mut acc: u64 = 0;
                for j in 0..8 {
                    let acc_in = acc;
                    let incr = if j < 4 && acc_in == 8 * j as u64 {
                        a_bytes[j].trailing_zeros() as u64
                    } else {
                        0
                    };
                    t_out[j][0] = incr as u32;
                    t_out[j][1] = acc_in as u32;
                    acc = acc_in + incr;
                }
            }
            ZiskOp::Clz => {
                // Count leading zeros, reverse byte-chain (scan MSB -> LSB). Mirror of Ctz: a
                // byte is still in the leading-zero run iff acc_in == 8*(7-j); it then adds its
                // own leading zeros (8 for a zero byte, else 0..7). Increments telescope to clz.
                binary_extension_table_op = BinaryExtensionTableOp::Clz;
                let mut acc: u64 = 0;
                for j in (0..8).rev() {
                    let acc_in = acc;
                    let incr = if acc_in == 8 * (7 - j) as u64 {
                        a_bytes[j].leading_zeros() as u64
                    } else {
                        0
                    };
                    t_out[j][0] = incr as u32;
                    t_out[j][1] = acc_in as u32;
                    acc = acc_in + incr;
                }
            }
            ZiskOp::ClzW => {
                // Count leading zeros of the low 32 bits. Same reverse chain as Clz but only the
                // low 4 bytes participate; the top of the word is byte 3.
                binary_extension_table_op = BinaryExtensionTableOp::ClzW;
                let mut acc: u64 = 0;
                for j in (0..4).rev() {
                    let acc_in = acc;
                    // `leading_zeros` on the u8 byte gives 0..8 (its position within the 32-bit
                    // word is handled by the 8*(3-j) threshold, mirroring Ctz_w).
                    let incr = if acc_in == 8 * (3 - j) as u64 {
                        a_bytes[j].leading_zeros() as u64
                    } else {
                        0
                    };
                    t_out[j][0] = incr as u32;
                    t_out[j][1] = acc_in as u32;
                    acc = acc_in + incr;
                }
            }
            ZiskOp::Pack => {
                // rd = rs1[31:0] | (rs2[31:0] << 32). free_in_a already holds rs1[31:0] in its
                // low 4 bytes and rs2[31:0] in its high 4 bytes, so each byte lands in place.
                binary_extension_table_op = BinaryExtensionTableOp::Pack;
                for j in 0..8 {
                    let out = (a_bytes[j] as u64) << (8 * j as u64);
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::PackH => {
                // rd = rs1[7:0] | (rs2[7:0] << 8): byte 0 -> result byte 0, byte 4 -> result byte 1.
                binary_extension_table_op = BinaryExtensionTableOp::PackH;
                t_out[0][0] = a_bytes[0] as u32;
                t_out[4][0] = (a_bytes[4] as u32) << 8;
            }
            ZiskOp::PackW => {
                // rd = sext32(rs1[15:0] | (rs2[15:0] << 16)): bytes 0,1 -> result bytes 0,1;
                // bytes 4,5 -> result bytes 2,3; sign-extend from bit 7 of byte 5 (bit 31).
                binary_extension_table_op = BinaryExtensionTableOp::PackW;
                t_out[0][0] = a_bytes[0] as u32;
                t_out[1][0] = (a_bytes[1] as u32) << 8;
                t_out[4][0] = (a_bytes[4] as u32) << 16;
                t_out[5][0] = (a_bytes[5] as u32) << 24;
                if a_bytes[5] & (SIGN_BYTE as u8) != 0 {
                    t_out[5][1] = MASK_32 as u32;
                }
            }
            ZiskOp::Bclr => {
                // rd = a & ~(1 << pos). Only the byte holding `pos` is affected; the mask is a
                // no-op on the others, so it can be applied uniformly (branch-free).
                binary_extension_table_op = BinaryExtensionTableOp::Bclr;
                for j in 0..8 {
                    let a_pos = (a_bytes[j] as u64) << (8 * j as u64);
                    let out = a_pos & !(1u64 << b_low);
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::Bext => {
                // rd = (a >> pos) & 1: the extracted bit lands at result bit 0.
                binary_extension_table_op = BinaryExtensionTableOp::Bext;
                let target = (b_low >> 3) as usize;
                let bit = b_low & 0x07;
                t_out[target][0] = (((a_bytes[target] as u64) >> bit) & 1) as u32;
            }
            ZiskOp::Binv => {
                // rd = a ^ (1 << pos): only the byte holding `pos` flips it.
                binary_extension_table_op = BinaryExtensionTableOp::Binv;
                let target = (b_low >> 3) as usize;
                for j in 0..8 {
                    let a_pos = (a_bytes[j] as u64) << (8 * j as u64);
                    let out = if j == target { a_pos ^ (1u64 << b_low) } else { a_pos };
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            ZiskOp::Bset => {
                // rd = a | (1 << pos): only the byte holding `pos` sets it.
                binary_extension_table_op = BinaryExtensionTableOp::Bset;
                let target = (b_low >> 3) as usize;
                for j in 0..8 {
                    let a_pos = (a_bytes[j] as u64) << (8 * j as u64);
                    let out = if j == target { a_pos | (1u64 << b_low) } else { a_pos };
                    t_out[j][0] = (out & 0xffffffff) as u32;
                    t_out[j][1] = ((out >> 32) & 0xffffffff) as u32;
                }
            }
            _ => panic!("BinaryExtensionSM::process_slice() found invalid opcode={}", input.op),
        }

        for (i, a_byte) in a_bytes.iter().enumerate() {
            // For chain ops (forward or reverse) the fourth argument is acc_in (carried in
            // free_in_c[j][1]), which selects the row within the block; for the rest it is the
            // shared B value, restricted to its low 6 bits (the only part the table enumerates).
            let table_b = if op_is_chain || op_is_chain_rev {
                t_out[i][1] as u64
            } else {
                in2_low & LS_6_BITS
            };
            let table_row = BinaryExtensionTableSM::calculate_table_row(
                binary_extension_table_op,
                i as u64,
                *a_byte as u64,
                table_b,
            );
            self.std.inc_virtual_row_one(self.table_id, table_row);
        }

        // Build the trace row: the shared columns first, then the ones only the full air owns
        // (a no-op on the reduced air).
        let mut row: R = Default::default();
        row.set_shared_fields(
            input.op,
            &a_bytes,
            (in2_low & LS_6_BITS) as u8,
            &t_out,
            op_is_shift,
            op_is_combine,
        );
        row.set_full_only_fields(
            (in2_low >> 6) & 1 != 0,
            (in2_low >> 7) & 1 != 0,
            op_is_chain,
            op_is_chain_rev,
            &[in2_0, in2_1],
        );

        row
    }

    /// Computes the witness for the given set of operations.
    ///
    /// # Arguments
    /// * `operations` - The list of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` representing the computed witness.
    pub fn compute_witness<T, R: BinaryExtensionRow<F, T>>(
        &self,
        inputs: &[Vec<BinaryInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut binary_e_trace = R::new_trace(trace_buffer)?;

        let num_rows = R::trace_num_rows(&binary_e_trace);

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        debug_assert!(total_inputs <= num_rows, "{} <= {}", total_inputs, num_rows);

        tracing::debug!(
            "··· Creating Binary Extension{} instance [{} / {} rows filled {:.2}%]",
            if R::is_full() { "Full" } else { "" },
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        // Split the trace buffer into slices matching each inner vector’s length.
        let sizes: Vec<usize> = inputs.iter().map(|v| v.len()).collect();
        let mut slices = Vec::with_capacity(inputs.len());
        let mut rest = &mut R::trace_buffer_mut(&mut binary_e_trace)[..];
        for size in sizes {
            let (head, tail) = rest.split_at_mut(size);
            slices.push(head);
            rest = tail;
        }

        // Process each slice in parallel, and use the corresponding inner input from `inputs`.
        slices.into_par_iter().enumerate().for_each(|(i, slice)| {
            slice.iter_mut().enumerate().for_each(|(j, trace_row)| {
                *trace_row = self.process_slice::<T, R>(&inputs[i][j]);
            });
        });

        // Range-check the high part of the shift amount carried in b[0]. Only the full air has
        // that column (and the constraint); the reduced air keeps the whole amount in free_in_b,
        // so it must not contribute to the range table.
        if R::is_full() {
            for row in inputs.iter() {
                for input in row.iter() {
                    let opcode = ZiskOp::try_from_code(input.op).expect("Invalid ZiskOp opcode");
                    if opcode_is_shift(opcode) {
                        let row = (input.b >> 8) & 0xFFFFFF;
                        self.std.range_check_one(self.range_id, row);
                    }
                }
            }
        }

        // Set SEXT_B(0) as the padding row
        let mut padding_row: R = Default::default();
        padding_row.set_shared_fields(
            ZiskOp::SignExtendB.code(),
            &[0; 8],
            0,
            &[[0; 2]; 8],
            false,
            false,
        );

        let padding_size = num_rows - total_inputs;
        for i in 0..8 {
            let multiplicity = padding_size as u64;
            let row =
                BinaryExtensionTableSM::calculate_table_row(BinaryExtensionTableOp::SextB, i, 0, 0);
            self.std.inc_virtual_row(self.table_id, row, multiplicity);
        }

        Ok(R::into_air_instance(&mut binary_e_trace, padding_row, total_inputs))
    }
}
