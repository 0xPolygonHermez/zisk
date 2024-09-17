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
use zisk_pil::BinaryExtensionTable0Row;
const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct BinaryExtensionTableSM<F> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredOperation>>,

    _phantom: std::marker::PhantomData<F>,
}

#[derive(Debug)]
pub enum ExtensionTableSMErr {
    InvalidOpcode,
}

impl<F: AbstractField + 'static> BinaryExtensionTableSM<F> {
    pub fn new(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let binary_extension_table = Self {
            registered_predecessors: AtomicU32::new(0),
            inputs: Mutex::new(Vec::new()),
            _phantom: std::marker::PhantomData,
        };
        let binary_extension_table = Arc::new(binary_extension_table);

        wcm.register_component(binary_extension_table.clone(), Some(airgroup_id), Some(air_ids));

        binary_extension_table
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <BinaryExtensionTableSM<F> as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );
        }
    }

    pub fn operations() -> Vec<u8> {
        // TODO! Review this codes
        vec![0x0d, 0x0e, 0x0f, 0x24, 0x25, 0x26]
    }

    pub fn process_slice(
        input: &Vec<ZiskRequiredOperation>,
    ) -> Result<Vec<BinaryExtensionTable0Row<F>>, ExtensionTableSMErr> {
        // Create the trace vector
        let mut _trace: Vec<BinaryExtensionTable0Row<F>> = Vec::new();

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

impl<F> WitnessComponent<F> for BinaryExtensionTableSM<F> {
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

impl<F: AbstractField> Provable<ZiskRequiredOperation, OpResult> for BinaryExtensionTableSM<F> {
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
                    //let trace = ExtensionTableSM::process_slice::<F>(&_drained_inputs);
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
