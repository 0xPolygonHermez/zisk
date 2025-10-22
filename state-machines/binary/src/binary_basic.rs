//! The `BinaryBasicSM` module implements the logic for the Binary Basic State Machine.
//!
//! This state machine processes binary-related operations.

use std::sync::Arc;

use crate::{
    binary_constants::*, BinaryBasicFrops, BinaryBasicTableOp, BinaryBasicTableSM, BinaryInput,
};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use rayon::prelude::*;
use std::cmp::Ordering as CmpOrdering;
use zisk_core::zisk_ops::ZiskOp;
#[cfg(not(feature = "packed"))]
use zisk_pil::{BinaryTrace, BinaryTraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{BinaryTracePacked, BinaryTraceRowPacked};

#[cfg(feature = "packed")]
type BinaryTraceRowType<F> = BinaryTraceRowPacked<F>;
#[cfg(feature = "packed")]
type BinaryTraceType<F> = BinaryTracePacked<F>;

#[cfg(not(feature = "packed"))]
type BinaryTraceRowType<F> = BinaryTraceRow<F>;
#[cfg(not(feature = "packed"))]
type BinaryTraceType<F> = BinaryTrace<F>;

const BYTES: usize = 8;
const HALF_BYTES: usize = BYTES / 2;
const MASK_U64: u64 = 0xFFFF_FFFF_FFFF_FFFF;

/// The `BinaryBasicSM` struct encapsulates the logic of the Binary Basic State Machine.
pub struct BinaryBasicSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,

    /// The table ID for the Binary Basic State Machine
    table_id: usize,

    /// The table ID for the FROPS
    frops_table_id: usize,
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
        let table_id = std.get_virtual_table_id(BinaryBasicTableSM::TABLE_ID);

        // Get the FROPS table ID
        let frops_table_id = std.get_virtual_table_id(BinaryBasicFrops::TABLE_ID);

        Arc::new(Self { std, table_id, frops_table_id })
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

    /// Returns the initial carry value for a given opcode.
    ///
    /// # Arguments
    /// * `opcode` - The opcode to evaluate.
    ///
    /// # Returns
    /// The initial carry value (`u64`).
    fn get_initial_carry(opcode: u8) -> u64 {
        let is_zisk_op = ZiskOp::try_from_code(opcode).is_ok();
        if is_zisk_op {
            0
        } else {
            match opcode {
                LT_ABS_NP_OP | LT_ABS_PN_OP => 2,
                GT_OP => 0,
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
    /// # Returns
    /// A `BinaryTraceRow` representing the operation's result.
    #[inline(always)]
    pub fn process_slice(&self, input: &BinaryInput) -> BinaryTraceRowType<F> {
        // Create an empty trace
        let mut row: BinaryTraceRowType<F> = Default::default();

        // Execute the opcode
        let opcode = input.op;
        let a = input.a;
        let b = input.b;

        let (c, _) = Self::execute(input.op, input.a, input.b);

        // Set mode32
        let mode32 = Self::opcode_is_32_bits(opcode);
        let mode64 = !mode32;
        row.set_mode32(mode32);

        // Set c_filtered
        let c_filtered = if mode32 { c & 0xFFFFFFFF } else { c };

        // Split a in bytes and store them in free_in_a
        let a_bytes: [u8; 8] = a.to_le_bytes();
        for (i, value) in a_bytes.iter().enumerate() {
            row.set_free_in_a(i, *value);
        }

        // Split b in bytes and store them in free_in_b
        let b_bytes: [u8; 8] = b.to_le_bytes();
        for (i, value) in b_bytes.iter().enumerate() {
            row.set_free_in_b(i, *value);
        }

        // Split c in bytes and store them in free_in_c
        let c_bytes: [u8; 8] = c.to_le_bytes();
        for (i, value) in c_bytes.iter().enumerate() {
            row.set_free_in_c(i, *value);
        }

        // Set use last carry and carry[], based on operation
        let mut cout: u64;
        let mut cin: u64 = Self::get_initial_carry(opcode);
        let plast: [u64; 8] =
            if mode32 { [0, 0, 0, 1, 0, 0, 0, 0] } else { [0, 0, 0, 0, 0, 0, 0, 1] };

        // Calculate the byte that sets the carry
        let carry_byte = if mode32 { 3 } else { 7 };

        let binary_basic_table_op: BinaryBasicTableOp;
        match opcode {
            MINU_OP | MINUW_OP | MIN_OP | MINW_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(true);

                let result_is_a: u64 = if (a == b) || (b == c_filtered) { 0 } else { 1 };

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == MINU_OP) || (opcode == MINUW_OP) {
                    BinaryBasicTableOp::Minu
                } else {
                    BinaryBasicTableOp::Min
                };

                // Set use last carry to zero
                row.set_use_last_carry(false);

                // Set has initial carry
                row.set_has_initial_carry(false);

                // Apply the logic to every byte
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
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
                    if (binary_basic_table_op == BinaryBasicTableOp::Min)
                        && (plast[i] == 1)
                        && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80)
                    {
                        cout = if (a_bytes[i] & 0x80) != 0 { 1 } else { 0 };
                    }
                    if mode32 && (i >= 4) {
                        cout = 0;
                    }
                    cin = cout;
                    row.set_carry(i, cin != 0);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 2 + 4 * result_is_a;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            BinaryBasicTableOp::Ext32
                        } else {
                            binary_basic_table_op
                        },
                        if mode32 && (i >= 4) { c_bytes[3] as u64 } else { a_bytes[i] as u64 },
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            MAXU_OP | MAXUW_OP | MAX_OP | MAXW_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(true);

                let result_is_a: u64 = if (a == b) || (b == c_filtered) { 0 } else { 1 };

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == MAXU_OP) || (opcode == MAXUW_OP) {
                    BinaryBasicTableOp::Maxu
                } else {
                    BinaryBasicTableOp::Max
                };

                // Set use last carry to zero
                row.set_use_last_carry(false);

                // Set has initial carry
                row.set_has_initial_carry(false);

                // Apply the logic to every byte
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
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

                    // If the chunk is signed, then the result is the sign of a
                    if (binary_basic_table_op == BinaryBasicTableOp::Max)
                        && (plast[i] == 1)
                        && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80)
                    {
                        cout = if (a_bytes[i] & 0x80) != 0 { 0 } else { 1 };
                    }
                    if mode32 && (i >= 4) {
                        cout = 0;
                    }
                    cin = cout;
                    row.set_carry(i, cin != 0);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 2 + 4 * result_is_a;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            BinaryBasicTableOp::Ext32
                        } else {
                            binary_basic_table_op
                        },
                        if mode32 && (i >= 4) { c_bytes[3] as u64 } else { a_bytes[i] as u64 },
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            LT_ABS_NP_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::LtAbsNP;

                // Set use last carry
                row.set_use_last_carry(true);

                // Set has initial carry
                row.set_has_initial_carry(true);

                // Apply the logic to every byte
                for i in 0..8 {
                    let _clt = cin & 0x01;
                    let _cop = (cin & 0x02) >> 1;

                    let _a = (a_bytes[i] as u64 ^ 0xFF) + _cop;
                    let _b = b_bytes[i] as u64;

                    // Calculate the output carry
                    let previous_cin = cin;
                    match (_a & 0xFF).cmp(&_b) {
                        CmpOrdering::Less => {
                            cout = 1;
                        }
                        CmpOrdering::Equal => {
                            cout = _clt;
                        }
                        CmpOrdering::Greater => {
                            cout = 0;
                        }
                    }

                    cout += 2 * (_a >> 8);
                    row.set_carry(i, cout != 0);

                    // Set carry for next iteration
                    cin = cout;

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            LT_ABS_PN_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::LtAbsPN;

                // Set use last carry
                row.set_use_last_carry(true);

                // Set has initial carry
                row.set_has_initial_carry(true);

                // Apply the logic to every byte
                for i in 0..8 {
                    let _clt = cin & 0x1;
                    let _cop = (cin & 0x02) >> 1;

                    let _a = a_bytes[i] as u64;
                    let _b = (b_bytes[i] as u64 ^ 0xFF) + _cop;

                    // Calculate the output carry
                    let previous_cin = cin;
                    match _a.cmp(&(_b & 0xFF)) {
                        CmpOrdering::Less => {
                            cout = 1;
                        }
                        CmpOrdering::Equal => {
                            cout = _clt;
                        }
                        CmpOrdering::Greater => {
                            cout = 0;
                        }
                    }

                    cout += 2 * (_b >> 8);
                    row.set_carry(i, cout != 0);

                    // Set carry for next iteration
                    cin = cout;

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            LTU_OP | LTUW_OP | LT_OP | LTW_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(false);

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == LTU_OP) || (opcode == LTUW_OP) {
                    BinaryBasicTableOp::Ltu
                } else {
                    BinaryBasicTableOp::Lt
                };

                // Set use last carry to one
                row.set_use_last_carry(true);

                // Set has initial carry
                row.set_has_initial_carry(false);

                // Apply the logic to every byte
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
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
                        && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80)
                    {
                        cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                    }
                    cin = cout;
                    row.set_carry(i, cin != 0);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cin + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            BinaryBasicTableOp::Ext32
                        } else {
                            binary_basic_table_op
                        },
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            GT_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Gt;

                // Set use last carry to one
                row.set_use_last_carry(true);

                // Set has initial carry
                row.set_has_initial_carry(false);

                // Apply the logic to every byte
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
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
                    if (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                        cout = if b_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                    }
                    row.set_carry(i, cout != 0);

                    // Set carry for next iteration
                    cin = cout;

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            EQ_OP | EQW_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Eq;

                // Set use last carry to one
                row.set_use_last_carry(true);

                // Set has initial carry
                row.set_has_initial_carry(false);

                // Apply the logic to every byte
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
                    if (a_bytes[i] == b_bytes[i]) && (cin == 0) {
                        cout = 0;
                    } else {
                        cout = 1;
                    }
                    if plast[i] == 1 {
                        cout = 1 - cout;
                    }
                    cin = cout;
                    row.set_carry(i, cin != 0);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            BinaryBasicTableOp::Ext32
                        } else {
                            binary_basic_table_op
                        },
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            ADD_OP | ADDW_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Add;

                // Set use last carry to zero
                row.set_use_last_carry(false);

                // Set has initial carry
                row.set_has_initial_carry(false);

                // Apply the logic to every byte
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
                    let result = cin + a_bytes[i] as u64 + b_bytes[i] as u64;
                    cout = result >> 8;
                    cin = if i == carry_byte { 0 } else { cout };
                    row.set_carry(i, cin != 0);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cin;

                    // Set a and b bytes
                    let a_byte = if mode32 && (i >= 4) { c_bytes[3] } else { a_bytes[i] };
                    let b_byte = if mode32 && (i >= 4) { 0 } else { b_bytes[i] };

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            BinaryBasicTableOp::Ext32
                        } else {
                            binary_basic_table_op
                        },
                        a_byte as u64,
                        b_byte as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            SUB_OP | SUBW_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Sub;

                // Set use last carry to zero
                row.set_use_last_carry(false);

                // Set has initial carry
                row.set_has_initial_carry(false);

                // Apply the logic to every byte
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
                    cout = if a_bytes[i] as u64 >= (b_bytes[i] as u64 + cin) { 0 } else { 1 };
                    cin = if i == carry_byte { 0 } else { cout };
                    row.set_carry(i, cin != 0);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cin;

                    // Set a and b bytes
                    let a_byte = if mode32 && (i >= 4) { c_bytes[3] } else { a_bytes[i] };
                    let b_byte = if mode32 && (i >= 4) { 0 } else { b_bytes[i] };

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            BinaryBasicTableOp::Ext32
                        } else {
                            binary_basic_table_op
                        },
                        a_byte as u64,
                        b_byte as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            LEU_OP | LEUW_OP | LE_OP | LEW_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(false);

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == LEU_OP) || (opcode == LEUW_OP) {
                    BinaryBasicTableOp::Leu
                } else {
                    BinaryBasicTableOp::Le
                };

                // Set use last carry to one
                row.set_use_last_carry(true);

                // Set has initial carry
                row.set_has_initial_carry(false);

                // Apply the logic to every byte
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
                    cout = 0;
                    if a_bytes[i] <= b_bytes[i] {
                        cout = 1;
                    }
                    if (binary_basic_table_op == BinaryBasicTableOp::Le)
                        && (plast[i] == 1)
                        && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80)
                    {
                        cout = c;
                    }
                    cin = cout;
                    row.set_carry(i, cin != 0);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cin + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::calculate_table_row(
                        if mode32 && (i >= 4) {
                            BinaryBasicTableOp::Ext32
                        } else {
                            binary_basic_table_op
                        },
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            AND_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::And;

                row.set_use_last_carry(false);

                // Set has initial carry
                row.set_has_initial_carry(false);

                // No carry
                for i in 0..8 {
                    row.set_carry(i, false);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
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
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            OR_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Or;

                row.set_use_last_carry(false);

                // Set has initial carry
                row.set_has_initial_carry(false);

                // No carry
                for i in 0..8 {
                    row.set_carry(i, false);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
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
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            XOR_OP => {
                // Set opcode is min or max
                row.set_op_is_min_max(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Xor;

                // Set use last carry to zero
                row.set_use_last_carry(false);

                // Set has initial carry
                row.set_has_initial_carry(false);

                // No carry
                for i in 0..8 {
                    row.set_carry(i, false);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
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
                    self.std.inc_virtual_row(self.table_id, row, 1);
                }
            }
            _ => panic!("BinaryBasicSM::process_slice() found invalid opcode={opcode}"),
        }

        // Set cout
        let cout32 = row.get_carry(HALF_BYTES - 1);
        let cout64 = row.get_carry(BYTES - 1);
        row.set_cout(if mode64 { cout64 } else { cout32 });

        // Set result_is_a
        row.set_result_is_a(row.get_op_is_min_max() && row.get_cout());

        // Set use_last_carry_mode32 and use_last_carry_mode64
        row.set_use_last_carry_mode32(mode32 && row.get_use_last_carry());
        row.set_use_last_carry_mode64(mode64 && row.get_use_last_carry());

        // Set micro opcode
        row.set_m_op(binary_basic_table_op as u8);

        // Set m_op_or_ext
        let ext_32_op = BinaryBasicTableOp::Ext32 as u8;

        row.set_m_op_or_ext(if mode64 { binary_basic_table_op as u8 } else { ext_32_op });

        // Set free_in_a_or_c and free_in_b_or_zero
        for i in 0..HALF_BYTES {
            row.set_free_in_a_or_c(
                i,
                if mode64 {
                    row.get_free_in_a(i + HALF_BYTES)
                } else {
                    row.get_free_in_c(HALF_BYTES - 1)
                },
            );
            row.set_free_in_b_or_zero(i, mode64 as u8 * row.get_free_in_b(i + HALF_BYTES));
        }

        if row.get_use_last_carry() {
            // Set first and last elements
            row.set_free_in_c(7, row.get_free_in_c(0));
            row.set_free_in_c(0, 0);
        }

        // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
        row.set_multiplicity(true);

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
    pub fn compute_witness(
        &self,
        inputs: &[Vec<BinaryInput>],
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut binary_trace = BinaryTraceType::new_from_vec(trace_buffer);

        let num_rows = binary_trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        tracing::info!(
            "··· Creating Binary instance [{} / {} rows filled {:.2}%]",
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        // Split the binary_e_trace.buffer into slices matching each inner vector’s length.
        let sizes: Vec<usize> = inputs.iter().map(|v| v.len()).collect();
        let mut slices = Vec::with_capacity(inputs.len());
        let mut rest = &mut binary_trace.buffer[..];
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

        // Note: We can choose any operation that trivially satisfies the constraints on padding
        // rows
        let mut padding_row = BinaryTraceRowType::default();
        padding_row.set_m_op(AND_OP);
        padding_row.set_m_op_or_ext(AND_OP);

        binary_trace.buffer[total_inputs..num_rows]
            .par_iter_mut()
            .for_each(|slot| *slot = padding_row);

        let padding_size = num_rows - total_inputs;
        for last in 0..2 {
            let multiplicity = (7 - 6 * last as u64) * padding_size as u64;
            let row = BinaryBasicTableSM::calculate_table_row(
                BinaryBasicTableOp::And,
                0,
                0,
                0,
                last as u64,
                0,
            );
            self.std.inc_virtual_row(self.table_id, row, multiplicity);
        }

        AirInstance::new_from_trace(FromTrace::new(&mut binary_trace))
    }
    pub fn compute_frops(&self, frops_inputs: &Vec<u32>) {
        for row in frops_inputs {
            self.std.inc_virtual_row(self.frops_table_id, *row as u64, 1);
        }
    }
}
