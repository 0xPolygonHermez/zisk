//! The `BinaryBasicSM` module implements the logic for the Binary Basic State Machine.
//!
//! This state machine processes binary-related operations.

use std::sync::Arc;

use crate::{
    binary_constants::*, BinaryBasicTableOp, BinaryBasicTableSM, BinaryInput, BinaryLanes,
};
use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use std::cmp::Ordering as CmpOrdering;
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{
    BinaryAirValues, BinaryHugeAirValues, BinaryHugeTrace, BinaryHugeTraceRowOps,
    BinaryLargeAirValues, BinaryLargeTrace, BinaryLargeTraceRowOps, BinaryTrace, BinaryTraceRowOps,
};

const MASK_U64: u64 = 0xFFFF_FFFF_FFFF_FFFF;
const SIGN_BYTE: u8 = 0x80;

/// Ties a basic row type to the trace of the air it fills and to that air's packing width.
///
/// The three airs commit the same columns at different widths (`b_op[1]` vs `b_op[2]` vs `b_op[4]`),
/// so they cannot share a row type: each has its own, and `Self::LANES_X_ROW` is what tells the
/// shared fill logic how many slots to write.
///
/// The setters carry the same names as the generated ones on purpose: `process_slice` is bound by
/// this trait alone, so there is nothing to be ambiguous with, and the body reads the same as it did
/// before lanes existed — with the slot it writes to as its first argument.
pub trait BinaryBasicRow<F: PrimeField64, T>: Default + Copy + Send + Sync {
    /// Operations this air packs into one row.
    const LANES_X_ROW: usize;

    fn set_b_op(&mut self, lane: usize, value: u8);
    fn set_mode32(&mut self, lane: usize, value: bool);
    fn set_result_is_a(&mut self, lane: usize, value: bool);
    fn set_use_first_byte(&mut self, lane: usize, value: bool);
    fn set_c_is_signed(&mut self, lane: usize, value: bool);
    fn set_all_free_in_a(&mut self, lane: usize, values: &[u8; 8]);
    fn set_all_free_in_b(&mut self, lane: usize, values: &[u8; 8]);
    fn set_all_free_in_c(&mut self, lane: usize, values: &[u8; 8]);
    fn set_all_carry(&mut self, lane: usize, values: &[u8; 8]);

    fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<T>;
    fn trace_num_rows(trace: &T) -> usize;
    fn trace_buffer_mut(trace: &mut T) -> &mut [Self];

    /// Fills the padding rows and wraps the trace into an `AirInstance`.
    ///
    /// `padding_size` is counted in *slots*, not rows: the bus sees one operation per slot, so what
    /// has to be cancelled is the number of empty slots.
    fn into_air_instance(
        trace: &mut T,
        padding_row: Self,
        rows_used: usize,
        padding_size: usize,
    ) -> AirInstance<F>;
}

