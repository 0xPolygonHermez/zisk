use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use log::info;
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use rayon::Scope;
use sm_common::{create_prover_buffer, OpResult, Provable};
use std::cmp::Ordering as CmpOrdering;
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredOperation};
use zisk_pil::*;

use crate::{BinaryBasicTableOp, BinaryBasicTableSM};

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
            /*<BinaryBasicSM<F> as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );*/

            self.binary_basic_table_sm.unregister_predecessor();
        }
    }

    pub fn operations() -> Vec<u8> {
        vec![
            // 64 bits opcodes
            ZiskOp::Add.code(),
            ZiskOp::Sub.code(),
            ZiskOp::Ltu.code(),
            ZiskOp::Lt.code(),
            ZiskOp::Leu.code(),
            ZiskOp::Le.code(),
            ZiskOp::Eq.code(),
            ZiskOp::Minu.code(),
            ZiskOp::Min.code(),
            ZiskOp::Maxu.code(),
            ZiskOp::Max.code(),
            ZiskOp::And.code(),
            ZiskOp::Or.code(),
            ZiskOp::Xor.code(),
            // 32 bits opcodes
            ZiskOp::AddW.code(),
            ZiskOp::SubW.code(),
            ZiskOp::LtuW.code(),
            ZiskOp::LtW.code(),
            ZiskOp::LeuW.code(),
            ZiskOp::LeW.code(),
            ZiskOp::EqW.code(),
            ZiskOp::MinuW.code(),
            ZiskOp::MinW.code(),
            ZiskOp::MaxuW.code(),
            ZiskOp::MaxW.code(),
        ]
    }

    fn opcode_is_32_bits(opcode: ZiskOp) -> bool {
        match opcode {
            ZiskOp::Add |
            ZiskOp::Sub |
            ZiskOp::Ltu |
            ZiskOp::Lt |
            ZiskOp::Leu |
            ZiskOp::Le |
            ZiskOp::Eq |
            ZiskOp::Minu |
            ZiskOp::Min |
            ZiskOp::Maxu |
            ZiskOp::Max |
            ZiskOp::And |
            ZiskOp::Or |
            ZiskOp::Xor => false,

            ZiskOp::AddW |
            ZiskOp::SubW |
            ZiskOp::LtuW |
            ZiskOp::LtW |
            ZiskOp::LeuW |
            ZiskOp::LeW |
            ZiskOp::EqW |
            ZiskOp::MinuW |
            ZiskOp::MinW |
            ZiskOp::MaxuW |
            ZiskOp::MaxW => true,

            _ => panic!("Binary basic opcode_is_32_bits() got invalid opcode={:?}", opcode),
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
        let c: u64;
        let flag: bool;
        (c, flag) = ZiskOp::execute(operation.opcode, operation.a, operation.b);
        let _flag = flag;

        // Calculate result_is_a
        let result_is_a: u64 = if operation.b == c { 0 } else { 1 };

        // Set mode32
        let opcode = ZiskOp::try_from_code(operation.opcode).expect("Invalid ZiskOp opcode");
        let mode32 = Self::opcode_is_32_bits(opcode);
        row.mode32 = F::from_bool(mode32);

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

        let binary_basic_table_op: BinaryBasicTableOp;
        let op = ZiskOp::try_from_code(operation.opcode).unwrap();
        match op {
            ZiskOp::Add | ZiskOp::AddW => {
                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Add;

                // Set use last carry to zero
                row.use_last_carry = F::zero();

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
                        c_bytes[i] as u64,
                        flags,
                        i as u64,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            ZiskOp::Sub | ZiskOp::SubW => {
                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Sub;

                // Set use last carry to zero
                row.use_last_carry = F::zero();

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
                        c_bytes[i] as u64,
                        flags,
                        i as u64,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            ZiskOp::Ltu | ZiskOp::LtuW | ZiskOp::Lt | ZiskOp::LtW => {
                // Set the binary basic table opcode
                binary_basic_table_op = if (op == ZiskOp::Ltu) || (op == ZiskOp::LtuW) {
                    BinaryBasicTableOp::Ltu
                } else {
                    BinaryBasicTableOp::Lt
                };

                // Set use last carry to one
                row.use_last_carry = F::one();

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
                        if i == 7 { c_bytes[0] as u64 } else { 0 },
                        flags,
                        i as u64,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            ZiskOp::Leu | ZiskOp::LeuW | ZiskOp::Le | ZiskOp::LeW => {
                // Set the binary basic table opcode
                binary_basic_table_op = if (op == ZiskOp::Leu) || (op == ZiskOp::LeuW) {
                    BinaryBasicTableOp::Leu
                } else {
                    BinaryBasicTableOp::Le
                };

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
                        if i == 7 { c_bytes[0] as u64 } else { 0 },
                        flags,
                        i as u64,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            ZiskOp::Eq | ZiskOp::EqW => {
                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Eq;

                // Set use last carry to one
                row.use_last_carry = F::one();

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
                        if i == 7 { c_bytes[0] as u64 } else { 0 },
                        flags,
                        i as u64,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            ZiskOp::Minu | ZiskOp::MinuW | ZiskOp::Min | ZiskOp::MinW => {
                // Set the binary basic table opcode
                binary_basic_table_op = if (op == ZiskOp::Minu) || (op == ZiskOp::MinuW) {
                    BinaryBasicTableOp::Minu
                } else {
                    BinaryBasicTableOp::Min
                };

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
                    if (binary_basic_table_op == BinaryBasicTableOp::Min) &&
                        (plast[i] == 1) &&
                        (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80)
                    {
                        cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                    }
                    if i == 7 {
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
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        c_bytes[i] as u64,
                        flags,
                        i as u64,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            ZiskOp::Maxu | ZiskOp::MaxuW | ZiskOp::Max | ZiskOp::MaxW => {
                // Set the binary basic table opcode
                binary_basic_table_op = if (op == ZiskOp::Maxu) || (op == ZiskOp::MaxuW) {
                    BinaryBasicTableOp::Maxu
                } else {
                    BinaryBasicTableOp::Max
                };

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
                    if (binary_basic_table_op == BinaryBasicTableOp::Max) &&
                        (plast[i] == 1) &&
                        (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80)
                    {
                        cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                    }
                    if i == 7 {
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
                        a_bytes[i] as u64,
                        b_bytes[i] as u64,
                        previous_cin,
                        plast[i],
                        c_bytes[i] as u64,
                        flags,
                        i as u64,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            ZiskOp::And => {
                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::And;

                row.use_last_carry = F::zero();

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
                        c_bytes[i] as u64,
                        flags,
                        i as u64,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            ZiskOp::Or => {
                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Or;

                row.use_last_carry = F::zero();

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
                        c_bytes[i] as u64,
                        flags,
                        i as u64,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            ZiskOp::Xor => {
                // Set the binary basic table opcode
                binary_basic_table_op = BinaryBasicTableOp::Xor;

                row.use_last_carry = F::zero();

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
                        c_bytes[i] as u64,
                        flags,
                        i as u64,
                    );
                    multiplicity[row as usize] += 1;
                }
            }
            _ => panic!("BinaryBasicSM::process_slice() found invalid opcode={}", operation.opcode),
        }

        if row.use_last_carry == F::one() {
            // Set first and last elements
            row.free_in_c[7] = row.free_in_c[0];
            row.free_in_c[0] = F::zero();
        }

        // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
        row.multiplicity = F::one();

        // Set micro opcode
        row.m_op = F::from_canonical_u8(binary_basic_table_op as u8);

        // Return
        row
    }

    pub fn prove_instance(
        &self,
        operations: Vec<ZiskRequiredOperation>,
        prover_buffer: &mut [F],
        offset: u64,
    ) {
        Self::prove_internal(
            &self.wcm,
            &self.binary_basic_table_sm,
            operations,
            prover_buffer,
            offset,
        );
    }

    fn prove_internal(
        wcm: &WitnessManager<F>,
        binary_basic_table_sm: &BinaryBasicTableSM<F>,
        operations: Vec<ZiskRequiredOperation>,
        prover_buffer: &mut [F],
        offset: u64,
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
            BinaryTrace::<F>::map_buffer(prover_buffer, air.num_rows(), offset as usize).unwrap();

        for (i, operation) in operations.iter().enumerate() {
            let row = Self::process_slice(operation, &mut multiplicity_table);
            trace_buffer[i] = row;
        }
        timer_stop_and_log_trace!(BINARY_TRACE);

        timer_start_trace!(BINARY_PADDING);
        let padding_row = BinaryRow::<F> {
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
            let row = BinaryBasicTableSM::<F>::calculate_table_row(
                BinaryBasicTableOp::And,
                0,
                0,
                0,
                last as u64,
                0,
                0,
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
}

impl<F: Send + Sync> WitnessComponent<F> for BinaryBasicSM<F> {}

impl<F: Field> Provable<ZiskRequiredOperation, OpResult> for BinaryBasicSM<F> {
    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, _scope: &Scope) {
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

                let (mut prover_buffer, offset) = create_prover_buffer(
                    &wcm.get_ectx(),
                    &wcm.get_sctx(),
                    ZISK_AIRGROUP_ID,
                    BINARY_AIR_IDS[0],
                );

                Self::prove_internal(
                    &wcm,
                    &binary_basic_table_sm,
                    drained_inputs,
                    &mut prover_buffer,
                    offset,
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
