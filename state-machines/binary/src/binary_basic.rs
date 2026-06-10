//! The `BinaryBasicSM` module implements the logic for the Binary Basic State Machine.
//!
//! This state machine processes binary-related operations.

use std::sync::Arc;

use crate::{binary_constants::*, BinaryBasicTableOp, BinaryBasicTableSM, BinaryInput};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use rayon::prelude::*;
use std::cmp::Ordering as CmpOrdering;
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{BinaryAirValues, BinaryTrace, BinaryTraceRowOps};

const MASK_U64: u64 = 0xFFFF_FFFF_FFFF_FFFF;
const SIGN_BYTE: u8 = 0x80;

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
    /// # Returns
    /// A `BinaryTraceRow` representing the operation's result.
    #[inline(always)]
    pub fn process_slice<R: BinaryTraceRowOps<F>>(&self, input: &BinaryInput) -> R {
        // Create an empty trace
        let mut row: R = R::default();

        // Execute the opcode
        let opcode = input.op;
        let a = input.a;
        let b = input.b;

        let (c, _) = Self::execute(input.op, input.a, input.b);

        // Set mode32
        let mode32 = Self::opcode_is_32_bits(opcode);
        row.set_mode32(mode32);

        // Set c_filtered
        let c_filtered = if mode32 { c & 0xFF_FF_FF_FF } else { c };

        // Split a, b, c into bytes
        let a_bytes: [u8; 8] = a.to_le_bytes();
        let b_bytes: [u8; 8] = b.to_le_bytes();
        let c_bytes: [u8; 8] = c.to_le_bytes();

        // Store bytes in free_in_a, free_in_b, free_in_c
        row.set_all_free_in_a(&a_bytes);
        row.set_all_free_in_b(&b_bytes);
        if Self::opcode_is_comparator(opcode) {
            row.set_all_free_in_c(&[0u8; 8]);
        } else {
            row.set_all_free_in_c(&c_bytes);
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
                row.set_use_first_byte(false);

                // Set result_is_a
                let result_is_a: u64 = if (a == b) || (b == c_filtered) { 0 } else { 1 };
                row.set_result_is_a(result_is_a != 0);

                // Set c_is_signed
                row.set_c_is_signed(c_is_signed != 0);

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

                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
                    let flags = cout + 2 * result_is_a + 8 * plast[i] * c_is_signed;

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
                row.set_all_carry(&carry);
            }
            MAXU_OP | MAXUW_OP | MAX_OP | MAXW_OP => {
                // Set first byte
                row.set_use_first_byte(false);

                // Set result_is_a
                let result_is_a: u64 = if (a == b) || (b == c_filtered) { 0 } else { 1 };
                row.set_result_is_a(result_is_a != 0);

                // Set c_is_signed
                row.set_c_is_signed(c_is_signed != 0);

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

                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
                    let flags = cout + 2 * result_is_a + 8 * plast[i] * c_is_signed;

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
                row.set_all_carry(&carry);
            }
            LT_ABS_NP_OP => {
                // Set first byte
                row.set_use_first_byte(true);

                // Set result_is_a
                row.set_result_is_a(false);

                // Set c_is_signed
                row.set_c_is_signed(false);

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

                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
                    let flags = cout + 4;

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
                row.set_all_carry(&carry);
            }
            LT_ABS_PN_OP => {
                // Set first byte
                row.set_use_first_byte(true);

                // Set result_is_a
                row.set_result_is_a(false);

                // Set c_is_signed
                row.set_c_is_signed(false);

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

                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
                    let flags = cout + 4;

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
                row.set_all_carry(&carry);
            }
            LTU_OP | LTUW_OP | LT_OP | LTW_OP => {
                // Set first byte
                row.set_use_first_byte(false);

                // Set result_is_a
                row.set_result_is_a(false);

                // Set c_is_signed
                row.set_c_is_signed(false);

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

                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
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
                row.set_all_carry(&carry);
            }
            GT_OP => {
                // Set first byte
                row.set_use_first_byte(false);

                // Set result_is_a
                row.set_result_is_a(false);

                // Set c_is_signed
                row.set_c_is_signed(false);

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

                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
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
                row.set_all_carry(&carry);
            }
            EQ_OP | EQW_OP => {
                // Set first byte
                row.set_use_first_byte(false);

                // Set result_is_a
                row.set_result_is_a(false);

                // Set c_is_signed
                row.set_c_is_signed(false);

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

                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
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
                row.set_all_carry(&carry);
            }
            ADD_OP | ADDW_OP => {
                // Set first byte
                row.set_use_first_byte(false);

                // Set result_is_a
                row.set_result_is_a(false);

                // Set c_is_signed
                row.set_c_is_signed(c_is_signed != 0);

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

                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
                    let flags = cin + 8 * plast[i] * c_is_signed;

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
                row.set_all_carry(&carry);
            }
            SUB_OP | SUBW_OP => {
                // Set first byte
                row.set_use_first_byte(false);

                // Set result_is_a
                row.set_result_is_a(false);

                // Set c_is_signed
                row.set_c_is_signed(c_is_signed != 0);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Sub;

                let mut carry = [0u8; 8];
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
                    cout = if a_bytes[i] as u64 >= (b_bytes[i] as u64 + cin) { 0 } else { 1 };
                    cin = if i == carry_byte { 0 } else { cout };
                    carry[i] = cin as u8;

                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
                    let flags = cin + 8 * plast[i] * c_is_signed;

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
                row.set_all_carry(&carry);
            }
            LEU_OP | LEUW_OP | LE_OP | LEW_OP => {
                // Set first byte
                row.set_use_first_byte(false);

                // Set result_is_a
                row.set_result_is_a(false);

                // Set c_is_signed
                row.set_c_is_signed(false);

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
                    cout = 0;
                    if a_bytes[i] <= b_bytes[i] {
                        cout = 1;
                    }
                    if (binary_basic_table_op == BinaryBasicTableOp::Le)
                        && (plast[i] == 1)
                        && (a_bytes[i] & SIGN_BYTE) != (b_bytes[i] & SIGN_BYTE)
                    {
                        cout = c;
                    }
                    cin = cout;
                    carry[i] = cin as u8;

                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
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
                row.set_all_carry(&carry);
            }
            AND_OP => {
                // Set first byte
                row.set_use_first_byte(false);

                // Set result_is_a
                row.set_result_is_a(false);

                // Set c_is_signed
                row.set_c_is_signed(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::And;

                // No carry
                row.set_all_carry(&[0u8; 8]);

                for i in 0..8 {
                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
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
                row.set_use_first_byte(false);

                // Set result_is_a
                row.set_result_is_a(false);

                // Set c_is_signed
                row.set_c_is_signed(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Or;

                // No carry
                row.set_all_carry(&[0u8; 8]);

                for i in 0..8 {
                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
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
                row.set_use_first_byte(false);

                // Set result_is_a
                row.set_result_is_a(false);

                // Set c_is_signed
                row.set_c_is_signed(false);

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Xor;

                // No carry
                row.set_all_carry(&[0u8; 8]);

                for i in 0..8 {
                    // FLAGS[i] = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
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
            _ => panic!("BinaryBasicSM::process_slice() found invalid opcode={opcode}"),
        }

        // Set b_op
        row.set_b_op(binary_basic_table_op as u8);

        row
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// The trace layout (packed/non-packed) is determined by `R`, fixed at construction.
    pub fn compute_witness<R: BinaryTraceRowOps<F>>(
        &self,
        inputs: &[Vec<BinaryInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = BinaryTrace::<R>::new_from_vec(trace_buffer)?;
        let num_rows = trace.num_rows();
        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        tracing::debug!(
            "··· Creating Binary instance [{} / {} rows filled {:.2}%]",
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        // Split the buffer into per-chunk slices and fill in parallel.
        let sizes: Vec<usize> = inputs.iter().map(|v| v.len()).collect();
        let mut slices = Vec::with_capacity(inputs.len());
        let mut rest = &mut trace.buffer[..];
        for size in sizes {
            let (head, tail) = rest.split_at_mut(size);
            slices.push(head);
            rest = tail;
        }
        slices.into_par_iter().enumerate().for_each(|(i, slice)| {
            slice.iter_mut().enumerate().for_each(|(j, row)| {
                *row = self.process_slice::<R>(&inputs[i][j]);
            });
        });

        // Set ADD(0,0) as the padding row
        let padding_size = num_rows - total_inputs;
        if padding_size > 0 {
            let mut padding_row = R::default();
            padding_row.set_b_op(ADD_OP);
            trace.buffer[total_inputs..num_rows]
                .par_iter_mut()
                .for_each(|slot| *slot = padding_row);

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

        let mut air_values = BinaryAirValues::<F>::new();
        air_values.padding_size = F::from_usize(padding_size);
        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }
}
