use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::AbstractField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;
use rayon::Scope;
use sm_common::{OpResult, Provable};
use std::cmp::Ordering as CmpOrdering;
use zisk_core::{opcode_execute, ZiskRequiredOperation};
use zisk_pil::Binary0Row;
const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct BinaryBasicSM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
}

#[derive(Debug)]
pub enum BinaryBasicSMErr {
    InvalidOpcode,
}

impl BinaryBasicSM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, air_ids: &[usize]) -> Arc<Self> {
        let binary_basic =
            Self { registered_predecessors: AtomicU32::new(0), inputs: Mutex::new(Vec::new()) };
        let binary_basic = Arc::new(binary_basic);

        wcm.register_component(binary_basic.clone(), Some(air_ids));

        binary_basic
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <BinaryBasicSM as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );
        }
    }

    pub fn operations() -> Vec<u8> {
        vec![
            0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x20, 0x21, 0x22,
            0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
        ]
    }

    fn is_last_carry_opcode(opcode: u8) -> bool {
        !((opcode == 0x02/* ADD */) ||
            (opcode == 0x03/* SUB */) ||
            (opcode == 0x20/* AND */) ||
            (opcode == 0x21/* OR */) ||
            (opcode == 0x22/* XOR */) ||
            (opcode == 0x12/* ADD_W */) ||
            (opcode == 0x13/* SUB_W */))
    }

    pub fn process_slice<F: AbstractField>(
        input: &Vec<ZiskRequiredOperation>,
    ) -> Result<Vec<Binary0Row<F>>, BinaryBasicSMErr> {
        // Create the trace vector
        let mut trace: Vec<Binary0Row<F>> = Vec::new();

        for i in input {
            // Create an empty trace
            let mut t: Binary0Row<F> = Default::default();

            // Execute the opcode
            let c: u64;
            let flag: bool;
            (c, flag) = opcode_execute(i.opcode, i.a, i.b);
            let _flag = flag;

            // Decompose the opcode into mode32 & op
            let mode32 = (i.opcode & 0x10) != 0;
            t.mode32 = F::from_bool(mode32);
            let op = i.opcode & 0xEF;
            t.op = F::from_canonical_u8(op);

            // Split a in bytes and store them in free_in_a
            let a_bytes: [u8; 8] = i.a.to_le_bytes();
            for (i, value) in a_bytes.iter().enumerate() {
                t.free_in_a[i] = F::from_canonical_u8(*value);
            }

            // Split b in bytes and store them in free_in_b
            let b_bytes: [u8; 8] = i.b.to_le_bytes();
            for (i, value) in b_bytes.iter().enumerate() {
                t.free_in_b[i] = F::from_canonical_u8(*value);
            }

            // Split c in bytes and store them in free_in_c
            let c_bytes: [u8; 8] = c.to_le_bytes();
            for (i, value) in c_bytes.iter().enumerate() {
                t.free_in_c[i] = F::from_canonical_u8(*value);
            }

            // Set use last carry, which depends on the opcode
            t.use_last_carry = F::from_bool(Self::is_last_carry_opcode(i.opcode));

            // Get use last carry and carry, based on operation
            let mut cout: u64;
            let mut cin: u64 = 0;
            let plast: [u64; 8] =
                if mode32 { [0, 0, 0, 1, 0, 0, 0, 0] } else { [0, 0, 0, 0, 0, 0, 0, 1] };
            match op {
                0x02 /* ADD, ADD_W */ => {
                    // Set use last carry to zero
                    t.use_last_carry = F::from_canonical_u64(0);

                    // Apply the logic to every byte
                    for i in 0..8 {
                        let r = cin + a_bytes[i] as u64 + b_bytes[i] as u64;
                        debug_assert!((r & 0xff) == c_bytes[i] as u64);
                        cout = r >> 8;
                        t.carry[i] = F::from_canonical_u64(cin);
                        cin = cout;
                    }
                    t.carry[8] = F::from_canonical_u64(cin);
                }
                0x03 /* SUB, SUB_W */ => {
                    // Set use last carry to zero
                    t.use_last_carry = F::from_canonical_u64(0);

                    // Apply the logic to every byte
                    for i in 0..8 {
                        cout = if (a_bytes[i] as u64 - cin) >= b_bytes[i] as u64 { 0 } else { 1 };
                        debug_assert!((256 * cout + a_bytes[i] as u64 - cin - b_bytes[i] as u64) == c_bytes[i] as u64);
                        t.carry[i] = F::from_canonical_u64(cin);
                        cin = cout;
                    }
                    t.carry[8] = F::from_canonical_u64(cin);
                }
                0x04 | 0x05 /*LTU,LTU_W,LT,LT_W*/ => {
                    // Set use last carry to one
                    t.use_last_carry = F::from_canonical_u64(1);

                    // Apply the logic to every byte
                    //cout = 0;
                    for i in 0..8 {
                        //let mut c: u64;
                        match a_bytes[i].cmp(&b_bytes[i]) {
                            CmpOrdering::Greater => {
                                cout = 0;
                                //c = 0;
                            },
                            CmpOrdering::Less => {
                                cout = 1;
                                //c = plast[i];
                            },
                            CmpOrdering::Equal => {
                                cout = cin;
                                //c = plast[i] * cin;
                            },
                        }

                        // If the chunk is signed, then the result is the sign of a
                        if (op == 0x05) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            //c = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                            cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                        }
                        //debug_assert!(c == c_bytes[i] as u64);
                        t.carry[i] = F::from_canonical_u64(cin);
                        cin = cout;
                    }
                    t.carry[8] = F::from_canonical_u64(cin);
                }
                0x06 | 0x07 /* LEU, LEU_W, LE, LE_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::from_canonical_u64(1);

                    // Apply the logic to every byte
                    for i in 0..8 {
                        //let mut c: u64 = 0;
                        cout = 0;
                        if a_bytes[i] <= b_bytes[i] {
                            cout = 1;
                            //c = plast[i];
                        }

                        if (op == 0x07) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            //c = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                            cout = c;
                        }
                        t.carry[i] = F::from_canonical_u64(cin);
                        cin = cout;
                    }
                    t.carry[8] = F::from_canonical_u64(cin);
                }
                0x08 /* EQ, EQ_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::from_canonical_u64(1);

                    // Apply the logic to every byte
                    for i in 0..8 {
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
                        t.carry[i] = F::from_canonical_u64(cin);
                        cin = cout;
                    }
                    t.carry[8] = F::from_canonical_u64(cin);
                }
                0x09 | 0x0a /* MINU, MINU_W, MIN, MIN_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::from_canonical_u64(1);

                    // Apply the logic to every byte
                    for i in 0..8 {
                        //let mut c: u64 = 0;
                        cout = 0;
                        if a_bytes[i] <= b_bytes[i] {
                            cout = 1;
                            //c = if plast[i] == 1 { a_bytes[i] as u64 } else { b_bytes[i] as u64 };
                        }
                        else {
                            //c = b_bytes[i] as u64;
                        }

                        // If the chunk is signed, then the result is the sign of a
                        if (op == 0x0a) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            //c = if a_bytes[i] & 0x80 != 0 { a_bytes[i] as u64 } else { b_bytes[i] as u64 };
                            cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                        }
                        //debug_assert!(c == c_bytes[i] as u64);
                        t.carry[i] = F::from_canonical_u64(cin);
                        cin = cout;
                    }
                    t.carry[8] = F::from_canonical_u64(cin);
                }
                0x0b | 0x0c /* MAXU, MAXU_W, MAX, MAX_W */ => {
                    // Set use last carry to one
                    t.use_last_carry = F::from_canonical_u64(1);

                    // Apply the logic to every byte
                    for i in 0..8 {
                        //let mut c: u64 = 0;
                        cout = 0;
                        if a_bytes[i] >= b_bytes[i] {
                            cout = 1;
                            //c = if plast[i] == 1 { a_bytes[i] as u64 } else { b_bytes[i] as u64 };
                        }
                        else {
                            //c = b_bytes[i] as u64;
                        }

                        // If the chunk is signed, then the result is the sign of a
                        if (op == 0x0c) && (plast[i] == 1) && (a_bytes[i] & 0x80) != (b_bytes[i] & 0x80) {
                            //c = if a_bytes[i] & 0x80 != 0 { b_bytes[i] as u64 } else { a_bytes[i] as u64 };
                            cout = if a_bytes[i] & 0x80 != 0 { 1 } else { 0 };
                        }
                        //debug_assert!(c == c_bytes[i] as u64);
                        t.carry[i] = F::from_canonical_u64(cin);
                        cin = cout;
                    }
                    t.carry[8] = F::from_canonical_u64(cin);
                }
                0x20 /*AND*/ => {
                    t.use_last_carry = F::from_canonical_u64(0);

                    // No carry
                    for i in 0..9 {
                        t.carry[i] = F::from_canonical_u64(0);
                    }
                }
                0x21 /*OR*/ => {
                    t.use_last_carry = F::from_canonical_u64(0);

                    // No carry
                    for i in 0..9 {
                        t.carry[i] = F::from_canonical_u64(0);
                    }
                }
                0x22 /*XOR*/ => {
                    t.use_last_carry = F::from_canonical_u64(0);

                    // No carry
                    for i in 0..9 {
                        t.carry[i] = F::from_canonical_u64(0);
                    }
                }
                _ => panic!("BinaryBasicSM::process_slice() found invalid opcode={}", i.opcode),
            }

            // TODO: Ask Xavi
            t.multiplicity = F::from_canonical_u64(0);

            // Store the trace in the vector
            trace.push(t);
        }

        // Return successfully
        Ok(trace)
    }
}

impl<F> WitnessComponent<F> for BinaryBasicSM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
    }
}

impl Provable<ZiskRequiredOperation, OpResult> for BinaryBasicSM {
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

            while inputs.len() >= PROVE_CHUNK_SIZE || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
                let _drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                scope.spawn(move |_| {
                    // TODO! Implement prove drained_inputs (a chunk of operations)
                    //let trace = BinaryBasicSM::process_slice::<F>(&_drained_inputs);
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