/// Emits the row-to-trace binding for one basic air. The bodies are identical; only the row-ops
/// trait, the trace alias, its air values and the packing width change.
macro_rules! impl_binary_basic_row {
    ($row_ops:ident, $trace:ident, $air_values:ident, $lanes:expr) => {
        impl<F: PrimeField64, R: $row_ops<F>> BinaryBasicRow<F, $trace<R>> for R {
            const LANES_X_ROW: usize = $lanes;

            #[inline(always)]
            fn set_b_op(&mut self, lane: usize, value: u8) {
                $row_ops::set_b_op(self, lane, value);
            }
            #[inline(always)]
            fn set_mode32(&mut self, lane: usize, value: bool) {
                $row_ops::set_mode32(self, lane, value);
            }
            #[inline(always)]
            fn set_result_is_a(&mut self, lane: usize, value: bool) {
                $row_ops::set_result_is_a(self, lane, value);
            }
            #[inline(always)]
            fn set_use_first_byte(&mut self, lane: usize, value: bool) {
                $row_ops::set_use_first_byte(self, lane, value);
            }
            #[inline(always)]
            fn set_c_is_signed(&mut self, lane: usize, value: bool) {
                $row_ops::set_c_is_signed(self, lane, value);
            }
            #[inline(always)]
            fn set_all_free_in_a(&mut self, lane: usize, values: &[u8; 8]) {
                for (j, &v) in values.iter().enumerate() {
                    $row_ops::set_free_in_a(self, lane, j, v);
                }
            }
            #[inline(always)]
            fn set_all_free_in_b(&mut self, lane: usize, values: &[u8; 8]) {
                for (j, &v) in values.iter().enumerate() {
                    $row_ops::set_free_in_b(self, lane, j, v);
                }
            }
            #[inline(always)]
            fn set_all_free_in_c(&mut self, lane: usize, values: &[u8; 8]) {
                for (j, &v) in values.iter().enumerate() {
                    $row_ops::set_free_in_c(self, lane, j, v);
                }
            }
            #[inline(always)]
            fn set_all_carry(&mut self, lane: usize, values: &[u8; 8]) {
                for (j, &v) in values.iter().enumerate() {
                    $row_ops::set_carry(self, lane, j, v);
                }
            }

            fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<$trace<R>> {
                $trace::<R>::new_from_vec(trace_buffer)
            }

            fn trace_num_rows(trace: &$trace<R>) -> usize {
                trace.num_rows()
            }

            fn trace_buffer_mut(trace: &mut $trace<R>) -> &mut [Self] {
                &mut trace.buffer
            }

            fn into_air_instance(
                trace: &mut $trace<R>,
                padding_row: Self,
                rows_used: usize,
                padding_size: usize,
            ) -> AirInstance<F> {
                let num_rows = trace.num_rows();
                trace.buffer[rows_used..num_rows]
                    .par_iter_mut()
                    .for_each(|slot| *slot = padding_row);

                let mut air_values = $air_values::<F>::new();
                air_values.padding_size = F::from_usize(padding_size);
                AirInstance::new_from_trace(FromTrace::new(trace).with_air_values(&mut air_values))
            }
        }
    };
}

impl_binary_basic_row!(BinaryTraceRowOps, BinaryTrace, BinaryAirValues, 1);
impl_binary_basic_row!(BinaryLargeTraceRowOps, BinaryLargeTrace, BinaryLargeAirValues, 2);
impl_binary_basic_row!(BinaryHugeTraceRowOps, BinaryHugeTrace, BinaryHugeAirValues, 4);

/// The `BinaryBasicSM` struct encapsulates the logic of the Binary Basic State Machine.
pub struct BinaryBasicSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,

    /// The table ID for the Binary Basic State Machine
    table_id: usize,
}

impl<F: PrimeField64> BinaryBasicSM<F> {
    /// Creates a new Binary Basic State Machine instance.
    ///
    /// # Arguments
    /// * `std` - An `Arc`-wrapped reference to the PIL2 standard library.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `BinaryBasicSM`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Get the table ID
        let table_id =
            std.get_virtual_table_id(BinaryBasicTableSM::TABLE_ID).expect("Failed to get range ID");

