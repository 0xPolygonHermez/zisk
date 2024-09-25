use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{opcode_execute, ZiskRequiredBinaryExtensionTable, P2_12, P2_6, P2_9};
use zisk_pil::*;
const PROVE_CHUNK_SIZE: usize = 1 << 16;
const MULTIPLICITY_TABLE_SIZE: usize = 1 << 22;

pub struct BinaryExtensionTableSM<F> {
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredBinaryExtensionTable>>,

    // Row multiplicity table
    multiplicity: Mutex<Vec<u32>>,

    _phantom: std::marker::PhantomData<F>,
}

#[derive(Debug)]
pub enum ExtensionTableSMErr {
    InvalidOpcode,
}

impl<F: Field> BinaryExtensionTableSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let binary_extension_table = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            inputs: Mutex::new(Vec::new()),
            multiplicity: Mutex::new(vec![0; MULTIPLICITY_TABLE_SIZE]),
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
            <BinaryExtensionTableSM<F> as Provable<ZiskRequiredBinaryExtensionTable, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );

            let buffer_allocator = self.wcm.get_ectx().buffer_allocator.as_ref();
            let (buffer_size, offsets) = buffer_allocator
                .get_buffer_info(
                    self.wcm.get_sctx(),
                    BINARY_EXTENSION_AIRGROUP_ID,
                    BINARY_EXTENSION_TABLE_AIR_IDS[0],
                )
                .expect("Binary extension Table buffer not found");

            let mut buffer: Vec<F> = vec![F::zero(); buffer_size as usize];
            let mut trace_accessor = BinaryExtensionTable0Trace::map_buffer(
                &mut buffer,
                MULTIPLICITY_TABLE_SIZE,
                offsets[0] as usize,
            )
            .unwrap();

            let multiplicity = self.multiplicity.lock().unwrap();
            for i in 0..MULTIPLICITY_TABLE_SIZE {
                trace_accessor[i].multiplicity = F::from_canonical_u32(multiplicity[i]);
            }

            let air_instance = AirInstance::new(
                BINARY_EXTENSION_TABLE_AIRGROUP_ID,
                BINARY_EXTENSION_TABLE_AIR_IDS[0],
                None,
                buffer,
            );
            // self.wcm.get_pctx().air_instance_repo.add_air_instance(air_instance);
        }
    }

    pub fn operations() -> Vec<u8> {
        // TODO! Review this codes
        vec![0x0d, 0x0e, 0x0f, 0x24, 0x25, 0x26]
    }

    pub fn process_slice(&self, input: &Vec<ZiskRequiredBinaryExtensionTable>) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for i in input {
            // Calculate the different row offset contributors, according to the PIL
            let offset_a = i.a;
            let offset_b = i.b * P2_6;
            let offset_offset = i.offset * P2_9;
            let offset_operation = (i.opcode as u64 - 2) * P2_12;
            let row = offset_a + offset_b + offset_offset + offset_operation;
            assert!(row < MULTIPLICITY_TABLE_SIZE as u64);
            multiplicity[row as usize] += 1;
        }
    }
}

impl<F: Send + Sync> WitnessComponent<F> for BinaryExtensionTableSM<F> {
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

impl<F: Field> Provable<ZiskRequiredBinaryExtensionTable, OpResult> for BinaryExtensionTableSM<F> {
    fn calculate(
        &self,
        operation: ZiskRequiredBinaryExtensionTable,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = opcode_execute(operation.opcode, operation.a, operation.b);
        Ok(result)
    }

    fn prove(&self, operations: &[ZiskRequiredBinaryExtensionTable], drain: bool, _scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            while inputs.len() >= PROVE_CHUNK_SIZE || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
                let drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                self.process_slice(&drained_inputs);
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: ZiskRequiredBinaryExtensionTable,
        drain: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());

        self.prove(&[operation], drain, scope);

        result
    }
}
