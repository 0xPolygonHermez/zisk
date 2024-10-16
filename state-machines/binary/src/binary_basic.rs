use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use log::info;
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use rayon::{prelude::*, Scope};
use sm_common::{create_prover_buffer, OpResult, Provable, ThreadController};
use std::cmp::Ordering as CmpOrdering;
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredBinaryBasicTable, ZiskRequiredOperation};
use zisk_pil::*;

use crate::BinaryBasicTableSM;

const EXT_32_OP: u8 = 0x23;

pub struct BinaryBasicSM<F> {
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Thread controller to manage the execution of the state machines
    threads_controller: Arc<ThreadController>,

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
            threads_controller: Arc::new(ThreadController::new()),
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

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <BinaryBasicSM<F> as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );

            self.threads_controller.wait_for_threads();

            self.binary_basic_table_sm.unregister_predecessor(scope);
        }
    }

    pub fn operations() -> Vec<u8> {
        vec![
            0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x20, 0x21, 0x22,
            0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
        ]
    }

    pub fn process_slice(
        required: &Vec<ZiskRequiredOperation>,
    ) -> (Vec<Binary0Row<F>>, Vec<ZiskRequiredBinaryBasicTable>) {
        // Create the trace vector
        let mut trace: Vec<Binary0Row<F>> = Vec::with_capacity(required.len());

        // Create the table required vector
        let mut table_required: Vec<ZiskRequiredBinaryBasicTable> =
            Vec::with_capacity(required.len() * 8);

        for r in required {
            // Create an empty trace
            let mut t: Binary0Row<F> = Default::default();

            // Execute the opcode
            let c: u64;
            let flag: bool;
            (c, flag) = ZiskOp::execute(r.opcode, r.a, r.b);
            let _flag = flag;

            // Calculate result_is_a
            let result_is_a: u64 = if r.b == c { 0 } else { 1 };

            // Decompose the opcode into mode32 & op
            let mode32 = (r.opcode & 0x10) != 0;
            t.mode32 = F::from_bool(mode32);
            let m_op = r.opcode & 0xEF;
            t.m_op = F::from_canonical_u8(m_op);

            // Split a in bytes and store them in free_in_a
            let a_bytes: [u8; 8] = r.a.to_le_bytes();
            for (i, value) in a_bytes.iter().enumerate() {
                t.free_in_a[i] = F::from_canonical_u8(*value);
            }

            // Split b in bytes and store them in free_in_b
            let b_bytes: [u8; 8] = r.b.to_le_bytes();
            for (i, value) in b_bytes.iter().enumerate() {
                t.free_in_b[i] = F::from_canonical_u8(*value);
            }

            // Split c in bytes and store them in free_in_c
            let c_bytes: [u8; 8] = c.to_le_bytes();
            for (i, value) in c_bytes.iter().enumerate() {
                t.free_in_c[i] = F::from_canonical_u8(*value);
            }

            // Set main SM step
            t.main_step = F::from_canonical_u64(r.step);

            // Set use last carry and carry[], based on operation
            let mut cout: u64;
            let mut cin: u64 = 0;
            let plast: [u64; 8] =
                if mode32 { [0, 0, 0, 1, 0, 0, 0, 0] } else { [0, 0, 0, 0, 0, 0, 0, 1] };
            // Calculate the byte that sets the carry
            let carry_byte = if mode32 { 3 } else { 7 };

            match m_op {
                0x02 /* ADD, ADD_W */ => {
                    // Set use last carry to zero
                    t.use_last_carry = F::zero();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        let result = cin + a_bytes[i] as u64 + b_bytes[i] as u64;
                        debug_assert!((result & 0xff) == c_bytes[i] as u64);
                        cout = result >> 8;
                        cin = if i == carry_byte { 0 } else { cout };
                        t.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cin;

                        // Set a and b bytes
                        let a_byte = if mode32 && (i >= 4) { c_bytes[3] } else { a_bytes[i] };
                        let b_byte = if mode32 && (i >= 4) { 0 } else { b_bytes[i] };

                        // Create a table required
                        let tr = ZiskRequiredBinaryBasicTable {
                            opcode: m_op,
                            a: a_bytes[i] as u64,
                            b: b_bytes[i] as u64,
                            row: BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_byte as u64, b_byte as u64, previous_cin, plast[i], c_bytes[i] as u64, flags, i as u64),
                            multiplicity: 1,
                        };

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x03 /* SUB, SUB_W */ => {
                    // Set use last carry to zero
                    t.use_last_carry = F::zero();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        cout = if a_bytes[i] as u64 >= (b_bytes[i] as u64 + cin) { 0 } else { 1 };
                        debug_assert!((256 * cout + a_bytes[i] as u64 - cin - b_bytes[i] as u64) == c_bytes[i] as u64);
                        cin = if i == carry_byte { 0 } else { cout };
                        t.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cin;

                        // Set a and b bytes
                        let a_byte = if mode32 && (i >= 4) { c_bytes[3] } else { a_bytes[i] };
                        let b_byte = if mode32 && (i >= 4) { 0 } else { b_bytes[i] };

                        // Create a table required
                        let tr = ZiskRequiredBinaryBasicTable {
                            opcode: m_op,
                            a: a_bytes[i] as u64,
                            b: b_bytes[i] as u64,
                            row: BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_byte as u64, b_byte as u64, previous_cin, plast[i], c_bytes[i] as u64, flags, i as u64),
                            multiplicity: 1,
                        };

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x04 | 0x05 /*LTU,LTU_W,LT,LT_W*/ => {
                    // Set use last carry to one
                    t.use_last_carry = F::one();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        match a_bytes[i].cmp(&b_bytes[i]) {
                            CmpOrdering::Greater => {
                                cout = 0;
                            },
                            CmpOrdering::Less => {
                                cout = 1;
                            },
                            CmpOrdering::Equal => {
                                cout = cin;
                            },
                        }

                        // If the chunk is signed, then the result is the sign of a
                        if (m_op == 0x05) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                        }
                        cin = cout;
                        t.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cin + 8*plast[i];

                        // Create a tablerequired
                        let tr = ZiskRequiredBinaryBasicTable {
                            opcode: m_op,
                            a: a_bytes[i] as u64,
                            b: b_bytes[i] as u64,
                            row: BinaryBasicTableSM::<F>::calculate_table_row(
                                if mode32 && (i >= 4) { EXT_32_OP } else { m_op },
                                a_bytes[i] as u64,
                                b_bytes[i] as u64,
                                previous_cin,
                                plast[i],
                                if i == 7 { c_bytes[0] as u64 } else { 0 },
                                flags,
                                i as u64),
                            multiplicity: 1,
                            };

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x06 | 0x07 /* LEU, LEU_W, LE, LE_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::one();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        cout = 0;
                        if a_bytes[i] <= b_bytes[i] {
                            cout = 1;
                        }
                        if (m_op == 0x07) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            cout = c;
                        }
                        cin = cout;
                        t.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cin + 8*plast[i];

                        // Create a table required
                        let tr = ZiskRequiredBinaryBasicTable {
                            opcode: m_op,
                            a: a_bytes[i] as u64,
                            b: b_bytes[i] as u64,
                            row: BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_bytes[i] as u64, b_bytes[i] as u64, previous_cin, plast[i],
                            if i == 7 { c_bytes[0] as u64 } else { 0 },
                            flags, i as u64),
                            multiplicity: 1,
                        };

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x08 /* EQ, EQ_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::one();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        if (a_bytes[i] == b_bytes[i]) && (cin == 0) {
                            cout = 0;
                            debug_assert!(plast[i] == c_bytes[i] as u64);
                        } else {
                            cout = 1;
                            debug_assert!(0 == c_bytes[i] as u64);
                        }
                        if plast[i] == 1 {
                            cout = 1 - cout;
                        }
                        cin = cout;
                        t.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cout + 8*plast[i];

                        // Create a table required
                        let tr = ZiskRequiredBinaryBasicTable {
                            opcode: m_op,
                            a: a_bytes[i] as u64,
                            b: b_bytes[i] as u64,
                            row: BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_bytes[i] as u64, b_bytes[i] as u64, previous_cin, plast[i],
                            if i == 7 { c_bytes[0] as u64 } else { 0 },
                            flags, i as u64),
                            multiplicity: 1,
                        };

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x09 | 0x0a /* MINU, MINU_W, MIN, MIN_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::zero();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        cout = 0;
                        if a_bytes[i] <= b_bytes[i] {
                            cout = 1;
                        }

                        // If the chunk is signed, then the result is the sign of a
                        if (m_op == 0x0a) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                        }
                        if i == 7 {
                            cout = 0;
                        }
                        cin = cout;
                        t.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cout + 2 + 4*result_is_a;

                        // Create a table required
                        let tr = ZiskRequiredBinaryBasicTable {
                            opcode: m_op,
                            a: a_bytes[i] as u64,
                            b: b_bytes[i] as u64,
                            row: BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_bytes[i] as u64, b_bytes[i] as u64, previous_cin, plast[i], c_bytes[i] as u64, flags, i as u64),
                            multiplicity: 1,
                        };

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x0b | 0x0c /* MAXU, MAXU_W, MAX, MAX_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::zero();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        cout = 0;
                        if a_bytes[i] >= b_bytes[i] {
                            cout = 1;
                        }

                        // If the chunk is signed, then the result is the sign of a
                        if (m_op == 0x0c) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                        }
                        if i == 7 {
                            cout = 0;
                        }
                        cin = cout;
                        t.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cout + 2 + 4*result_is_a;

                        // Create a table required
                        let tr = ZiskRequiredBinaryBasicTable {
                            opcode: m_op,
                            a: a_bytes[i] as u64,
                            b: b_bytes[i] as u64,
                            row: BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_bytes[i] as u64, b_bytes[i] as u64, previous_cin, plast[i], c_bytes[i] as u64, flags, i as u64),
                            multiplicity: 1,
                        };

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x20 /*AND*/ => {
                    t.use_last_carry = F::zero();

                    // No carry
                    for i in 0..8 {
                        t.carry[i] = F::zero();

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = 0;

                        // Create a table required
                        let tr = ZiskRequiredBinaryBasicTable {
                            opcode: m_op,
                            a: a_bytes[i] as u64,
                            b: b_bytes[i] as u64,
                            row: BinaryBasicTableSM::<F>::calculate_table_row(m_op, a_bytes[i] as u64, b_bytes[i] as u64, 0, plast[i], c_bytes[i] as u64, flags, i as u64),
                            multiplicity: 1,
                        };

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x21 /*OR*/ => {
                    t.use_last_carry = F::zero();

                    // No carry
                    for i in 0..8 {
                        t.carry[i] = F::zero();

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = 0;

                        // Create a table required
                        let tr = ZiskRequiredBinaryBasicTable {
                            opcode: m_op,
                            a: a_bytes[i] as u64,
                            b: b_bytes[i] as u64,
                            row: BinaryBasicTableSM::<F>::calculate_table_row(m_op, a_bytes[i] as u64, b_bytes[i] as u64, 0, plast[i], c_bytes[i] as u64, flags, i as u64),
                            multiplicity: 1,
                        };

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x22 /*XOR*/ => {
                    t.use_last_carry = F::zero();

                    // No carry
                    for i in 0..8 {
                        t.carry[i] = F::zero();

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = 0;

                        // Create a table required
                        let tr = ZiskRequiredBinaryBasicTable {
                            opcode: m_op,
                            a: a_bytes[i] as u64,
                            b: b_bytes[i] as u64,
                            row: BinaryBasicTableSM::<F>::calculate_table_row(m_op, a_bytes[i] as u64, b_bytes[i] as u64, 0, plast[i], c_bytes[i] as u64, flags, i as u64),
                            multiplicity: 1,
                        };

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                _ => panic!("BinaryBasicSM::process_slice() found invalid opcode={} m_op={}", r.opcode, m_op),
            }

            if t.use_last_carry == F::one() {
                // Set first and last elements
                t.free_in_c[7] = t.free_in_c[0];
                t.free_in_c[0] = F::zero();
            }

            // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
            t.multiplicity = F::one();

            // Store the trace in the vector
            trace.push(t);
        }

        // Return
        (trace, table_required)
    }

    #[inline(always)]
    pub fn process_slice_buff(
        operation: &ZiskRequiredOperation,
        multiplicity: &mut Vec<u64>,
    ) -> Binary0Row<F> {
        // Create an empty trace
        let mut row: Binary0Row<F> = Default::default();

        // Execute the opcode
        let c: u64;
        let flag: bool;
        (c, flag) = ZiskOp::execute(operation.opcode, operation.a, operation.b);
        let _flag = flag;

        // Calculate result_is_a
        let result_is_a: u64 = if operation.b == c { 0 } else { 1 };

        // Decompose the opcode into mode32 & op
        let mode32 = (operation.opcode & 0x10) != 0;
        row.mode32 = F::from_bool(mode32);
        let m_op = operation.opcode & 0xEF;
        row.m_op = F::from_canonical_u8(m_op);

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
        let mut cin: u64 = 0;
        let plast: [u64; 8] =
            if mode32 { [0, 0, 0, 1, 0, 0, 0, 0] } else { [0, 0, 0, 0, 0, 0, 0, 1] };
        // Calculate the byte that sets the carry
        let carry_byte = if mode32 { 3 } else { 7 };

        match m_op {
                0x02 /* ADD, ADD_W */ => {
                    // Set use last carry to zero
                    row.use_last_carry = F::zero();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        let result = cin + a_bytes[i] as u64 + b_bytes[i] as u64;
                        debug_assert!((result & 0xff) == c_bytes[i] as u64);
                        cout = result >> 8;
                        cin = if i == carry_byte { 0 } else { cout };
                        row.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cin;

                        // Set a and b bytes
                        let a_byte = if mode32 && (i >= 4) { c_bytes[3] } else { a_bytes[i] };
                        let b_byte = if mode32 && (i >= 4) { 0 } else { b_bytes[i] };

                        // Store the required in the vector
                        let row = BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_byte as u64, b_byte as u64, previous_cin, plast[i], c_bytes[i] as u64, flags, i as u64);
                        multiplicity[row as usize] += 1;
                    }
                }
                0x03 /* SUB, SUB_W */ => {
                    // Set use last carry to zero
                    row.use_last_carry = F::zero();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        cout = if a_bytes[i] as u64 >= (b_bytes[i] as u64 + cin) { 0 } else { 1 };
                        debug_assert!((256 * cout + a_bytes[i] as u64 - cin - b_bytes[i] as u64) == c_bytes[i] as u64);
                        cin = if i == carry_byte { 0 } else { cout };
                        row.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cin;

                        // Set a and b bytes
                        let a_byte = if mode32 && (i >= 4) { c_bytes[3] } else { a_bytes[i] };
                        let b_byte = if mode32 && (i >= 4) { 0 } else { b_bytes[i] };

                        // Store the required in the vector
                        let row = BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_byte as u64, b_byte as u64, previous_cin, plast[i], c_bytes[i] as u64, flags, i as u64);
                        multiplicity[row as usize] += 1;
                    }
                }
                0x04 | 0x05 /*LTU,LTU_W,LT,LT_W*/ => {
                    // Set use last carry to one
                    row.use_last_carry = F::one();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        match a_bytes[i].cmp(&b_bytes[i]) {
                            CmpOrdering::Greater => {
                                cout = 0;
                            },
                            CmpOrdering::Less => {
                                cout = 1;
                            },
                            CmpOrdering::Equal => {
                                cout = cin;
                            },
                        }

                        // If the chunk is signed, then the result is the sign of a
                        if (m_op == 0x05) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                        }
                        cin = cout;
                        row.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cin + 8*plast[i];

                        // Store the required in the vector
                        let row = BinaryBasicTableSM::<F>::calculate_table_row(
                                if mode32 && (i >= 4) { EXT_32_OP } else { m_op },
                                a_bytes[i] as u64,
                                b_bytes[i] as u64,
                                previous_cin,
                                plast[i],
                                if i == 7 { c_bytes[0] as u64 } else { 0 },
                                flags,
                                i as u64);
                        multiplicity[row as usize] += 1;
                    }
                }
                0x06 | 0x07 /* LEU, LEU_W, LE, LE_W */ => {
                    // Set use last carry to one
                    row.use_last_carry = F::one();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        cout = 0;
                        if a_bytes[i] <= b_bytes[i] {
                            cout = 1;
                        }
                        if (m_op == 0x07) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            cout = c;
                        }
                        cin = cout;
                        row.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cin + 8*plast[i];

                        // Store the required in the vector
                        let row = BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_bytes[i] as u64, b_bytes[i] as u64, previous_cin, plast[i],
                            if i == 7 { c_bytes[0] as u64 } else { 0 },
                            flags, i as u64);
                        multiplicity[row as usize] += 1;
                    }
                }
                0x08 /* EQ, EQ_W */ => {
                    // Set use last carry to one
                    row.use_last_carry = F::one();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        if (a_bytes[i] == b_bytes[i]) && (cin == 0) {
                            cout = 0;
                            debug_assert!(plast[i] == c_bytes[i] as u64);
                        } else {
                            cout = 1;
                            debug_assert!(0 == c_bytes[i] as u64);
                        }
                        if plast[i] == 1 {
                            cout = 1 - cout;
                        }
                        cin = cout;
                        row.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cout + 8*plast[i];

                        // Store the required in the vector
                        let row = BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_bytes[i] as u64, b_bytes[i] as u64, previous_cin, plast[i],
                            if i == 7 { c_bytes[0] as u64 } else { 0 },
                            flags, i as u64);
                        multiplicity[row as usize] += 1;
                    }
                }
                0x09 | 0x0a /* MINU, MINU_W, MIN, MIN_W */ => {
                    // Set use last carry to one
                    row.use_last_carry = F::zero();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        cout = 0;
                        if a_bytes[i] <= b_bytes[i] {
                            cout = 1;
                        }

                        // If the chunk is signed, then the result is the sign of a
                        if (m_op == 0x0a) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                        }
                        if i == 7 {
                            cout = 0;
                        }
                        cin = cout;
                        row.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cout + 2 + 4*result_is_a;

                        // Store the required in the vector
                        let row = BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_bytes[i] as u64, b_bytes[i] as u64, previous_cin, plast[i], c_bytes[i] as u64, flags, i as u64);
                        multiplicity[row as usize] += 1;
                    }
                }
                0x0b | 0x0c /* MAXU, MAXU_W, MAX, MAX_W */ => {
                    // Set use last carry to one
                    row.use_last_carry = F::zero();

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let previous_cin = cin;
                        cout = 0;
                        if a_bytes[i] >= b_bytes[i] {
                            cout = 1;
                        }

                        // If the chunk is signed, then the result is the sign of a
                        if (m_op == 0x0c) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                        }
                        if i == 7 {
                            cout = 0;
                        }
                        cin = cout;
                        row.carry[i] = F::from_canonical_u64(cin);

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = cout + 2 + 4*result_is_a;

                        // Store the required in the vector
                        let row = BinaryBasicTableSM::<F>::calculate_table_row(if mode32 && (i >= 4) { EXT_32_OP } else { m_op }, a_bytes[i] as u64, b_bytes[i] as u64, previous_cin, plast[i], c_bytes[i] as u64, flags, i as u64);
                        multiplicity[row as usize] += 1;
                    }
                }
                0x20 /*AND*/ => {
                    row.use_last_carry = F::zero();

                    // No carry
                    for i in 0..8 {
                        row.carry[i] = F::zero();

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = 0;

                        // Store the required in the vector
                        let row = BinaryBasicTableSM::<F>::calculate_table_row(m_op, a_bytes[i] as u64, b_bytes[i] as u64, 0, plast[i], c_bytes[i] as u64, flags, i as u64);
                        multiplicity[row as usize] += 1;
                    }
                }
                0x21 /*OR*/ => {
                    row.use_last_carry = F::zero();

                    // No carry
                    for i in 0..8 {
                        row.carry[i] = F::zero();

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = 0;

                        // Store the required in the vector
                        let row = BinaryBasicTableSM::<F>::calculate_table_row(m_op, a_bytes[i] as u64, b_bytes[i] as u64, 0, plast[i], c_bytes[i] as u64, flags, i as u64);
                        multiplicity[row as usize] += 1;
                    }
                }
                0x22 /*XOR*/ => {
                    row.use_last_carry = F::zero();

                    // No carry
                    for i in 0..8 {
                        row.carry[i] = F::zero();

                        //FLAGS[i] = cout + 2*op_is_min_max + 4*result_is_a + 8*USE_CARRY[i]*plast;
                        let flags = 0;

                        // Store the required in the vector
                        let row = BinaryBasicTableSM::<F>::calculate_table_row(m_op, a_bytes[i] as u64, b_bytes[i] as u64, 0, plast[i], c_bytes[i] as u64, flags, i as u64);
                        multiplicity[row as usize] += 1;
                    }
                }
                _ => panic!("BinaryBasicSM::process_slice() found invalid opcode={} m_op={}", operation.opcode, m_op),
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
        offset: u64,
    ) {
        timer_start_trace!(BINARY_TRACE);
        let air = self.wcm.get_pctx().pilout.get_air(BINARY_AIRGROUP_ID, BINARY_AIR_IDS[0]);
        let air_binary_table =
            self.wcm.get_pctx().pilout.get_air(BINARY_TABLE_AIRGROUP_ID, BINARY_TABLE_AIR_IDS[0]);
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
            Binary0Trace::<F>::map_buffer(prover_buffer, air.num_rows(), offset as usize).unwrap();

        for (i, operation) in operations.iter().enumerate() {
            let row = Self::process_slice_buff(&operation, &mut multiplicity_table);
            trace_buffer[i] = row;
        }
        timer_stop_and_log_trace!(BINARY_TRACE);

        timer_start_trace!(BINARY_PADDING);
        let padding_row = Binary0Row::<F> {
            m_op: F::from_canonical_u8(0x20),
            multiplicity: F::zero(),
            main_step: F::zero(), /* TODO: remove, since main_step is just for
                                   * debugging */
            ..Default::default()
        };

        for i in operations.len()..air.num_rows() {
            trace_buffer[i] = padding_row;
        }

        let padding_size = air.num_rows() - operations.len();
        for last in 0..2 {
            let multiplicity = (7 - 6 * last as u64) * padding_size as u64;
            let row =
                BinaryBasicTableSM::<F>::calculate_table_row(0x20, 0, 0, 0, last as u64, 0, 0, 0);
            multiplicity_table[row as usize] += multiplicity;
        }
        timer_stop_and_log_trace!(BINARY_PADDING);

        timer_start_trace!(BINARY_TABLE);
        self.binary_basic_table_sm.process_slice_buff(&multiplicity_table);
        timer_stop_and_log_trace!(BINARY_TABLE);
    }
}

