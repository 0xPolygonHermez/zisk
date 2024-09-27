use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{opcode_execute, ZiskRequiredBinaryBasicTable, P2_16, P2_17, P2_18, P2_8};
use zisk_pil::*;

const MULTIPLICITY_TABLE_SIZE: usize = 1 << 22;

pub struct BinaryBasicTableSM<F> {
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredBinaryBasicTable>>,

    // Row multiplicity table
    multiplicity: Mutex<Vec<u64>>,

    _phantom: std::marker::PhantomData<F>,
}

#[derive(Debug)]
pub enum BasicTableSMErr {
    InvalidOpcode,
}

impl<F: Field> BinaryBasicTableSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let binary_basic_table = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            inputs: Mutex::new(Vec::new()),
            multiplicity: Mutex::new(vec![0; MULTIPLICITY_TABLE_SIZE]),
            _phantom: std::marker::PhantomData,
        };
        let binary_basic_table = Arc::new(binary_basic_table);
        wcm.register_component(binary_basic_table.clone(), Some(airgroup_id), Some(air_ids));

        binary_basic_table
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <BinaryBasicTableSM<F> as Provable<ZiskRequiredBinaryBasicTable, OpResult>>::prove(
                self,
                &[],
                true,
                scope,
            );

            let buffer_allocator = self.wcm.get_ectx().buffer_allocator.as_ref();
            let (buffer_size, offsets) = buffer_allocator
                .get_buffer_info(
                    self.wcm.get_sctx(),
                    BINARY_TABLE_AIRGROUP_ID,
                    BINARY_TABLE_AIR_IDS[0],
                )
                .expect("BinaryTable buffer not found");

            let mut buffer: Vec<F> = vec![F::zero(); buffer_size as usize];
            let mut trace_accessor = BinaryTable0Trace::map_buffer(
                &mut buffer,
                MULTIPLICITY_TABLE_SIZE,
                offsets[0] as usize,
            )
            .unwrap();

            let multiplicity = self.multiplicity.lock().unwrap();
            for i in 0..MULTIPLICITY_TABLE_SIZE {
                trace_accessor[i].multiplicity = F::from_canonical_u64(multiplicity[i]);
            }

            let air_instance =
                AirInstance::new(BINARY_TABLE_AIRGROUP_ID, BINARY_TABLE_AIR_IDS[0], None, buffer);
            self.wcm.get_pctx().air_instance_repo.add_air_instance(air_instance);
        }
    }

    pub fn operations() -> Vec<u8> {
        // TODO! Review this codes
        vec![0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x20, 0x21, 0x22]
    }

    pub fn process_slice(&self, input: &Vec<ZiskRequiredBinaryBasicTable>) {
        // Create the trace vector
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for i in input {
            // Calculate the different row offset contributors, according to the PIL
            let offset_a: u64;
            let offset_b: u64;
            let offset_cin: u64;
            let offset_last: u64;
            let offset_operation: u64;
            if i.opcode <= 12 {
                offset_a = i.a;
                offset_b = i.b * P2_8;
                offset_cin = i.cin * P2_16;
                offset_last = i.last * P2_17;
                offset_operation = (i.opcode as u64 - 2) * P2_18;
            } else {
                offset_a = i.a;
                offset_b = i.b * P2_8;
                offset_cin = 0;
                offset_last = i.last * P2_16;
                offset_operation = (11 * P2_18) + (i.opcode as u64 - 32) * P2_17;
            }
            let row = offset_a + offset_b + offset_cin + offset_last + offset_operation;
            assert!(row < MULTIPLICITY_TABLE_SIZE as u64);
            multiplicity[row as usize] += 1;
        }
    }
}

impl<F: Send + Sync> WitnessComponent<F> for BinaryBasicTableSM<F> {
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

impl<F: Field> Provable<ZiskRequiredBinaryBasicTable, OpResult> for BinaryBasicTableSM<F> {
    fn calculate(
        &self,
        operation: ZiskRequiredBinaryBasicTable,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = opcode_execute(operation.opcode, operation.a, operation.b);
        Ok(result)
    }

    fn prove(&self, operations: &[ZiskRequiredBinaryBasicTable], drain: bool, _scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            let air = self
                .wcm
                .get_pctx()
                .pilout
                .get_air(BINARY_TABLE_AIRGROUP_ID, BINARY_TABLE_AIR_IDS[0]);
            let num_rows = air.num_rows();

            while inputs.len() >= num_rows || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(num_rows, inputs.len());
                let drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                self.process_slice(&drained_inputs);
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: ZiskRequiredBinaryBasicTable,
        drain: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());

        self.prove(&[operation], drain, scope);
        result
    }
}
