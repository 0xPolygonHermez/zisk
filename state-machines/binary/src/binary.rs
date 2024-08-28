use std::sync::{Arc, Mutex};

use crate::{BinaryBasicSM, BinaryExtensionSM};
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{opcode_execute, ZiskRequiredOperation};

const PROVE_CHUNK_SIZE: usize = 1 << 12;

#[allow(dead_code)]
pub struct BinarySM {
    inputs_basic: Mutex<Vec<ZiskRequiredOperation>>,
    inputs_extension: Mutex<Vec<ZiskRequiredOperation>>,
    binary_basic_sm: Arc<BinaryBasicSM>,
    binary_extension_sm: Arc<BinaryExtensionSM>,
}

impl BinarySM {
    pub fn new<F>(
        wcm: &mut WitnessManager<F>,
        binary_basic_sm: Arc<BinaryBasicSM>,
        binary_extension_sm: Arc<BinaryExtensionSM>,
    ) -> Arc<Self> {
        let binary_sm = Self {
            inputs_basic: Mutex::new(Vec::new()),
            inputs_extension: Mutex::new(Vec::new()),
            binary_basic_sm,
            binary_extension_sm,
        };
        let binary_sm = Arc::new(binary_sm);

        wcm.register_component(binary_sm.clone() as Arc<dyn WitnessComponent<F>>, None);

        binary_sm
    }
}

impl<F> WitnessComponent<F> for BinarySM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: usize,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
    }
}

impl Provable<ZiskRequiredOperation, OpResult> for BinarySM {
    fn calculate(
        &self,
        operation: ZiskRequiredOperation,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = opcode_execute(operation.opcode, operation.a, operation.b);
        Ok(result)
    }

    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, scope: &Scope) {
        let mut _inputs_basic = Vec::new();
        let mut _inputs_extension = Vec::new();

        let basic_operations = BinaryBasicSM::operations();
        let extension_operations = BinaryExtensionSM::operations();

        // TODO Split the operations into basic and extended operations in parallel
        for operation in operations {
            if basic_operations.contains(&operation.opcode) {
                _inputs_basic.push(operation.clone());
            }
            if extension_operations.contains(&operation.opcode) {
                _inputs_extension.push(operation.clone());
            } else {
                panic!("BinarySM: Operator {:x} not found", operation.opcode);
            }
        }

        let mut inputs_basic = self.inputs_basic.lock().unwrap();
        inputs_basic.extend(_inputs_basic);

        while inputs_basic.len() >= PROVE_CHUNK_SIZE || (drain && !inputs_basic.is_empty()) {
            let drained_inputs_basic = inputs_basic.drain(..PROVE_CHUNK_SIZE).collect::<Vec<_>>();
            let binary_basic_sm_cloned = self.binary_basic_sm.clone();

            scope.spawn(move |scope| {
                binary_basic_sm_cloned.prove(&drained_inputs_basic, drain, scope);
            });
        }
        drop(inputs_basic);

        let mut inputs_extension = self.inputs_extension.lock().unwrap();
        inputs_extension.extend(_inputs_extension);

        while inputs_extension.len() >= PROVE_CHUNK_SIZE || (drain && !inputs_extension.is_empty())
        {
            let drained_inputs_extension =
                inputs_extension.drain(..PROVE_CHUNK_SIZE).collect::<Vec<_>>();
            let binary_extension_sm_cloned = self.binary_extension_sm.clone();

            scope.spawn(move |scope| {
                binary_extension_sm_cloned.prove(&drained_inputs_extension, drain, scope);
            });
        }
        drop(inputs_extension);
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
