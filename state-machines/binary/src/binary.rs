use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Condvar, Mutex,
};

use crate::{BinaryBasicSM, BinaryExtensionSM};
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{opcode_execute, ZiskRequiredOperation};

const PROVE_CHUNK_SIZE: usize = 1 << 12;

#[allow(dead_code)]
pub struct BinarySM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Mechanism to control the number of working threads
    working_threads: Arc<AtomicU32>,
    mutex: Arc<Mutex<()>>,
    condvar: Arc<Condvar>,

    // Inputs
    inputs_basic: Mutex<Vec<ZiskRequiredOperation>>,
    inputs_extension: Mutex<Vec<ZiskRequiredOperation>>,

    // Secondary State machines
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
            registered_predecessors: AtomicU32::new(0),
            working_threads: Arc::new(AtomicU32::new(0)),
            mutex: Arc::new(Mutex::new(())),
            condvar: Arc::new(Condvar::new()),
            inputs_basic: Mutex::new(Vec::new()),
            inputs_extension: Mutex::new(Vec::new()),
            binary_basic_sm,
            binary_extension_sm,
        };
        let binary_sm = Arc::new(binary_sm);

        wcm.register_component(binary_sm.clone(), None);

        binary_sm.binary_basic_sm.register_predecessor();
        binary_sm.binary_extension_sm.register_predecessor();

        binary_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <BinarySM as Provable<ZiskRequiredOperation, OpResult>>::prove(self, &[], true, scope);

            let mut guard = self.mutex.lock().unwrap();
            while self.working_threads.load(Ordering::SeqCst) > 0 {
                guard = self.condvar.wait(guard).unwrap();
            }

            self.binary_basic_sm.unregister_predecessor(scope);
            self.binary_extension_sm.unregister_predecessor(scope);
        }
    }
}

impl<F> WitnessComponent<F> for BinarySM {
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
            } else if extension_operations.contains(&operation.opcode) {
                _inputs_extension.push(operation.clone());
            } else {
                panic!("BinarySM: Operator {:#04x} not found", operation.opcode);
            }
        }

        let mut inputs_basic = self.inputs_basic.lock().unwrap();
        inputs_basic.extend(_inputs_basic);

        while inputs_basic.len() >= PROVE_CHUNK_SIZE || (drain && !inputs_basic.is_empty()) {
            let num_drained_basic = std::cmp::min(PROVE_CHUNK_SIZE, inputs_basic.len());
            let drained_inputs_basic = inputs_basic.drain(..num_drained_basic).collect::<Vec<_>>();
            let binary_basic_sm_cloned = self.binary_basic_sm.clone();

            self.working_threads.fetch_add(1, Ordering::SeqCst);
            let mutex = self.mutex.clone();
            let condvar = self.condvar.clone();
            let working_threads = self.working_threads.clone();

            scope.spawn(move |scope| {
                binary_basic_sm_cloned.prove(&drained_inputs_basic, drain, scope);

                let _guard = mutex.lock().unwrap();
                working_threads.fetch_sub(1, Ordering::SeqCst);
                condvar.notify_all();
            });
        }
        drop(inputs_basic);

        let mut inputs_extension = self.inputs_extension.lock().unwrap();
        inputs_extension.extend(_inputs_extension);

        while inputs_extension.len() >= PROVE_CHUNK_SIZE || (drain && !inputs_extension.is_empty())
        {
            let num_drained_extension = std::cmp::min(PROVE_CHUNK_SIZE, inputs_extension.len());
            let drained_inputs_extension =
                inputs_extension.drain(..num_drained_extension).collect::<Vec<_>>();
            let binary_extension_sm_cloned = self.binary_extension_sm.clone();

            self.working_threads.fetch_add(1, Ordering::SeqCst);
            let mutex = self.mutex.clone();
            let condvar = self.condvar.clone();
            let working_threads = self.working_threads.clone();

            scope.spawn(move |scope| {
                binary_extension_sm_cloned.prove(&drained_inputs_extension, drain, scope);

                let _guard = mutex.lock().unwrap();
                working_threads.fetch_sub(1, Ordering::SeqCst);
                condvar.notify_all();
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
