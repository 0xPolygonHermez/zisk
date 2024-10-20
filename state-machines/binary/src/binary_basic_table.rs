use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use log::info;
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use rayon::{prelude::*, Scope};
use sm_common::create_prover_buffer;
use zisk_core::{P2_16, P2_17, P2_18, P2_19, P2_8};
use zisk_pil::{BINARY_TABLE_AIRGROUP_ID, BINARY_TABLE_AIR_IDS};

pub struct BinaryBasicTableSM<F> {
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Row multiplicity table
    num_rows: usize,
    multiplicity: Mutex<Vec<u64>>,
}

#[derive(Debug)]
pub enum BasicTableSMErr {
    InvalidOpcode,
}

impl<F: Field> BinaryBasicTableSM<F> {
    const MY_NAME: &'static str = "BinaryT ";

    pub fn new(wcm: Arc<WitnessManager<F>>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let air = wcm.get_pctx().pilout.get_air(BINARY_TABLE_AIRGROUP_ID, BINARY_TABLE_AIR_IDS[0]);

        let binary_basic_table = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            num_rows: air.num_rows(),
            multiplicity: Mutex::new(vec![0; air.num_rows()]),
        };
        let binary_basic_table = Arc::new(binary_basic_table);
        wcm.register_component(binary_basic_table.clone(), Some(airgroup_id), Some(air_ids));

        binary_basic_table
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self, _: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.create_air_instance();
        }
    }

    pub fn operations() -> Vec<u8> {
        // TODO! Review this codes
        vec![0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x20, 0x21, 0x22]
    }

    pub fn process_slice(&self, input: &[u64]) {
        // Create the trace vector
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (i, val) in input.iter().enumerate() {
            multiplicity[i] += *val;
        }
    }

    //lookup_proves(BINARY_TABLE_ID, [LAST, OP, A, B, CIN, C, FLAGS], multiplicity);
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_table_row(
        opcode: u8,
        a: u64,
        b: u64,
        cin: u64,
        last: u64,
        _c: u64,
        _flags: u64,
        _i: u64,
    ) -> u64 {
        // Calculate the different row offset contributors, according to the PIL
        let offset_a: u64 = a;
        let offset_b: u64 = b * P2_8;
        let offset_last: u64 = if Self::opcode_has_last(opcode) { last * P2_16 } else { 0 };
        let offset_cin: u64 = if Self::opcode_has_cin(opcode) { cin * P2_17 } else { 0 };
        let offset_result_is_a: u64 = if Self::opcode_result_is_a(opcode) { P2_18 } else { 0 }; // TODO: Should we add it only if c == a?
        let offset_opcode: u64 = Self::offset_opcode(opcode);

        offset_a + offset_b + offset_last + offset_cin + offset_result_is_a + offset_opcode
        //assert!(row < self.num_rows as u64);
    }

    fn opcode_has_last(opcode: u8) -> bool {
        match opcode {
            0x09 | 0x0a | 0x0b | 0x0c | 0x04 | 0x05 | 0x08 | 0x02 | 0x03 | 0x06 | 0x07 | 0x20 | 0x21 | 0x22 => true,
            0x23 /* EXT_32 */ => false,
            _ => panic!("BinaryBasicTableSM::opcode_has_last() got invalid opcode={}", opcode),
        }
    }

    fn opcode_has_cin(opcode: u8) -> bool {
        match opcode {
            0x09 | 0x0a | 0x0b | 0x0c | 0x04 | 0x05 | 0x08 | 0x02 | 0x03 => true,
            0x06 | 0x07 | 0x20 | 0x21 | 0x22 | 0x23 => false,
            _ => panic!("BinaryBasicTableSM::opcode_has_cin() got invalid opcode={}", opcode),
        }
    }

    fn opcode_result_is_a(opcode: u8) -> bool {
        match opcode {
            0x09..=0x0c => true,
            0x04 | 0x05 | 0x08 | 0x02 | 0x03 | 0x06 | 0x07 | 0x20 | 0x21 | 0x22 | 0x23 => false,
            _ => panic!("BinaryBasicTableSM::opcode_result_is_a() got invalid opcode={}", opcode),
        }
    }

    fn offset_opcode(opcode: u8) -> u64 {
        match opcode {
            0x09 => 0,
            0x0a => P2_19,
            0x0b => 2 * P2_19,
            0x0c => 3 * P2_19,
            0x04 => 4 * P2_19,
            0x05 => 4 * P2_19 + P2_18,
            0x08 => 4 * P2_19 + 2 * P2_18,
            0x02 => 4 * P2_19 + 3 * P2_18,
            0x03 => 4 * P2_19 + 4 * P2_18,
            0x06 => 4 * P2_19 + 5 * P2_18,
            0x07 => 4 * P2_19 + 5 * P2_18 + P2_17,
            0x20 => 4 * P2_19 + 5 * P2_18 + 2 * P2_17,
            0x21 => 4 * P2_19 + 5 * P2_18 + 3 * P2_17,
            0x22 => 4 * P2_19 + 5 * P2_18 + 4 * P2_17,
            0x23 => 4 * P2_19 + 5 * P2_18 + 5 * P2_17,
            _ => panic!("BinaryBasicTableSM::offset_opcode() got invalid opcode={}", opcode),
        }
    }

    pub fn create_air_instance(&self) {
        // Create the prover buffer
        let (mut prover_buffer, offset) = create_prover_buffer(
            self.wcm.get_ectx(),
            self.wcm.get_sctx(),
            BINARY_TABLE_AIRGROUP_ID,
            BINARY_TABLE_AIR_IDS[0],
        );

        let multiplicity = self.multiplicity.lock().unwrap();

        prover_buffer[offset as usize..offset as usize + self.num_rows]
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, input)| *input = F::from_canonical_u64(multiplicity[i]));

        info!(
            "{}: ··· Creating Binary basic table instance [{} rows filled 100%]",
            Self::MY_NAME,
            self.num_rows,
        );

        let air_instance = AirInstance::new(
            BINARY_TABLE_AIRGROUP_ID,
            BINARY_TABLE_AIR_IDS[0],
            None,
            prover_buffer,
        );
        self.wcm.get_pctx().air_instance_repo.add_air_instance(air_instance);
    }
}

impl<F: Send + Sync> WitnessComponent<F> for BinaryBasicTableSM<F> {}
