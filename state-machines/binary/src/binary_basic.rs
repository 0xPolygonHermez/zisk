use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use log::info;
use p3_field::AbstractField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable, ThreadController};
use std::cmp::Ordering as CmpOrdering;
use zisk_core::{opcode_execute, ZiskRequiredBinaryBasicTable, ZiskRequiredOperation};
use zisk_pil::*;

use crate::BinaryBasicTableSM;

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

impl<F: AbstractField + Copy + Send + Sync + 'static> BinaryBasicSM<F> {
    const MY_NAME: &'static str = "BinarySM";

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
        let mut trace: Vec<Binary0Row<F>> = Vec::new();

        // Create the table required vector
        let mut table_required: Vec<ZiskRequiredBinaryBasicTable> = Vec::new();

        for r in required {
            // Create an empty trace
            let mut t: Binary0Row<F> = Default::default();

            // Execute the opcode
            let c: u64;
            let flag: bool;
            (c, flag) = opcode_execute(r.opcode, r.a, r.b);
            let _flag = flag;

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

            // Set use last carry and carry[], based on operation
            let mut cout: u64;
            let mut cin: u64 = 0;
            let plast: [u64; 8] =
                if mode32 { [0, 0, 0, 1, 0, 0, 0, 0] } else { [0, 0, 0, 0, 0, 0, 0, 1] };
            match m_op {
                0x02 /* ADD, ADD_W */ => {
                    // Set use last carry to zero
                    t.use_last_carry = F::from_canonical_u64(0);

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        let r = cin + a_bytes[i] as u64 + b_bytes[i] as u64;
                        debug_assert!((r & 0xff) == c_bytes[i] as u64);
                        cout = r >> 8;
                        cin = cout;
                        t.carry[i] = F::from_canonical_u64(cin);

                        // Create an empty required
                        let mut tr: ZiskRequiredBinaryBasicTable = Default::default();

                        // Fill it
                        tr.opcode = m_op;
                        tr.a = a_bytes[i] as u64;
                        tr.b = b_bytes[i] as u64;
                        tr.cin = cin;
                        tr.last = plast[i];

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x03 /* SUB, SUB_W */ => {
                    // Set use last carry to zero
                    t.use_last_carry = F::from_canonical_u64(0);

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        cout = if (a_bytes[i] as u64 - cin) >= b_bytes[i] as u64 { 0 } else { 1 };
                        debug_assert!((256 * cout + a_bytes[i] as u64 - cin - b_bytes[i] as u64) == c_bytes[i] as u64);
                        cin = cout;
                        t.carry[i] = F::from_canonical_u64(cin);

                        // Create an empty required
                        let mut tr: ZiskRequiredBinaryBasicTable = Default::default();

                        // Fill it
                        tr.opcode = m_op;
                        tr.a = a_bytes[i] as u64;
                        tr.b = b_bytes[i] as u64;
                        tr.cin = cin;
                        tr.last = plast[i];

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x04 | 0x05 /*LTU,LTU_W,LT,LT_W*/ => {
                    // Set use last carry to one
                    t.use_last_carry = F::from_canonical_u64(1);

                    // Apply the logic to every byte
                    //cout = 0;
                    for i in 0..8 {
                        // Calculate carry
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

                        // Create an empty required
                        let mut tr: ZiskRequiredBinaryBasicTable = Default::default();

                        // Fill it
                        tr.opcode = m_op;
                        tr.a = a_bytes[i] as u64;
                        tr.b = b_bytes[i] as u64;
                        tr.cin = cin;
                        tr.last = plast[i];

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x06 | 0x07 /* LEU, LEU_W, LE, LE_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::from_canonical_u64(1);

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        cout = 0;
                        if a_bytes[i] <= b_bytes[i] {
                            cout = 1;
                        }
                        if (m_op == 0x07) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            cout = c;
                        }
                        cin = cout;
                        t.carry[i] = F::from_canonical_u64(cin);

                        // Create an empty required
                        let mut tr: ZiskRequiredBinaryBasicTable = Default::default();

                        // Fill it
                        tr.opcode = m_op;
                        tr.a = a_bytes[i] as u64;
                        tr.b = b_bytes[i] as u64;
                        tr.cin = cin;
                        tr.last = plast[i];

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x08 /* EQ, EQ_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::from_canonical_u64(1);

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
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

                        // Create an empty required
                        let mut tr: ZiskRequiredBinaryBasicTable = Default::default();

                        // Fill it
                        tr.opcode = m_op;
                        tr.a = a_bytes[i] as u64;
                        tr.b = b_bytes[i] as u64;
                        tr.cin = cin;
                        tr.last = plast[i];

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x09 | 0x0a /* MINU, MINU_W, MIN, MIN_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::from_canonical_u64(1);

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        cout = 0;
                        if a_bytes[i] <= b_bytes[i] {
                            cout = 1;
                        }
                        else {
                        }

                        // If the chunk is signed, then the result is the sign of a
                        if (m_op == 0x0a) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                        }
                        cin = cout;
                        t.carry[i] = F::from_canonical_u64(cin);

                        // Create an empty required
                        let mut tr: ZiskRequiredBinaryBasicTable = Default::default();

                        // Fill it
                        tr.opcode = m_op;
                        tr.a = a_bytes[i] as u64;
                        tr.b = b_bytes[i] as u64;
                        tr.cin = cin;
                        tr.last = plast[i];

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x0b | 0x0c /* MAXU, MAXU_W, MAX, MAX_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::from_canonical_u64(1);

                    // Apply the logic to every byte
                    for i in 0..8 {
                        // Calculate carry
                        cout = 0;
                        if a_bytes[i] >= b_bytes[i] {
                            cout = 1;
                        }
                        else {
                        }

                        // If the chunk is signed, then the result is the sign of a
                        if (m_op == 0x0c) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                        }
                        cin = cout;
                        t.carry[i] = F::from_canonical_u64(cin);

                        // Create an empty required
                        let mut tr: ZiskRequiredBinaryBasicTable = Default::default();

                        // Fill it
                        tr.opcode = m_op;
                        tr.a = a_bytes[i] as u64;
                        tr.b = b_bytes[i] as u64;
                        tr.cin = cin;
                        tr.last = plast[i];

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x20 /*AND*/ => {
                    t.use_last_carry = F::from_canonical_u64(0);

                    // No carry
                    for i in 0..8 {
                        t.carry[i] = F::from_canonical_u64(0);

                        // Create an empty required
                        let mut tr: ZiskRequiredBinaryBasicTable = Default::default();

                        // Fill it
                        tr.opcode = m_op;
                        tr.a = a_bytes[i] as u64;
                        tr.b = b_bytes[i] as u64;
                        tr.cin = 0;
                        tr.last = plast[i];

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x21 /*OR*/ => {
                    t.use_last_carry = F::from_canonical_u64(0);

                    // No carry
                    for i in 0..8 {
                        t.carry[i] = F::from_canonical_u64(0);

                        // Create an empty required
                        let mut tr: ZiskRequiredBinaryBasicTable = Default::default();

                        // Fill it
                        tr.opcode = m_op;
                        tr.a = a_bytes[i] as u64;
                        tr.b = b_bytes[i] as u64;
                        tr.cin = 0;
                        tr.last = plast[i];

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                0x22 /*XOR*/ => {
                    t.use_last_carry = F::from_canonical_u64(0);

                    // No carry
                    for i in 0..8 {
                        t.carry[i] = F::from_canonical_u64(0);

                        // Create an empty required
                        let mut tr: ZiskRequiredBinaryBasicTable = Default::default();

                        // Fill it
                        tr.opcode = m_op;
                        tr.a = a_bytes[i] as u64;
                        tr.b = b_bytes[i] as u64;
                        tr.cin = 0;
                        tr.last = plast[i];

                        // Store the required in the vector
                        table_required.push(tr);
                    }
                }
                _ => panic!("BinaryBasicSM::process_slice() found invalid opcode={} m_op={}", r.opcode, m_op),
            }

            // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
            t.multiplicity = F::one();

            // Store the trace in the vector
            trace.push(t);
        }

        // Return
        (trace, table_required)
    }
}

impl<F: Send + Sync> WitnessComponent<F> for BinaryBasicSM<F> {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
    }
}

impl<F: AbstractField + Copy + Send + Sync + 'static> Provable<ZiskRequiredOperation, OpResult>
    for BinaryBasicSM<F>
{
    fn calculate(
        &self,
        operation: ZiskRequiredOperation,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = opcode_execute(operation.opcode, operation.a, operation.b);
        Ok(result)
    }

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
                    let (trace_row, table_required) = Self::process_slice(&drained_inputs);
                    binary_basic_table_sm.prove(&table_required, false, scope);

                    info!(
                        "{}: ··· Creating Binary basic instance [{} rows]",
                        Self::MY_NAME,
                        drained_inputs.len()
                    );
                    let buffer_allocator = wcm.get_ectx().buffer_allocator.as_ref();
                    let (buffer_size, offsets) = buffer_allocator
                        .get_buffer_info(wcm.get_sctx(), BINARY_AIRGROUP_ID, BINARY_AIR_IDS[0])
                        .expect("Binary basic buffer not found");

                    let trace_row_len = trace_row.len();
                    let trace_buffer =
                        Binary0Trace::<F>::map_row_vec(trace_row, true).unwrap().buffer.unwrap();
                    let mut buffer: Vec<F> = vec![F::zero(); buffer_size as usize];

                    buffer[offsets[0] as usize..
                        offsets[0] as usize + (trace_row_len * Binary0Row::<F>::ROW_SIZE)]
                        .copy_from_slice(&trace_buffer);

                    let air_instance =
                        AirInstance::new(BINARY_AIRGROUP_ID, BINARY_AIR_IDS[0], None, buffer);

                    wcm.get_pctx().air_instance_repo.add_air_instance(air_instance);

                    thread_controller.remove_working_thread();
                });
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: ZiskRequiredOperation,
        drain: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());

        self.prove(&[operation], drain, scope);

        result
    }
}
