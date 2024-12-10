use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use log::info;
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use std::cmp::Ordering as CmpOrdering;
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredOperation};
use zisk_pil::*;

use crate::{BinaryBasicTableOp, BinaryBasicTableSM};

// 64 bits opcodes
const MINU_OP: u8 = ZiskOp::Minu.code();
const MIN_OP: u8 = ZiskOp::Min.code();
const MAXU_OP: u8 = ZiskOp::Maxu.code();
const MAX_OP: u8 = ZiskOp::Max.code();
pub const LT_ABS_NP_OP: u8 = 0x06;
pub const LT_ABS_PN_OP: u8 = 0x07;
pub const LTU_OP: u8 = ZiskOp::Ltu.code();
const LT_OP: u8 = ZiskOp::Lt.code();
pub const GT_OP: u8 = 0x0a;
const EQ_OP: u8 = ZiskOp::Eq.code();
const ADD_OP: u8 = ZiskOp::Add.code();
const SUB_OP: u8 = ZiskOp::Sub.code();
const LEU_OP: u8 = ZiskOp::Leu.code();
const LE_OP: u8 = ZiskOp::Le.code();
const AND_OP: u8 = ZiskOp::And.code();
const OR_OP: u8 = ZiskOp::Or.code();
const XOR_OP: u8 = ZiskOp::Xor.code();

// 32 bits opcodes
const MINUW_OP: u8 = ZiskOp::MinuW.code();
const MINW_OP: u8 = ZiskOp::MinW.code();
const MAXUW_OP: u8 = ZiskOp::MaxuW.code();
const MAXW_OP: u8 = ZiskOp::MaxW.code();
const LTUW_OP: u8 = ZiskOp::LtuW.code();
const LTW_OP: u8 = ZiskOp::LtW.code();
const EQW_OP: u8 = ZiskOp::EqW.code();
const ADDW_OP: u8 = ZiskOp::AddW.code();
const SUBW_OP: u8 = ZiskOp::SubW.code();
const LEUW_OP: u8 = ZiskOp::LeuW.code();
const LEW_OP: u8 = ZiskOp::LeW.code();

const BYTES: usize = 8;
const HALF_BYTES: usize = BYTES / 2;
const MASK_U64: u64 = 0xFFFF_FFFF_FFFF_FFFF;

pub struct BinaryBasicSM<F> {
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,

    // Secondary State machines
    binary_basic_table_sm: Arc<BinaryBasicTableSM<F>>,
}

#[derive(Debug)]
pub enum BinaryBasicSMErr {
    InvalidOpcode,
}

