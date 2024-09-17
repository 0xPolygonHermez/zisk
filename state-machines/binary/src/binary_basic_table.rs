use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::AbstractField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{OpResult, Provable};
use zisk_core::{opcode_execute, ZiskRequiredBinaryBasedTable, P2_16, P2_17, P2_18, P2_8};
use zisk_pil::BinaryTable0Row;
const PROVE_CHUNK_SIZE: usize = 1 << 12;
const MULTIPLICITY_TABLE_SIZE: usize = 1 << 22;

pub struct BinaryBasicTableSM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredBinaryBasedTable>>,

    // Row multiplicity table
    multiplicity: Mutex<[u32; MULTIPLICITY_TABLE_SIZE as usize]>,
}

#[derive(Debug)]
pub enum BasicTableSMErr {
    InvalidOpcode,
}

impl BinaryBasicTableSM {
    pub fn new<F>(wcm: &mut WitnessManager<F>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let binary_basic_table = Self {
            registered_predecessors: AtomicU32::new(0),
            inputs: Mutex::new(Vec::new()),
            multiplicity: Mutex::new([0; MULTIPLICITY_TABLE_SIZE]),
        };
        let binary_basic_table = Arc::new(binary_basic_table);

        wcm.register_component(binary_basic_table.clone(), Some(airgroup_id), Some(air_ids));

        binary_basic_table
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor<F: AbstractField>(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <BinaryBasicTableSM as Provable<ZiskRequiredBinaryBasedTable, OpResult, F>>::prove(
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
        &self,
        input: &Vec<ZiskRequiredBinaryBasedTable>,
    ) -> Vec<BinaryTable0Row<F>> {
        // Create the trace vector
        let mut trace: Vec<BinaryTable0Row<F>> = Vec::new();

        let mut multiplicity = self.multiplicity.lock().unwrap();

        for i in input {
            // Calculate the different row offset contributors, according to the PIL
            let offset_operation: u64;
            let offset_a: u64;
            let offset_b: u64;
            let offset_cin: u64;
            let offset_last: u64;
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

            // Create an empty trace
            let mut t: BinaryTable0Row<F> = Default::default();

            // Find duplicates of this trace and reuse them by increasing their multiplicity.
            t.multiplicity = F::from_canonical_u32(multiplicity[row as usize]);

            // Store the trace in the vector
            trace.push(t);
        }

        // Return successfully
        trace
    }
}

impl<F> WitnessComponent<F> for BinaryBasicTableSM {
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

impl<F: AbstractField> Provable<ZiskRequiredBinaryBasedTable, OpResult, F> for BinaryBasicTableSM {
    fn calculate(
        &self,
        operation: ZiskRequiredBinaryBasedTable,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result: OpResult = opcode_execute(operation.opcode, operation.a, operation.b);
        Ok(result)
    }

    fn prove(&self, operations: &[ZiskRequiredBinaryBasedTable], drain: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            while inputs.len() >= PROVE_CHUNK_SIZE || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
                let _drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                //scope.spawn(move |_| {
                // TODO! Implement prove drained_inputs (a chunk of operations)
                //let trace = BasicTableSM::process_slice::<F>(&_drained_inputs);
                let trace = self.process_slice::<F>(&_drained_inputs);
                //BinaryBasicTableSM::process_slice(self, _drained_inputs);
                //});
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: ZiskRequiredBinaryBasedTable,
        drain: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = <BinaryBasicTableSM as Provable<
            ZiskRequiredBinaryBasedTable,
            (u64, bool),
            F,
        >>::calculate(self, operation.clone());
        <BinaryBasicTableSM as Provable<ZiskRequiredBinaryBasedTable, (u64, bool), F>>::prove(
            self,
            &[operation],
            drain,
            scope,
        );
        result
    }
}
