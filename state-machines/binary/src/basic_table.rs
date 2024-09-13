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
use zisk_pil::BinaryTable0Row;
const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct BasicTableSM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,
}

#[derive(Debug)]
pub enum BasicTableSMErr {
    InvalidOpcode,
}

impl BasicTableSM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let basic_table =
            Self { registered_predecessors: AtomicU32::new(0), inputs: Mutex::new(Vec::new()) };
        let basic_table = Arc::new(basic_table);

        wcm.register_component(basic_table.clone(), Some(airgroup_id), Some(air_ids));

        basic_table
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <BasicTableSM as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );
        }
    }

    pub fn operations() -> Vec<u8> {
        // TODO! Review this codes
        vec![0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x20, 0x21, 0x22]
    }

    pub fn process_slice<F: AbstractField>(
        input: &Vec<ZiskRequiredOperation>,
    ) -> Result<Vec<BinaryTable0Row<F>>, BasicTableSMErr> {
        // Create the trace vector
        let mut _trace: Vec<BinaryTable0Row<F>> = Vec::new();

        for _ in input {
            // Create an empty trace
            // let mut t: BinaryTable0Row<F> = Default::default();

            // TODO!

            // TODO: Find duplicates of this trace and reuse them by increasing their multiplicity.
            // t.multiplicity = F::one();

            // Store the trace in the vector
            // trace.push(t);
        }

        // Return successfully
        Ok(_trace)
    }
}

impl<F> WitnessComponent<F> for BasicTableSM {
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

impl Provable<ZiskRequiredOperation, OpResult> for BasicTableSM {
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
                    //let trace = BasicTableSM::process_slice::<F>(&_drained_inputs);
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
