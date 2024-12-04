use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use log::info;
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use rayon::prelude::*;
use zisk_core::{zisk_ops::ZiskOp, P2_11, P2_19, P2_8};
use zisk_pil::{BinaryExtensionTableTrace, BINARY_EXTENSION_TABLE_AIR_IDS, ZISK_AIRGROUP_ID};

#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BinaryExtensionTableOp {
    Sll = 0x31,
    Srl = 0x32,
    Sra = 0x33,
    SllW = 0x34,
    SrlW = 0x35,
    SraW = 0x36,
    SignExtendB = 0x37,
    SignExtendH = 0x38,
    SignExtendW = 0x39,
}

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
        let pctx = wcm.get_pctx();
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_EXTENSION_TABLE_AIR_IDS[0]);

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

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.create_air_instance();
        }
    }

    pub fn operations() -> Vec<u8> {
        vec![
            ZiskOp::Sll.code(),
            ZiskOp::Srl.code(),
            ZiskOp::Sra.code(),
            ZiskOp::SllW.code(),
            ZiskOp::SrlW.code(),
            ZiskOp::SraW.code(),
            ZiskOp::SignExtendB.code(),
            ZiskOp::SignExtendH.code(),
            ZiskOp::SignExtendW.code(),
        ]
    }

    pub fn process_slice(&self, input: &[u64]) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (i, val) in input.iter().enumerate() {
            multiplicity[i] += *val;
        }
    }

    //lookup_proves(BINARY_EXTENSION_TABLE_ID, [OP, OFFSET, A, B, C0, C1], multiplicity);
    pub fn calculate_table_row(opcode: BinaryExtensionTableOp, offset: u64, a: u64, b: u64) -> u64 {
        debug_assert!(offset <= 0x07);
        debug_assert!(a <= 0xFF);
        debug_assert!(b <= 0xFF);

        // Calculate the different row offset contributors, according to the PIL
        let offset_a: u64 = a;
        let offset_offset: u64 = offset * P2_8;
        let offset_b: u64 = b * P2_11;
        let offset_opcode: u64 = Self::offset_opcode(opcode);

        offset_a + offset_offset + offset_b + offset_opcode
    }

    fn offset_opcode(opcode: BinaryExtensionTableOp) -> u64 {
        match opcode {
            BinaryExtensionTableOp::Sll => 0,
            BinaryExtensionTableOp::Srl => P2_19,
            BinaryExtensionTableOp::Sra => 2 * P2_19,
            BinaryExtensionTableOp::SllW => 3 * P2_19,
            BinaryExtensionTableOp::SrlW => 4 * P2_19,
            BinaryExtensionTableOp::SraW => 5 * P2_19,
            BinaryExtensionTableOp::SignExtendB => 6 * P2_19,
            BinaryExtensionTableOp::SignExtendH => 6 * P2_19 + P2_11,
            BinaryExtensionTableOp::SignExtendW => 6 * P2_19 + 2 * P2_11,
        }
    }

    pub fn create_air_instance(&self) {
        let ectx = self.wcm.get_ectx();
        let mut dctx: std::sync::RwLockWriteGuard<'_, proofman_common::DistributionCtx> =
            ectx.dctx.write().unwrap();

        let mut multiplicity = self.multiplicity.lock().unwrap();

        let (is_myne, instance_global_idx) =
            dctx.add_instance(ZISK_AIRGROUP_ID, BINARY_EXTENSION_TABLE_AIR_IDS[0], 1);
        let owner = dctx.owner(instance_global_idx);

        let mut multiplicity_ = std::mem::take(&mut *multiplicity);
        dctx.distribute_multiplicity(&mut multiplicity_, owner);

        if is_myne {
            let trace: BinaryExtensionTableTrace<'_, _> = BinaryExtensionTableTrace::new(self.num_rows);
            let mut prover_buffer = trace.buffer.unwrap();

            prover_buffer[0..self.num_rows]
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, input)| *input = F::from_canonical_u64(multiplicity_[i]));

            info!(
                "{}: ··· Creating Binary extension table instance [{} rows filled 100%]",
                Self::MY_NAME,
                self.num_rows,
            );

            let air_instance = AirInstance::new(
                self.wcm.get_sctx(),
                ZISK_AIRGROUP_ID,
                BINARY_EXTENSION_TABLE_AIR_IDS[0],
                None,
                prover_buffer,
            );
            self.wcm
                .get_pctx()
                .air_instance_repo
                .add_air_instance(air_instance, Some(instance_global_idx));
        }
    }
}

impl<F: Send + Sync> WitnessComponent<F> for BinaryExtensionTableSM<F> {}
