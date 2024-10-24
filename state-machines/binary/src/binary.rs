use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::{BinaryBasicSM, BinaryBasicTableSM, BinaryExtensionSM, BinaryExtensionTableSM};
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::SetupCtx;
use rayon::Scope;
use sm_common::{OpResult, Provable, ThreadController};
use zisk_core::ZiskRequiredOperation;
use zisk_pil::{
    BINARY_AIRGROUP_ID, BINARY_AIR_IDS, BINARY_EXTENSION_AIRGROUP_ID, BINARY_EXTENSION_AIR_IDS,
    BINARY_EXTENSION_TABLE_AIRGROUP_ID, BINARY_EXTENSION_TABLE_AIR_IDS, BINARY_TABLE_AIRGROUP_ID,
    BINARY_TABLE_AIR_IDS,
};

const PROVE_CHUNK_SIZE: usize = 1 << 16;

#[allow(dead_code)]
pub struct BinarySM<F: PrimeField> {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Thread controller to manage the execution of the state machines
    threads_controller: Arc<ThreadController>,

    // Inputs
    inputs_basic: Mutex<Vec<ZiskRequiredOperation>>,
    inputs_extension: Mutex<Vec<ZiskRequiredOperation>>,

    // Secondary State machines
    binary_basic_sm: Arc<BinaryBasicSM<F>>,
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,

    sctx: Arc<SetupCtx>,
}

impl<F: PrimeField> BinarySM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, std: Arc<Std<F>>, sctx: Arc<SetupCtx>) -> Arc<Self> {
        let binary_basic_table_sm = BinaryBasicTableSM::new(
            wcm.clone(),
            sctx.clone(),
            BINARY_TABLE_AIRGROUP_ID,
            BINARY_TABLE_AIR_IDS,
        );
        let binary_basic_sm = BinaryBasicSM::new(
            wcm.clone(),
            sctx.clone(),
            binary_basic_table_sm,
            BINARY_AIRGROUP_ID,
            BINARY_AIR_IDS,
        );

        let binary_extension_table_sm = BinaryExtensionTableSM::new(
            wcm.clone(),
            sctx.clone(),
            BINARY_EXTENSION_TABLE_AIRGROUP_ID,
            BINARY_EXTENSION_TABLE_AIR_IDS,
        );
        let binary_extension_sm = BinaryExtensionSM::new(
            wcm.clone(),
            std,
            sctx.clone(),
            binary_extension_table_sm,
            BINARY_EXTENSION_AIRGROUP_ID,
            BINARY_EXTENSION_AIR_IDS,
        );

        let binary_sm = Self {
            registered_predecessors: AtomicU32::new(0),
            threads_controller: Arc::new(ThreadController::new()),
            inputs_basic: Mutex::new(Vec::new()),
            inputs_extension: Mutex::new(Vec::new()),
            binary_basic_sm,
            binary_extension_sm,
            sctx,
        };
        let binary_sm = Arc::new(binary_sm);

        wcm.register_component(binary_sm.clone(), None, None);

        binary_sm.binary_basic_sm.register_predecessor();
        binary_sm.binary_extension_sm.register_predecessor();

        binary_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <BinarySM<F> as Provable<ZiskRequiredOperation, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );
            self.threads_controller.wait_for_threads();

            self.binary_basic_sm.unregister_predecessor(scope);
            self.binary_extension_sm.unregister_predecessor(scope);
        }
    }

    pub fn prove_instance(
        &self,
        operations: Vec<ZiskRequiredOperation>,
        is_extension: bool,
        prover_buffer: &mut [F],
        offset: u64,
        scope: &Scope,
    ) {
        if !is_extension {
            self.binary_basic_sm.prove_instance(operations, prover_buffer, offset, scope);
        } else {
            self.binary_extension_sm.prove_instance(operations, prover_buffer, offset, scope);
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for BinarySM<F> {}

impl<F: PrimeField> Provable<ZiskRequiredOperation, OpResult> for BinarySM<F> {
    fn prove(&self, operations: &[ZiskRequiredOperation], drain: bool, scope: &Scope) {
        let mut _inputs_basic = Vec::new();
        let mut _inputs_extension = Vec::new();

        let basic_operations = BinaryBasicSM::<F>::operations();
        let extension_operations = BinaryExtensionSM::<F>::operations();

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

            self.threads_controller.add_working_thread();
            let thread_controller = self.threads_controller.clone();

            scope.spawn(move |scope| {
                binary_basic_sm_cloned.prove(&drained_inputs_basic, false, scope);

                thread_controller.remove_working_thread();
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

            self.threads_controller.add_working_thread();
            let thread_controller = self.threads_controller.clone();

            scope.spawn(move |scope| {
                binary_extension_sm_cloned.prove(&drained_inputs_extension, false, scope);

                thread_controller.remove_working_thread();
            });
        }
        drop(inputs_extension);
    }
}
