use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::AbstractField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredOperation};
use zisk_pil::BinaryExtension0Row;

const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct BinaryExtensionSM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
}

#[derive(Debug)]
pub enum BinaryExtensionSMErr {
    InvalidOpcode,
}

impl BinaryExtensionSM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let binary_extension_sm =
            Self { registered_predecessors: AtomicU32::new(0), inputs: Mutex::new(Vec::new()) };
        let binary_extension_sm = Arc::new(binary_extension_sm);

        wcm.register_component(binary_extension_sm.clone(), Some(airgroup_id), Some(air_ids));

        binary_extension_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <BinaryExtensionSM as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );
        }
    }

    pub fn operations() -> Vec<u8> {
        vec![0x0d, 0x0e, 0x0f, 0x1d, 0x1e, 0x1f, 0x24, 0x25, 0x26]
    }

    pub fn process_slice<F: AbstractField>(
        input: &Vec<ZiskRequiredOperation>,
    ) -> Result<Vec<BinaryExtension0Row<F>>, BinaryExtensionSMErr> {
        // Create the trace vector
        let mut trace: Vec<BinaryExtension0Row<F>> = Vec::new();

        for i in input {
            // Create an empty trace
            let mut t: BinaryExtension0Row<F> = BinaryExtension0Row::<F> {
                m_op: F::from_canonical_u8(i.opcode),
                ..Default::default()
            };

            // Execute the opcode
            let c: u64;
            let flag: bool;
            (c, flag) = ZiskOp::execute(i.opcode, i.a, i.b);
            let _flag = flag;

            // Decompose the opcode into mode32 & op
            let mode32 = (i.opcode & 0x10) != 0;
            t.mode32 = F::from_bool(mode32);
            let op = i.opcode & 0xEF;
            t.m_op = F::from_canonical_u8(op);
            let mode16 = i.opcode == 0x25;
            t.mode16 = F::from_bool(mode16);
            let mode8 = i.opcode == 0x24;
            t.mode8 = F::from_bool(mode8);

            // Split a in bytes and store them in in1
            let a_bytes: [u8; 8] = i.a.to_le_bytes();
            for (i, value) in a_bytes.iter().enumerate() {
                t.in1[i] = F::from_canonical_u8(*value);
            }

            // Store b low part into in2_low
            t.in2_low = F::from_canonical_u64(i.b & if mode32 { 0x1F } else { 0x3F });

            // Store b high part into free_in2
            t.free_in2[0] = F::from_canonical_u64(
                (i.b >> if mode32 { 5 } else { 6 }) & if mode32 { 0x7FF } else { 0x3FF },
            );
            t.free_in2[1] = F::from_canonical_u64((i.b >> 16) & 0xFFFF);
            t.free_in2[2] = F::from_canonical_u64((i.b >> 32) & 0xFFFF);
            t.free_in2[3] = F::from_canonical_u64(i.b >> 48);

            // Store c
            let c_bytes: [u8; 8] = c.to_le_bytes();
            for (i, value) in c_bytes.iter().enumerate() {
                t.out[i] = F::from_canonical_u8(*value);
            }

            /*
            match i.opcode {
                0x0d /* SLL */ => {

                },

                0x0e /* SRL */ => {

                },

                0x0f /* SRA */ => {

                },

                0x1d /* SLL_W */ => {

                },

                0x1e /* SRL_W */ => {

                },

                0x1f /* SRA_W */ => {

                },

                0x24 /* SE_B */ => {

                },

                0x25 /* SE_H */ => {

                },

                0x26 /* SE_W */ => {

                },
                _ => panic!("BinaryExtensionSM::process_slice() found invalid opcode={}", i.opcode),
            }*/

            // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
            t.multiplicity = F::one();

            // Store the trace in the vector
            trace.push(t);
        }

        // Return successfully
        Ok(trace)
    }
}

impl<F> WitnessComponent<F> for BinaryExtensionSM {
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

impl Provable<ZiskRequiredOperation, OpResult> for BinaryExtensionSM {
    fn calculate(
        &self,
        operation: ZiskRequiredOperation,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = ZiskOp::execute(operation.opcode, operation.a, operation.b);
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