        Arc::new(Self { std, table_id })
    }

    /// Determines if an opcode corresponds to a 32-bit operation.
    ///
    /// # Arguments
    /// * `opcode` - The opcode to evaluate.
    ///
    /// # Returns
    /// `true` if the opcode is 32-bit; `false` otherwise.
    fn opcode_is_32_bits(opcode: u8) -> bool {
        const OPCODES_32_BITS: [u8; 11] = [
            MINUW_OP, MINW_OP, MAXUW_OP, MAXW_OP, LTUW_OP, LTW_OP, EQW_OP, ADDW_OP, SUBW_OP,
            LEUW_OP, LEW_OP,
        ];

        OPCODES_32_BITS.contains(&opcode)
    }

    fn opcode_is_comparator(opcode: u8) -> bool {
        matches!(
            opcode,
            LT_ABS_NP_OP
                | LT_ABS_PN_OP
                | LTU_OP
                | LTUW_OP
                | LT_OP
                | LTW_OP
                | GT_OP
                | EQ_OP
                | EQW_OP
                | LEU_OP
                | LEUW_OP
                | LE_OP
                | LEW_OP
        )
    }

    /// Helper function for LT_ABS_NP operation execution.
    fn lt_abs_np_execute(a: u64, b: u64) -> (u64, bool) {
        let a_pos = (a ^ MASK_U64).wrapping_add(1);
        if a_pos < b {
            (1, true)
        } else {
            (0, false)
        }
    }

    /// Helper function for LT_ABS_PN operation execution.
    fn lt_abs_pn_execute(a: u64, b: u64) -> (u64, bool) {
        let b_pos = (b ^ MASK_U64).wrapping_add(1);
        if a < b_pos {
            (1, true)
        } else {
            (0, false)
        }
    }

    /// Helper function for GT operation execution.
    fn gt_execute(a: u64, b: u64) -> (u64, bool) {
        if (a as i64) > (b as i64) {
            (1, true)
        } else {
            (0, false)
        }
    }

    /// Executes a binary operation based on the opcode and inputs `a` and `b`.
    ///
    /// # Arguments
    /// * `opcode` - The operation code to execute.
    /// * `a` - The first operand.
    /// * `b` - The second operand.
    ///
    /// # Returns
    /// A tuple containing:
    /// * The result of the operation (`u64`).
    /// * A boolean indicating whether the operation generated a carry/flag.
    fn execute(opcode: u8, a: u64, b: u64) -> (u64, bool) {
        let is_zisk_op = ZiskOp::try_from_code(opcode).is_ok();
        if is_zisk_op {
            ZiskOp::execute(opcode, a, b)
        } else {
            match opcode {
                LT_ABS_NP_OP => Self::lt_abs_np_execute(a, b),
                LT_ABS_PN_OP => Self::lt_abs_pn_execute(a, b),
                GT_OP => Self::gt_execute(a, b),
                _ => panic!("BinaryBasicSM::execute() got invalid opcode={opcode:?}"),
            }
        }
    }

    /// Processes a slice of operation data, generating a trace row and updating multiplicities.
    ///
    /// # Arguments
    /// * `operation` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    ///
    /// Fills one slot of a row from one operation, updating the table multiplicities it touches.
    #[inline(always)]
    pub fn process_slice<T, R: BinaryBasicRow<F, T>>(
        &self,
        row: &mut R,
        lane: usize,
        input: &BinaryInput,
    ) {
        // Execute the opcode
        let opcode = input.op;
        let a = input.a;
        let b = input.b;

        let (c, _) = Self::execute(input.op, input.a, input.b);

        // Set mode32
        let mode32 = Self::opcode_is_32_bits(opcode);
        row.set_mode32(lane, mode32);

        // Set c_filtered
        let c_filtered = if mode32 { c & 0xFF_FF_FF_FF } else { c };

        // Split a, b, c into bytes
        let a_bytes: [u8; 8] = a.to_le_bytes();
        let b_bytes: [u8; 8] = b.to_le_bytes();
        let c_bytes: [u8; 8] = c.to_le_bytes();

        // Store bytes in free_in_a, free_in_b, free_in_c
        row.set_all_free_in_a(lane, &a_bytes);
        row.set_all_free_in_b(lane, &b_bytes);
        if Self::opcode_is_comparator(opcode) {
            row.set_all_free_in_c(lane, &[0u8; 8]);
        } else {
            row.set_all_free_in_c(lane, &c_bytes);
        }

        // Set use last carry and carry[], based on operation
        let mut cout: u64;
        let mut cin: u64 = 0;
        let pfirst: [u64; 8] = [1, 0, 0, 0, 0, 0, 0, 0];
        let plast: [u64; 8] =
            if mode32 { [0, 0, 0, 1, 0, 0, 0, 0] } else { [0, 0, 0, 0, 0, 0, 0, 1] };

        // Calculate the byte that sets the carry
        let carry_byte = if mode32 { 3 } else { 7 };

        // Determine if c is signed
        let c_is_signed = if c_bytes[carry_byte] & SIGN_BYTE != 0 { 1 } else { 0 };

        let binary_basic_table_op: BinaryBasicTableOp;
        match opcode {
            MINU_OP | MINUW_OP | MIN_OP | MINW_OP => {
                // Set first byte
                row.set_use_first_byte(lane, false);

                // Set result_is_a
                let result_is_a: u64 = if (a == b) || (b == c_filtered) { 0 } else { 1 };
                row.set_result_is_a(lane, result_is_a != 0);

                // Set c_is_signed
                row.set_c_is_signed(lane, c_is_signed != 0);

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == MINU_OP) || (opcode == MINUW_OP) {
                    BinaryBasicTableOp::Minu
                } else {
                    BinaryBasicTableOp::Min
                };

                let mut carry = [0u8; 8];
                for i in 0..8 {
                    // Calculate carry
                    match a_bytes[i].cmp(&b_bytes[i]) {
                        CmpOrdering::Greater => {
                            cout = 0;
                        }
                        CmpOrdering::Less => {
                            cout = 1;
                        }
                        CmpOrdering::Equal => {
                            cout = cin;
                        }
                    }

                    // In the last byte, set cout to 0
                    if plast[i] == 1 {
                        cout = 0;
                    }

                    carry[i] = cout as u8;
                    let previous_cin = cin;
                    cin = cout;

                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = cout + 16 * result_is_a + 64 * plast[i] * c_is_signed;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            if c_is_signed == 1 {
                                BinaryBasicTableOp::SextFF
                            } else {
                                BinaryBasicTableOp::Sext00
                            }
                        } else {
                            binary_basic_table_op
                        },
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
                row.set_all_carry(lane, &carry);
            }
            MAXU_OP | MAXUW_OP | MAX_OP | MAXW_OP => {
                // Set first byte
                row.set_use_first_byte(lane, false);

                // Set result_is_a
                let result_is_a: u64 = if (a == b) || (b == c_filtered) { 0 } else { 1 };
                row.set_result_is_a(lane, result_is_a != 0);

                // Set c_is_signed
                row.set_c_is_signed(lane, c_is_signed != 0);

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == MAXU_OP) || (opcode == MAXUW_OP) {
                    BinaryBasicTableOp::Maxu
                } else {
                    BinaryBasicTableOp::Max
                };

                let mut carry = [0u8; 8];
                for i in 0..8 {
                    // Calculate carry
                    match a_bytes[i].cmp(&b_bytes[i]) {
                        CmpOrdering::Greater => {
                            cout = 1;
                        }
                        CmpOrdering::Less => {
                            cout = 0;
                        }
                        CmpOrdering::Equal => {
                            cout = cin;
                        }
                    }

                    // In the last byte, set cout to 0
                    if plast[i] == 1 {
                        cout = 0;
                    }

                    carry[i] = cout as u8;

                    let previous_cin = cin;
                    cin = cout;

                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = cout + 16 * result_is_a + 64 * plast[i] * c_is_signed;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            if c_is_signed == 1 {
                                BinaryBasicTableOp::SextFF
                            } else {
                                BinaryBasicTableOp::Sext00
                            }
                        } else {
                            binary_basic_table_op
                        },
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
                row.set_all_carry(lane, &carry);
            }
            LT_ABS_NP_OP => {
                // Set first byte
                row.set_use_first_byte(lane, true);

                // Set result_is_a
                row.set_result_is_a(lane, false);

                // Set c_is_signed
                row.set_c_is_signed(lane, false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::LtAbsNP;

                let mut carry = [0u8; 8];
                for i in 0..8 {
                    // Decode the two carries packed into cin = 0bYX:
                    //   clt  (X) = carry of the LT comparison between |a| and b
                    //   cneg (Y) = carry of the negation (a ^ 0xFF) + cneg
                    let clt = if pfirst[i] == 1 { 0 } else { cin & 0x01 };
                    let cneg = if pfirst[i] == 1 { 1 } else { (cin & 0x02) >> 1 };

                    // |a| byte = (a ^ 0xFF) + cneg. Compare its low byte (abs_a)
                    // against b, then carry the negation overflow (_a >> 8) in bit 1
                    let _a = (a_bytes[i] ^ 0xFF) as u64 + cneg;
                    let abs_a = _a & 0xFF;
                    let _b = b_bytes[i] as u64;

                    cout = if abs_a < _b {
                        1
                    } else if abs_a == _b {
                        clt
                    } else {
                        0
                    };

                    // Encode the negation carry for the next byte
                    cout += 2 * (_a >> 8);
                    carry[i] = cout as u8;

                    let previous_cin = cin;
                    cin = cout;

                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = cout + 32;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        if i == 0 { 2 * pfirst[i] } else { plast[i] },
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
                row.set_all_carry(lane, &carry);
            }
            LT_ABS_PN_OP => {
                // Set first byte
                row.set_use_first_byte(lane, true);

                // Set result_is_a
                row.set_result_is_a(lane, false);

                // Set c_is_signed
                row.set_c_is_signed(lane, false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::LtAbsPN;

                let mut carry = [0u8; 8];
                for i in 0..8 {
                    let _a = a_bytes[i] as i64;
                    let _b = (b_bytes[i] as u64 ^ 0xFF) as i64;
                    let sub = if pfirst[i] == 1 { _a - (_b + 1) } else { _a - _b };

                    // Calculate the output carry
                    match sub.cmp(&0) {
                        CmpOrdering::Less => {
                            cout = 1;
                        }
                        CmpOrdering::Equal => {
                            cout = cin;
                        }
                        CmpOrdering::Greater => {
                            cout = 0;
                        }
                    }
                    carry[i] = cout as u8;

                    let previous_cin = cin;
                    cin = cout;

                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = cout + 32;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        if i == 0 { 2 * pfirst[i] } else { plast[i] },
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
                row.set_all_carry(lane, &carry);
            }
            LTU_OP | LTUW_OP | LT_OP | LTW_OP => {
                // Set first byte
                row.set_use_first_byte(lane, false);

                // Set result_is_a
                row.set_result_is_a(lane, false);

                // Set c_is_signed
                row.set_c_is_signed(lane, false);

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == LTU_OP) || (opcode == LTUW_OP) {
                    BinaryBasicTableOp::Ltu
                } else {
                    BinaryBasicTableOp::Lt
                };

                let mut carry = [0u8; 8];
                for i in 0..8 {
                    // Calculate carry
                    match a_bytes[i].cmp(&b_bytes[i]) {
                        CmpOrdering::Greater => {
                            cout = 0;
                        }
                        CmpOrdering::Less => {
                            cout = 1;
                        }
                        CmpOrdering::Equal => {
                            cout = cin;
                        }
                    }

                    // If the chunk is signed, then the result is the sign of a
                    if (binary_basic_table_op.eq(&BinaryBasicTableOp::Lt))
                        && (plast[i] == 1)
                        && (a_bytes[i] & SIGN_BYTE) != (b_bytes[i] & SIGN_BYTE)
                    {
                        cout = if a_bytes[i] & SIGN_BYTE != 0 { 1 } else { 0 };
                    }
                    carry[i] = cout as u8;

                    let previous_cin = cin;
                    cin = cout;

                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = cin;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            BinaryBasicTableOp::Sext00
                        } else {
                            binary_basic_table_op
                        },
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
                row.set_all_carry(lane, &carry);
            }
            GT_OP => {
                // Set first byte
                row.set_use_first_byte(lane, false);

                // Set result_is_a
                row.set_result_is_a(lane, false);

                // Set c_is_signed
                row.set_c_is_signed(lane, false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Gt;

                let mut carry = [0u8; 8];
                for i in 0..8 {
                    // Calculate carry
                    match a_bytes[i].cmp(&b_bytes[i]) {
                        CmpOrdering::Greater => {
                            cout = 1;
                        }
                        CmpOrdering::Less => {
                            cout = 0;
                        }
                        CmpOrdering::Equal => {
                            cout = cin;
                        }
                    }

                    // The result is the sign of b
                    if (plast[i] == 1) && (a_bytes[i] & SIGN_BYTE) != (b_bytes[i] & SIGN_BYTE) {
                        cout = if b_bytes[i] & SIGN_BYTE != 0 { 1 } else { 0 };
                    }
                    carry[i] = cout as u8;

                    let previous_cin = cin;
                    cin = cout;

                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = cout;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
                row.set_all_carry(lane, &carry);
            }
            EQ_OP | EQW_OP => {
                // Set first byte
                row.set_use_first_byte(lane, false);

                // Set result_is_a
                row.set_result_is_a(lane, false);

                // Set c_is_signed
                row.set_c_is_signed(lane, false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Eq;

                let mut carry = [0u8; 8];
                for i in 0..8 {
                    // Calculate carry
                    if (a_bytes[i] == b_bytes[i]) && (cin == 0) {
                        cout = 0;
                    } else {
                        cout = 1;
                    }
                    if plast[i] == 1 {
                        cout = 1 - cout;
                    }
                    carry[i] = cout as u8;

                    let previous_cin = cin;
                    cin = cout;

                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = cout;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            BinaryBasicTableOp::Sext00
                        } else {
                            binary_basic_table_op
                        },
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
                row.set_all_carry(lane, &carry);
            }
            ADD_OP | ADDW_OP => {
                // Set first byte
                row.set_use_first_byte(lane, false);

                // Set result_is_a
                row.set_result_is_a(lane, false);

                // Set c_is_signed
                row.set_c_is_signed(lane, c_is_signed != 0);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Add;

                let mut carry = [0u8; 8];
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
                    let result = cin + a_bytes[i] as u64 + b_bytes[i] as u64;
                    cout = result >> 8;
                    cin = if i == carry_byte { 0 } else { cout };
                    carry[i] = cin as u8;

                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = cin + 64 * plast[i] * c_is_signed;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            if c_is_signed == 1 {
                                BinaryBasicTableOp::SextFF
                            } else {
                                BinaryBasicTableOp::Sext00
                            }
                        } else {
                            binary_basic_table_op
                        },
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
                row.set_all_carry(lane, &carry);
            }
            SUB_OP | SUBW_OP => {
                // Set first byte
                row.set_use_first_byte(lane, false);

                // Set result_is_a
                row.set_result_is_a(lane, false);

                // Set c_is_signed
                row.set_c_is_signed(lane, c_is_signed != 0);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Sub;

                let mut carry = [0u8; 8];
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
                    cout = if a_bytes[i] as u64 >= (b_bytes[i] as u64 + cin) { 0 } else { 1 };
                    cin = if i == carry_byte { 0 } else { cout };
                    carry[i] = cin as u8;

                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = cin + 64 * plast[i] * c_is_signed;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            if c_is_signed == 1 {
                                BinaryBasicTableOp::SextFF
                            } else {
                                BinaryBasicTableOp::Sext00
                            }
                        } else {
                            binary_basic_table_op
                        },
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
                row.set_all_carry(lane, &carry);
            }
            LEU_OP | LEUW_OP | LE_OP | LEW_OP => {
                // Set first byte
                row.set_use_first_byte(lane, false);

                // Set result_is_a
                row.set_result_is_a(lane, false);

                // Set c_is_signed
                row.set_c_is_signed(lane, false);

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == LEU_OP) || (opcode == LEUW_OP) {
                    BinaryBasicTableOp::Leu
                } else {
                    BinaryBasicTableOp::Le
                };

                // Compute all carries first
                let mut carry = [0u8; 8];
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;

                    if a_bytes[i] < b_bytes[i] {
                        cout = 0;
                    } else if a_bytes[i] == b_bytes[i] {
                        cout = cin;
                    } else {
                        cout = 1;
                    }
                    if plast[i] == 1 {
                        cout = 1 - cout;

                        if (binary_basic_table_op == BinaryBasicTableOp::Le)
                            && (a_bytes[i] & SIGN_BYTE) != (b_bytes[i] & SIGN_BYTE)
                        {
                            cout = if a_bytes[i] & SIGN_BYTE != 0 { 1 } else { 0 };
                        }
                    }
                    cin = cout;
                    carry[i] = cin as u8;

                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = cin;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            BinaryBasicTableOp::Sext00
                        } else {
                            binary_basic_table_op
                        },
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
                row.set_all_carry(lane, &carry);
            }
            AND_OP => {
                // Set first byte
                row.set_use_first_byte(lane, false);

                // Set result_is_a
                row.set_result_is_a(lane, false);

                // Set c_is_signed
                row.set_c_is_signed(lane, false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::And;

                // No carry
                row.set_all_carry(lane, &[0u8; 8]);

                for i in 0..8 {
                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = 0;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        0,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
            }
            OR_OP => {
                // Set first byte
                row.set_use_first_byte(lane, false);

                // Set result_is_a
                row.set_result_is_a(lane, false);

                // Set c_is_signed
                row.set_c_is_signed(lane, false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Or;

                // No carry
                row.set_all_carry(lane, &[0u8; 8]);

                for i in 0..8 {
                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = 0;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        0,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
            }
            XOR_OP => {
                // Set first byte
                row.set_use_first_byte(lane, false);

                // Set result_is_a
                row.set_result_is_a(lane, false);

                // Set c_is_signed
                row.set_c_is_signed(lane, false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Xor;

                // No carry
                row.set_all_carry(lane, &[0u8; 8]);

                for i in 0..8 {
                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = 0;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        0,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
            }
            ANDN_OP | ORN_OP | XNOR_OP | BREV8_OP => {
                // Bitwise ops with no carry, one table row per byte (like AND/OR/XOR).
                // ANDN/ORN/XNOR use both operands; BREV8 is unary (operand in b, output
                // depends only on b), which the table encodes independently of a.
                row.set_use_first_byte(lane, false);
                row.set_result_is_a(lane, false);
                row.set_c_is_signed(lane, false);

                binary_basic_table_op = match opcode {
                    ANDN_OP => BinaryBasicTableOp::Andn,
                    ORN_OP => BinaryBasicTableOp::Orn,
                    XNOR_OP => BinaryBasicTableOp::Xnor,
                    _ => BinaryBasicTableOp::Brev8,
                };

                // No carry
                row.set_all_carry(lane, &[0u8; 8]);

                for i in 0..8 {
                    let flags = 0;
                    let row = BinaryBasicTableSM::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        0,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
            }
            SH1ADD_OP | SH2ADD_OP | SH3ADD_OP => {
                // Zba shift-and-add: c = b + (a << shift), computed as an addition of the shifted
                // operand. The `shift` low bits of the shifted byte are always zero, so the bits
                // shifted out of the previous byte are simply added to it, together with the
                // addition carry: cin = (a_prev >> (8 - shift)) + carry, in [0, 2^shift]
                row.set_use_first_byte(lane, false);
                row.set_result_is_a(lane, false);
                row.set_c_is_signed(lane, false);

                binary_basic_table_op = match opcode {
                    SH1ADD_OP => BinaryBasicTableOp::Sh1add,
                    SH2ADD_OP => BinaryBasicTableOp::Sh2add,
                    _ => BinaryBasicTableOp::Sh3add,
                };
                let shift = binary_basic_table_op.shift();

                let mut carry = [0u8; 8];
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
                    let a_byte = a_bytes[i] as u64;
                    let result = ((a_byte << shift) & 0xFF) + b_bytes[i] as u64 + cin;

                    // The bits shifted out of this byte travel with the addition carry. Both are
                    // discarded in the last byte, since the result is truncated to 64 bits
                    cout = (result >> 8) + (a_byte >> (8 - shift));
                    cin = if i == carry_byte { 0 } else { cout };
                    carry[i] = cin as u8;

                    // FLAGS[i] = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
                    let flags = cin;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        binary_basic_table_op,
                        a_byte,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row_one(self.table_id, row);
                }
                row.set_all_carry(lane, &carry);
            }
            _ => panic!("BinaryBasicSM::process_slice() found invalid opcode={opcode}"),
        }

        // Set b_op
        row.set_b_op(lane, binary_basic_table_op as u8);
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// Operations are written slot by slot, in order: every lane of these airs proves every basic
    /// operation, so the fill is a plain sequential walk and the last row is the only partial one.
    pub fn compute_witness<T, R: BinaryBasicRow<F, T>>(
        &self,
        inputs: &[Vec<BinaryInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let lanes = BinaryLanes::new(R::LANES_X_ROW);
        let mut trace = R::new_trace(trace_buffer)?;

        let num_rows = R::trace_num_rows(&trace);
        let num_slots = lanes.slots(num_rows);

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(
            total_inputs <= num_slots,
            "Binary: {total_inputs} operations do not fit in {num_slots} slots",
        );

        tracing::debug!(
            "··· Creating Binary instance [{} / {} slots filled {:.2}%]",
            total_inputs,
            num_slots,
            total_inputs as f64 / num_slots as f64 * 100.0
        );

        // Slots are filled in order across the whole instance, so a chunk's operations can straddle
        // a row boundary. Rows are the unit of parallelism, so the walk is by row: each takes the
        // slice of the flattened inputs that belongs to it.
        let flat_inputs: Vec<&BinaryInput> = inputs.iter().flatten().collect();
        let rows_used = lanes.rows_for(total_inputs);
        let lanes_x_row = R::LANES_X_ROW;

        R::trace_buffer_mut(&mut trace)[..rows_used].par_iter_mut().enumerate().for_each(
            |(row_index, row)| {
                let base = row_index * lanes_x_row;
                let filled = lanes_x_row.min(total_inputs - base);
                for lane in 0..filled {
                    self.process_slice::<T, R>(row, lane, flat_inputs[base + lane]);
                }
                // Only the last row can be short. Its leftover lanes are not covered by the padding
                // rows written afterwards, and the trace buffer comes from a pool and is not zeroed,
                // so they get ADD(0,0), the padding operation, here.
                for lane in filled..lanes_x_row {
                    Self::set_padding_slot(row, lane);
                }
            },
        );

        // Every padded slot is one ADD(0,0) on the bus, whatever row it sits on: the leftover lanes
        // of the last filled row and every lane of the rows after it.
        let padding_size = num_slots - total_inputs;
        if padding_size > 0 {
            for last in 0..2 {
                let multiplicity = (7 - 6 * last) * padding_size as u64;
                let row = BinaryBasicTableSM::calculate_table_row(
                    BinaryBasicTableOp::Add,
                    0,
                    0,
                    0,
                    last,
                    0,
                );
                self.std.inc_virtual_row(self.table_id, row, multiplicity);
            }
        }

        let mut padding_row = R::default();
        for lane in 0..lanes_x_row {
            Self::set_padding_slot(&mut padding_row, lane);
        }

        Ok(R::into_air_instance(&mut trace, padding_row, rows_used, padding_size))
    }

    /// Writes ADD(0, 0) into one slot: the operation the air's `padding_size` cancels on the bus.
    #[inline(always)]
    fn set_padding_slot<T, R: BinaryBasicRow<F, T>>(row: &mut R, lane: usize) {
        row.set_b_op(lane, ADD_OP);
        row.set_mode32(lane, false);
        row.set_result_is_a(lane, false);
        row.set_use_first_byte(lane, false);
        row.set_c_is_signed(lane, false);
        row.set_all_free_in_a(lane, &[0; 8]);
        row.set_all_free_in_b(lane, &[0; 8]);
        row.set_all_free_in_c(lane, &[0; 8]);
        row.set_all_carry(lane, &[0; 8]);
    }
}
