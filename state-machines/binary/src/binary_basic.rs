use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::AbstractField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
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
    pub fn new<F>(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let binary_basic =
            Self { registered_predecessors: AtomicU32::new(0), inputs: Mutex::new(Vec::new()) };
        let binary_basic = Arc::new(binary_basic);

        wcm.register_component(binary_basic.clone(), Some(airgroup_id), Some(air_ids));

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

    pub fn process_slice<F: AbstractField>(
        input: &Vec<ZiskRequiredOperation>,
    ) -> Result<Vec<Binary0Row<F>>, BinaryBasicSMErr> {
        // Create the trace vector
        let mut trace: Vec<Binary0Row<F>> = Vec::new();

        for i in input {
            // Create an empty trace
            let mut t: Binary0Row<F> = Default::default();
            let a = i.a;
            let b = i.b;
            let c: u64;
            let flag: bool;

            match i.opcode {
            0x02 /*ADD*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x02);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x03 /*SUB*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x03);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x04 /*LTU*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x04);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x05 /*LT*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x05);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x06 /*LEU*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x06);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x07 /*LE*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x07);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x08 /*EQ*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x08);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x09 /*MINU*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x09);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x0a /*MIN*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x0a);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x0b /*MAXU*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x0b);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x0c /*MAX*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x0c);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x20 /*AND*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x20);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x21 /*OR*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x21);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x22 /*XOR*/ => {
                t.mode32 = F::from_canonical_u64(0);
                t.op = F::from_canonical_u64(0x22);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x12 /*ADD_W*/ => {
                t.mode32 = F::from_canonical_u64(1);
                t.op = F::from_canonical_u64(0x02);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x13 /*SUB_W*/ => {
                t.mode32 = F::from_canonical_u64(1);
                t.op = F::from_canonical_u64(0x03);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x14 /*LTU_W*/ => {
                t.mode32 = F::from_canonical_u64(1);
                t.op = F::from_canonical_u64(0x04);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x15 /*LT_W*/ => {
                t.mode32 = F::from_canonical_u64(1);
                t.op = F::from_canonical_u64(0x05);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x16 /*LEU_W*/ => {
                t.mode32 = F::from_canonical_u64(1);
                t.op = F::from_canonical_u64(0x06);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x17 /*LE_W*/ => {
                t.mode32 = F::from_canonical_u64(1);
                t.op = F::from_canonical_u64(0x07);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x18 /*EQ_W*/ => {
                t.mode32 = F::from_canonical_u64(1);
                t.op = F::from_canonical_u64(0x08);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x19 /*MINU_W*/ => {
                t.mode32 = F::from_canonical_u64(1);
                t.op = F::from_canonical_u64(0x09);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x1a /*MIN_W*/ => {
                t.mode32 = F::from_canonical_u64(1);
                t.op = F::from_canonical_u64(0x0a);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x1b /*MAXU_W*/ => {
                t.mode32 = F::from_canonical_u64(1);
                t.op = F::from_canonical_u64(0x0b);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            0x1c /*MAX_W*/ => {
                t.mode32 = F::from_canonical_u64(1);
                t.op = F::from_canonical_u64(0x0c);
                (c, flag) = opcode_execute(i.opcode, a, b);
            }
            _ => panic!("BinaryBasicSM::process_slice() found invalid opcode={}", i.opcode),
        }

            let _flag = flag;

            // Split a in bytes and store them in free_in_a
            let a_bytes: [u8; 8] = a.to_le_bytes();
            t.free_in_a[0] = F::from_canonical_u64(a_bytes[0].into());
            t.free_in_a[1] = F::from_canonical_u64(a_bytes[1].into());
            t.free_in_a[2] = F::from_canonical_u64(a_bytes[2].into());
            t.free_in_a[3] = F::from_canonical_u64(a_bytes[3].into());
            t.free_in_a[4] = F::from_canonical_u64(a_bytes[4].into());
            t.free_in_a[5] = F::from_canonical_u64(a_bytes[5].into());
            t.free_in_a[6] = F::from_canonical_u64(a_bytes[6].into());
            t.free_in_a[7] = F::from_canonical_u64(a_bytes[7].into());

            // Split b in bytes and store them in free_in_b
            let b_bytes: [u8; 8] = b.to_le_bytes();
            t.free_in_b[0] = F::from_canonical_u64(b_bytes[0].into());
            t.free_in_b[1] = F::from_canonical_u64(b_bytes[1].into());
            t.free_in_b[2] = F::from_canonical_u64(b_bytes[2].into());
            t.free_in_b[3] = F::from_canonical_u64(b_bytes[3].into());
            t.free_in_b[4] = F::from_canonical_u64(b_bytes[4].into());
            t.free_in_b[5] = F::from_canonical_u64(b_bytes[5].into());
            t.free_in_b[6] = F::from_canonical_u64(b_bytes[6].into());
            t.free_in_b[7] = F::from_canonical_u64(b_bytes[7].into());

            // Split c in bytes and store them in free_in_c
            let c_bytes: [u8; 8] = c.to_le_bytes();
            t.free_in_c[0] = F::from_canonical_u64(c_bytes[0].into());
            t.free_in_c[1] = F::from_canonical_u64(c_bytes[1].into());
            t.free_in_c[2] = F::from_canonical_u64(c_bytes[2].into());
            t.free_in_c[3] = F::from_canonical_u64(c_bytes[3].into());
            t.free_in_c[4] = F::from_canonical_u64(c_bytes[4].into());
            t.free_in_c[5] = F::from_canonical_u64(c_bytes[5].into());
            t.free_in_c[6] = F::from_canonical_u64(c_bytes[6].into());
            t.free_in_c[7] = F::from_canonical_u64(c_bytes[7].into());

            t.carry[0] = F::from_canonical_u64(0); // 9
            t.use_last_carry = F::from_canonical_u64(0);
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
