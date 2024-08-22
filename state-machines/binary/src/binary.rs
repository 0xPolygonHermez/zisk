use std::sync::{Arc, Mutex};

use crate::{BinaryBasicSM, BinaryExtensionSM};
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{opcode_execute, ZiskRequiredOperation};

const PROVE_CHUNK_SIZE: usize = 1 << 3;

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

    fn prove(&self, operations: &[ZiskRequiredOperation], is_last: bool, scope: &Scope) {
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
                panic!("Value not found in either vec1 or vec2!");
            }
        }

        let mut inputs_basic = self.inputs_basic.lock().unwrap();
        let mut inputs_extension = self.inputs_extension.lock().unwrap();

        inputs_basic.extend(_inputs_basic);
        inputs_extension.extend(_inputs_extension);

        // The following is a way to release the lock on the inputs_basic and inputs_extension
        // Mutexes asap NOTE: The `inputs_basic` lock is released when it goes out of scope
        // because it is shadowed
        let inputs_basic = if is_last || inputs_basic.len() >= PROVE_CHUNK_SIZE {
            let _inputs_basic = std::mem::take(&mut *inputs_basic);
            if _inputs_basic.is_empty() {
                None
            } else {
                Some(_inputs_basic)
            }
        } else {
            None
        };

        // NOTE: The `inputs_extension` lock is released when it goes out of scope because it is
        // shadowed
        let inputs_extension = if is_last || inputs_extension.len() >= PROVE_CHUNK_SIZE {
            let _inputs_extension = std::mem::take(&mut *inputs_extension);
            if _inputs_extension.is_empty() {
                None
            } else {
                Some(_inputs_extension)
            }
        } else {
            None
        };

        if inputs_basic.is_some() {
            let binary_basic_sm = self.binary_basic_sm.clone();
            scope.spawn(move |scope| {
                binary_basic_sm.prove(&inputs_basic.unwrap(), is_last, scope);
            });
        }

        if inputs_extension.is_some() {
            let binary_extension_sm = self.binary_extension_sm.clone();
            scope.spawn(move |scope| {
                binary_extension_sm.prove(&inputs_extension.unwrap(), is_last, scope);
            });
        }
    }

    fn calculate_prove(
        &self,
        operation: ZiskRequiredOperation,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());
        self.prove(&[operation], is_last, scope);
        result
    }
}