impl<F: Send + Sync> WitnessComponent<F> for BinaryBasicSM<F> {}

impl<F: Field> Provable<ZiskRequiredOperation, OpResult> for BinaryBasicSM<F> {
    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            let air = self.wcm.get_pctx().pilout.get_air(BINARY_AIRGROUP_ID, BINARY_AIR_IDS[0]);

            while inputs.len() >= air.num_rows() || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(air.num_rows(), inputs.len());
                let drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                let binary_basic_table_sm = self.binary_basic_table_sm.clone();
                let wcm = self.wcm.clone();

                self.threads_controller.add_working_thread();
                let thread_controller = self.threads_controller.clone();

                scope.spawn(move |scope| {
                    let (mut trace_row, mut table_required) = Self::process_slice(&drained_inputs);

                    let air = wcm.get_pctx().pilout.get_air(BINARY_AIRGROUP_ID, BINARY_AIR_IDS[0]);

                    info!(
                        "{}: ··· Creating Binary basic instance [{} / {} rows filled {:.2}%]",
                        Self::MY_NAME,
                        drained_inputs.len(),
                        air.num_rows(),
                        drained_inputs.len() as f64 / air.num_rows() as f64 * 100.0
                    );

                    let trace_row_len = trace_row.len();

                    if drain && (air.num_rows() > trace_row_len) {
                        let padding_row = Binary0Row::<F> {
                            m_op: F::from_canonical_u8(0x20),
                            multiplicity: F::zero(),
                            main_step: F::zero(), /* TODO: remove, since main_step is just for
                                                   * debugging */
                            ..Default::default()
                        };

                        trace_row.resize(air.num_rows(), unsafe { std::mem::zeroed() });
                        trace_row[trace_row_len..air.num_rows()]
                            .par_iter_mut()
                            .for_each(|input| *input = padding_row);

                        let padding_size = air.num_rows() - trace_row_len;
                        for last in 0..2 {
                            let multiplicity = (7 - 6 * last as u64) * padding_size as u64;
                            table_required.push(ZiskRequiredBinaryBasicTable {
                                opcode: 0,
                                a: 0,
                                b: 0,
                                row: BinaryBasicTableSM::<F>::calculate_table_row(
                                    0x20,
                                    0,
                                    0,
                                    0,
                                    last as u64,
                                    0,
                                    0,
                                    0,
                                ),
                                multiplicity,
                            });
                        }
                    }

                    binary_basic_table_sm.prove(&table_required, false, scope);

                    // Create the prover buffer
                    let (mut prover_buffer, offset) = create_prover_buffer(
                        wcm.get_ectx(),
                        wcm.get_sctx(),
                        BINARY_AIRGROUP_ID,
                        BINARY_AIR_IDS[0],
                    );

                    // Convert the Vec<Main0Row<F>> to a flat Vec<F> and copy the resulting values
                    // into the prover buffer
                    let trace_buffer =
                        Binary0Trace::<F>::map_row_vec(trace_row, true).unwrap().buffer.unwrap();
                    prover_buffer[offset as usize..offset as usize + trace_buffer.len()]
                        .par_iter_mut()
                        .zip(trace_buffer.par_iter())
                        .for_each(|(buffer_elem, main_elem)| {
                            *buffer_elem = *main_elem;
                        });

                    let air_instance = AirInstance::new(
                        BINARY_AIRGROUP_ID,
                        BINARY_AIR_IDS[0],
                        None,
                        prover_buffer,
                    );

                    wcm.get_pctx().air_instance_repo.add_air_instance(air_instance);

                    thread_controller.remove_working_thread();
                });
            }
        }
    }
}
