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
use zisk_core::{P2_11, P2_19, P2_8};
use zisk_pil::{BINARY_EXTENSION_TABLE_AIRGROUP_ID, BINARY_EXTENSION_TABLE_AIR_IDS};

pub struct BinaryExtensionTableSM<F> {
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Row multiplicity table
    num_rows: usize,
    multiplicity: Mutex<Vec<u64>>,
}

#[derive(Debug)]
pub enum ExtensionTableSMErr {
    InvalidOpcode,
}

impl<F: Field> BinaryExtensionTableSM<F> {
    const MY_NAME: &'static str = "BinaryET";

    pub fn new(wcm: Arc<WitnessManager<F>>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let air = wcm
            .get_pctx()
            .pilout
            .get_air(BINARY_EXTENSION_TABLE_AIRGROUP_ID, BINARY_EXTENSION_TABLE_AIR_IDS[0]);

        let binary_extension_table = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            num_rows: air.num_rows(),
            multiplicity: Mutex::new(vec![0; air.num_rows()]),
        };
        let binary_extension_table = Arc::new(binary_extension_table);
        wcm.register_component(binary_extension_table.clone(), Some(airgroup_id), Some(air_ids));

        binary_extension_table
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
        vec![0x0d, 0x0e, 0x0f, 0x24, 0x25, 0x26]
    }

    pub fn process_slice(&self, input: &[u64]) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (i, val) in input.iter().enumerate() {
            multiplicity[i] += *val;
        }
    }

    //lookup_proves(BINARY_EXTENSION_TABLE_ID, [OP, OFFSET, A, B, C0, C1], multiplicity);
    pub fn calculate_table_row(opcode: u8, offset: u64, a: u64, b: u64) -> u64 {
        // Calculate the different row offset contributors, according to the PIL
        assert!(a <= 0xff);
        let offset_a: u64 = a;
        assert!(offset < 0x08);
        let offset_offset: u64 = offset * P2_8;
        assert!(b <= 0x3f);
        let offset_b: u64 = b * P2_11;
        let offset_opcode: u64 = Self::offset_opcode(opcode);

        offset_a + offset_offset + offset_b + offset_opcode
        //assert!(row < self.num_rows as u64);
    }

    fn offset_opcode(opcode: u8) -> u64 {
        match opcode {
            0x0d => 0,
            0x0e => P2_19,
            0x0f => 2 * P2_19,
            0x1d => 3 * P2_19,
            0x1e => 4 * P2_19,
            0x1f => 5 * P2_19,
            0x23 => 6 * P2_19,
            0x24 => 6 * P2_19 + P2_11,
            0x25 => 6 * P2_19 + 2 * P2_11,
            _ => panic!("BinaryExtensionTableSM::offset_opcode() got invalid opcode={}", opcode),
        }
    }

    pub fn create_air_instance(&self) {
        let ectx = self.wcm.get_ectx();
        let mut dctx: std::sync::RwLockWriteGuard<'_, proofman_common::DistributionCtx> =
            ectx.dctx.write().unwrap();

        let mut multiplicity = self.multiplicity.lock().unwrap();

        let (is_myne, instance_idx) = dctx.add_instance(
            BINARY_EXTENSION_TABLE_AIRGROUP_ID,
            BINARY_EXTENSION_TABLE_AIR_IDS[0],
            1,
        );
        let owner = dctx.owner(instance_idx);

        let mut multiplicity_ = std::mem::take(&mut *multiplicity);
        dctx.distribute_multiplicity(&mut multiplicity_, owner);

        if is_myne {
            // Create the prover buffer
            let (mut prover_buffer, offset) = create_prover_buffer(
                self.wcm.get_ectx(),
                self.wcm.get_sctx(),
                BINARY_EXTENSION_TABLE_AIRGROUP_ID,
                BINARY_EXTENSION_TABLE_AIR_IDS[0],
            );

            prover_buffer[offset as usize..offset as usize + self.num_rows]
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, input)| *input = F::from_canonical_u64(multiplicity_[i]));

            info!(
                "{}: ··· Creating Binary extension table instance [{} rows filled 100%]",
                Self::MY_NAME,
                self.num_rows,
            );

            let air_instance = AirInstance::new(
                BINARY_EXTENSION_TABLE_AIRGROUP_ID,
                BINARY_EXTENSION_TABLE_AIR_IDS[0],
                None,
                prover_buffer,
            );
            self.wcm.get_pctx().air_instance_repo.add_air_instance(air_instance);
        }
    }
}

impl<F: Send + Sync> WitnessComponent<F> for BinaryExtensionTableSM<F> {}