impl<F: Field> BinaryBasicSM<F> {
    const MY_NAME: &'static str = "Binary  ";

    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        binary_basic_table_sm: Arc<BinaryBasicTableSM<F>>,
        airgroup_id: usize,
        air_ids: &[usize],
    ) -> Arc<Self> {
        let binary_basic = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            inputs: Mutex::new(Vec::new()),
            binary_basic_table_sm,
        };
        let binary_basic = Arc::new(binary_basic);

        wcm.register_component(binary_basic.clone(), Some(airgroup_id), Some(air_ids));

        binary_basic.binary_basic_table_sm.register_predecessor();

        binary_basic
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            // If there are remaining inputs, prove them
            self.prove(&[], true);

            self.binary_basic_table_sm.unregister_predecessor();
        }
    }

    pub fn operations() -> Vec<u8> {
        vec![
            MINU_OP,
            MIN_OP,
            MAXU_OP,
            MAX_OP,
            LT_ABS_NP_OP,
            LT_ABS_PN_OP,
            LTU_OP,
            LT_OP,
            GT_OP,
            EQ_OP,
            ADD_OP,
            SUB_OP,
            LEU_OP,
            LE_OP,
            AND_OP,
            OR_OP,
            XOR_OP,
            MINUW_OP,
            MINW_OP,
            MAXUW_OP,
            MAXW_OP,
            LTUW_OP,
            LTW_OP,
            EQW_OP,
            ADDW_OP,
            SUBW_OP,
            LEUW_OP,
            LEW_OP,
        ]
    }

    fn opcode_is_32_bits(opcode: u8) -> bool {
        const OPCODES_32_BITS: [u8; 11] = [
            MINUW_OP, MINW_OP, MAXUW_OP, MAXW_OP, LTUW_OP, LTW_OP, EQW_OP, ADDW_OP, SUBW_OP,
            LEUW_OP, LEW_OP,
        ];

        OPCODES_32_BITS.contains(&opcode)
    }

    fn lt_abs_np_execute(a: u64, b: u64) -> (u64, bool) {
        let a_pos = (a ^ MASK_U64).wrapping_add(1);
        if a_pos < b {
            (1, true)
        } else {
            (0, false)
        }
    }

    fn lt_abs_pn_execute(a: u64, b: u64) -> (u64, bool) {
        let b_pos = (b ^ MASK_U64).wrapping_add(1);
        if a < b_pos {
            (1, true)
        } else {
            (0, false)
        }
    }

    fn gt_execute(a: u64, b: u64) -> (u64, bool) {
        if (a as i64) > (b as i64) {
            (1, true)
        } else {
            (0, false)
        }
    }

    fn execute(opcode: u8, a: u64, b: u64) -> (u64, bool) {
        let is_zisk_op = ZiskOp::try_from_code(opcode).is_ok();
        if is_zisk_op {
            ZiskOp::execute(opcode, a, b)
        } else {
            match opcode {
                LT_ABS_NP_OP => Self::lt_abs_np_execute(a, b),
                LT_ABS_PN_OP => Self::lt_abs_pn_execute(a, b),
                GT_OP => Self::gt_execute(a, b),
                _ => panic!("BinaryBasicSM::execute() got invalid opcode={:?}", opcode),
            }
        }
    }

    fn get_inital_carry(opcode: u8) -> u64 {
        let is_zisk_op = ZiskOp::try_from_code(opcode).is_ok();
        if is_zisk_op {
            0
        } else {
            match opcode {
                LT_ABS_NP_OP | LT_ABS_PN_OP => 2,
                GT_OP => 0,
                _ => panic!("BinaryBasicSM::execute() got invalid opcode={:?}", opcode),
            }
        }
    }

    #[inline(always)]
    pub fn process_slice(
        operation: &ZiskRequiredOperation,
        multiplicity: &mut [u64],
    ) -> BinaryRow<F> {
        // Create an empty trace
        let mut row: BinaryRow<F> = Default::default();

        // Execute the opcode
        let opcode = operation.opcode;
        let (c, _) = Self::execute(opcode, operation.a, operation.b);

        // Set mode32
        let mode32 = Self::opcode_is_32_bits(opcode);
        row.mode32 = F::from_bool(mode32);
        let mode64 = F::from_bool(!mode32);

        // Set c_filtered
        let c_filtered = if mode32 { c & 0xFFFFFFFF } else { c };

        // Split a in bytes and store them in free_in_a
        let a_bytes: [u8; 8] = operation.a.to_le_bytes();
        for (i, value) in a_bytes.iter().enumerate() {
            row.free_in_a[i] = F::from_canonical_u8(*value);
        }

        // Split b in bytes and store them in free_in_b
        let b_bytes: [u8; 8] = operation.b.to_le_bytes();
        for (i, value) in b_bytes.iter().enumerate() {
            row.free_in_b[i] = F::from_canonical_u8(*value);
        }

        // Split c in bytes and store them in free_in_c
        let c_bytes: [u8; 8] = c.to_le_bytes();
        for (i, value) in c_bytes.iter().enumerate() {
            row.free_in_c[i] = F::from_canonical_u8(*value);
        }

        // Set main SM step
        row.main_step = F::from_canonical_u64(operation.step);

        // Set use last carry and carry[], based on operation
        let mut cout: u64;
        let mut cin: u64 = Self::get_inital_carry(opcode);
        let plast: [u64; 8] =
            if mode32 { [0, 0, 0, 1, 0, 0, 0, 0] } else { [0, 0, 0, 0, 0, 0, 0, 1] };

        // Calculate the byte that sets the carry
        let carry_byte = if mode32 { 3 } else { 7 };

        let binary_basic_table_op: BinaryBasicTableOp;
        match opcode {
            MINU_OP | MINUW_OP | MIN_OP | MINW_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::one();

                let result_is_a: u64 =
                    if (operation.a == operation.b) || (operation.b == c_filtered) { 0 } else { 1 };

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == MINU_OP) || (opcode == MINUW_OP) {
                    BinaryBasicTableOp::Minu
                } else {
                    BinaryBasicTableOp::Min
                };

                // Set use last carry to zero
                row.use_last_carry = F::zero();

                // Set has initial carry
                row.has_initial_carry = F::zero();

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
                    if (binary_basic_table_op == BinaryBasicTableOp::Min) &&
                        (plast[i] == 1) &&
                        (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80)
                    {
                        cout = if (a_bytes[i] & 0x80) != 0 { 1 } else { 0 };
                    }
                    if mode32 && (i >= 4) {
                        cout = 0;
                    }
                    cin = cout;
                    row.carry[i] = F::from_canonical_u64(cin);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 2 + 4 * result_is_a;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
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
                    multiplicity[row as usize] += 1;
                }
            }
            MAXU_OP | MAXUW_OP | MAX_OP | MAXW_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::one();

                let result_is_a: u64 =
                    if (operation.a == operation.b) || (operation.b == c_filtered) { 0 } else { 1 };

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == MAXU_OP) || (opcode == MAXUW_OP) {
                    BinaryBasicTableOp::Maxu
                } else {
                    BinaryBasicTableOp::Max
                };

                // Set use last carry to zero
                row.use_last_carry = F::zero();

                // Set has initial carry
                row.has_initial_carry = F::zero();

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
                    if (binary_basic_table_op == BinaryBasicTableOp::Max) &&
                        (plast[i] == 1) &&
                        (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80)
                    {
                        cout = if (a_bytes[i] & 0x80) != 0 { 0 } else { 1 };
                    }
                    if mode32 && (i >= 4) {
                        cout = 0;
                    }
                    cin = cout;
                    row.carry[i] = F::from_canonical_u64(cin);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 2 + 4 * result_is_a;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
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
                    multiplicity[row as usize] += 1;
                }
            }
            LT_ABS_NP_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::LtAbsNP;

                // Set use last carry
                row.use_last_carry = F::one();

                // Set has initial carry
                row.has_initial_carry = F::one();

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

                    cout += 2*(_a >> 8);
                    row.carry[i] = F::from_canonical_u64(cout);

                    // Set carry for next iteration
                    cin = cout;

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            LT_ABS_PN_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::LtAbsPN;

                // Set use last carry
                row.use_last_carry = F::one();

                // Set has initial carry
                row.has_initial_carry = F::one();

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

                    cout += 2*(_b >> 8);
                    row.carry[i] = F::from_canonical_u64(cout);

                    // Set carry for next iteration
                    cin = cout;

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            LTU_OP | LTUW_OP | LT_OP | LTW_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == LTU_OP) || (opcode == LTUW_OP) {
                    BinaryBasicTableOp::Ltu
                } else {
                    BinaryBasicTableOp::Lt
                };

                // Set use last carry to one
                row.use_last_carry = F::one();

                // Set has initial carry
                row.has_initial_carry = F::zero();

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
                    if (binary_basic_table_op.eq(&BinaryBasicTableOp::Lt)) &&
                        (plast[i] == 1) &&
                        (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80)
                    {
                        cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                    }
                    cin = cout;
                    row.carry[i] = F::from_canonical_u64(cin);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cin + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
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
                    multiplicity[row as usize] += 1;
                }
            }
            GT_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Gt;

                // Set use last carry to one
                row.use_last_carry = F::one();

                // Set has initial carry
                row.has_initial_carry = F::zero();

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
                    if (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80)
                    {
                        cout = if b_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                    }
                    row.carry[i] = F::from_canonical_u64(cout);

                    // Set carry for next iteration
                    cin = cout;

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        flags,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            EQ_OP | EQW_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Eq;

                // Set use last carry to one
                row.use_last_carry = F::one();

                // Set has initial carry
                row.has_initial_carry = F::zero();

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
                    row.carry[i] = F::from_canonical_u64(cin);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cout + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
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
                    multiplicity[row as usize] += 1;
                }
            }
            ADD_OP | ADDW_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Add;

                // Set use last carry to zero
                row.use_last_carry = F::zero();

                // Set has initial carry
                row.has_initial_carry = F::zero();

                // Apply the logic to every byte
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
                    let result = cin + a_bytes[i] as u64 + b_bytes[i] as u64;
                    cout = result >> 8;
                    cin = if i == carry_byte { 0 } else { cout };
                    row.carry[i] = F::from_canonical_u64(cin);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cin;

                    // Set a and b bytes
                    let a_byte = if mode32 && (i >= 4) { c_bytes[3] } else { a_bytes[i] };
                    let b_byte = if mode32 && (i >= 4) { 0 } else { b_bytes[i] };

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
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
                    multiplicity[row as usize] += 1;
                }
            }
            SUB_OP | SUBW_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Sub;

                // Set use last carry to zero
                row.use_last_carry = F::zero();

                // Set has initial carry
                row.has_initial_carry = F::zero();

                // Apply the logic to every byte
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
                    cout = if a_bytes[i] as u64 >= (b_bytes[i] as u64 + cin) { 0 } else { 1 };
                    cin = if i == carry_byte { 0 } else { cout };
                    row.carry[i] = F::from_canonical_u64(cin);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cin;

                    // Set a and b bytes
                    let a_byte = if mode32 && (i >= 4) { c_bytes[3] } else { a_bytes[i] };
                    let b_byte = if mode32 && (i >= 4) { 0 } else { b_bytes[i] };

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
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
                    multiplicity[row as usize] += 1;
                }
            }
            LEU_OP | LEUW_OP | LE_OP | LEW_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                binary_basic_table_op = if (opcode == LEU_OP) || (opcode == LEUW_OP) {
                    BinaryBasicTableOp::Leu
                } else {
                    BinaryBasicTableOp::Le
                };

                // Set use last carry to one
                row.use_last_carry = F::one();

                // Set has initial carry
                row.has_initial_carry = F::zero();

                // Apply the logic to every byte
                for i in 0..8 {
                    // Calculate carry
                    let previous_cin = cin;
                    cout = 0;
                    if a_bytes[i] <= b_bytes[i] {
                        cout = 1;
                    }
                    if (binary_basic_table_op == BinaryBasicTableOp::Le) &&
                        (plast[i] == 1) &&
                        (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80)
                    {
                        cout = c;
                    }
                    cin = cout;
                    row.carry[i] = F::from_canonical_u64(cin);

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = cin + 8 * plast[i];

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
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
                    multiplicity[row as usize] += 1;
                }
            }
            AND_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::And;

                row.use_last_carry = F::zero();

                // Set has initial carry
                row.has_initial_carry = F::zero();

                // No carry
                for i in 0..8 {
                    row.carry[i] = F::zero();

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = 0;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        0,
                        plast[i],
                        flags,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            OR_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Or;

                row.use_last_carry = F::zero();

                // Set has initial carry
                row.has_initial_carry = F::zero();

                // No carry
                for i in 0..8 {
                    row.carry[i] = F::zero();

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = 0;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        0,
                        plast[i],
                        flags,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            XOR_OP => {
                // Set opcode is min or max
                row.op_is_min_max = F::zero();

                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Xor;

                // Set use last carry to zero
                row.use_last_carry = F::zero();

                // Set has initial carry
                row.has_initial_carry = F::zero();

                // No carry
                for i in 0..8 {
                    row.carry[i] = F::zero();

                    //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                    let flags = 0;

                    // Store the required in the vector
                    let row = BinaryBasicTableSM::<F>::calculate_table_row(
                        binary_basic_table_op,
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        0,
                        plast[i],
                        flags,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            _ => panic!("BinaryBasicSM::process_slice() found invalid opcode={}", opcode),
        }

        // Set cout
        let cout32 = row.carry[HALF_BYTES - 1];
        let cout64 = row.carry[BYTES - 1];
        row.cout = mode64 * (cout64 - cout32) + cout32;

        // Set result_is_a
        row.result_is_a = row.op_is_min_max * row.cout;

        // Set use_last_carry_mode32 and use_last_carry_mode64
        row.use_last_carry_mode32 = F::from_bool(mode32) * row.use_last_carry;
        row.use_last_carry_mode64 = mode64 * row.use_last_carry;

        // Set micro opcode
        row.m_op = F::from_canonical_u8(binary_basic_table_op as u8);

        // Set m_op_or_ext
        let ext_32_op = F::from_canonical_u8(BinaryBasicTableOp::Ext32 as u8);
        row.m_op_or_ext = mode64 * (row.m_op - ext_32_op) + ext_32_op;

        // Set free_in_a_or_c and free_in_b_or_zero
        for i in 0..HALF_BYTES {
            row.free_in_a_or_c[i] = mode64 *
                (row.free_in_a[i + HALF_BYTES] - row.free_in_c[HALF_BYTES - 1]) +
                row.free_in_c[HALF_BYTES - 1];
            row.free_in_b_or_zero[i] = mode64 * row.free_in_b[i + HALF_BYTES];
        }

        if row.use_last_carry == F::one() {
            // Set first and last elements
            row.free_in_c[7] = row.free_in_c[0];
            row.free_in_c[0] = F::zero();
        }

        // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
        row.multiplicity = F::one();

        // Return
        row
    }

    pub fn prove_instance(
        &self,
        operations: Vec<ZiskRequiredOperation>,
        prover_buffer: &mut [F],
    ) {
        Self::prove_internal(
            &self.wcm,
            &self.binary_basic_table_sm,
            operations,
            prover_buffer,
        );
    }

    fn prove_internal(
        wcm: &WitnessManager<F>,
        binary_basic_table_sm: &BinaryBasicTableSM<F>,
        operations: Vec<ZiskRequiredOperation>,
        prover_buffer: &mut [F],
    ) {
        timer_start_trace!(BINARY_TRACE);
        let pctx = wcm.get_pctx();
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_AIR_IDS[0]);
        let air_binary_table = pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_TABLE_AIR_IDS[0]);
        assert!(operations.len() <= air.num_rows());

        info!(
            "{}: ··· Creating Binary basic instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            operations.len(),
            air.num_rows(),
            operations.len() as f64 / air.num_rows() as f64 * 100.0
        );

        let mut multiplicity_table = vec![0u64; air_binary_table.num_rows()];
        let mut trace_buffer =
            BinaryTrace::<F>::map_buffer(prover_buffer, air.num_rows(), 0).unwrap();

        for (i, operation) in operations.iter().enumerate() {
            let row = Self::process_slice(operation, &mut multiplicity_table);
            trace_buffer[i] = row;
        }
        timer_stop_and_log_trace!(BINARY_TRACE);

        timer_start_trace!(BINARY_PADDING);
        // Note: We can choose any operation that trivially satisfies the constraints on padding
        // rows
        let padding_row = BinaryRow::<F> {
            m_op: F::from_canonical_u8(AND_OP),
            m_op_or_ext: F::from_canonical_u8(AND_OP),
            ..Default::default()
        };

        for i in operations.len()..air.num_rows() {
            trace_buffer[i] = padding_row;
        }

        let padding_size = air.num_rows() - operations.len();
        for last in 0..2 {
            let multiplicity = (7 - 6 * last as u64) * padding_size as u64;
            let row = BinaryBasicTableSM::<F>::calculate_table_row(
                BinaryBasicTableOp::And,
                0,
                0,
                0,
                last as u64,
                0,
            );
            multiplicity_table[row as usize] += multiplicity;
        }
        timer_stop_and_log_trace!(BINARY_PADDING);

        timer_start_trace!(BINARY_TABLE);
        binary_basic_table_sm.process_slice(&multiplicity_table);
        timer_stop_and_log_trace!(BINARY_TABLE);

        std::thread::spawn(move || {
            drop(operations);
            drop(multiplicity_table);
        });
    }

    pub fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            let pctx = self.wcm.get_pctx();
            let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_AIR_IDS[0]);

            while inputs.len() >= air.num_rows() || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(air.num_rows(), inputs.len());
                let drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                let binary_basic_table_sm = self.binary_basic_table_sm.clone();
                let wcm = self.wcm.clone();

                let sctx = self.wcm.get_sctx().clone();

                let trace: BinaryTrace<'_, _> = BinaryTrace::new(air.num_rows());
                let mut prover_buffer = trace.buffer.unwrap();
    
                Self::prove_internal(
                    &wcm,
                    &binary_basic_table_sm,
                    drained_inputs,
                    &mut prover_buffer,
                );

                let air_instance = AirInstance::new(
                    sctx,
                    ZISK_AIRGROUP_ID,
                    BINARY_AIR_IDS[0],
                    None,
                    prover_buffer,
                );
                wcm.get_pctx().air_instance_repo.add_air_instance(air_instance, None);
            }
        }
    }
}

impl<F: Send + Sync> WitnessComponent<F> for BinaryBasicSM<F> {}
