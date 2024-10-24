use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use log::info;
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, SetupCtx};
use rayon::{prelude::*, Scope};
use sm_common::create_prover_buffer;
use zisk_core::{zisk_ops::ZiskOp, P2_11, P2_19, P2_8};
use zisk_pil::{BINARY_EXTENSION_TABLE_AIRGROUP_ID, BINARY_EXTENSION_TABLE_AIR_IDS};

#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BinaryExtensionTableOp {
    Sll = 0x0d,
    Srl = 0x0e,
    Sra = 0x0f,
    SllW = 0x1d,
    SrlW = 0x1e,
    SraW = 0x1f,
    SignExtendB = 0x23,
    SignExtendH = 0x24,
    SignExtendW = 0x25,
}

pub struct BinaryExtensionTableSM<F> {
    wcm: Arc<WitnessManager<F>>,
    sctx: Arc<SetupCtx>,

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

    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        sctx: Arc<SetupCtx>,
        airgroup_id: usize,
        air_ids: &[usize],
    ) -> Arc<Self> {
        let air = wcm
            .get_pctx()
            .pilout
            .get_air(BINARY_EXTENSION_TABLE_AIRGROUP_ID, BINARY_EXTENSION_TABLE_AIR_IDS[0]);

        let binary_extension_table = Self {
            wcm: wcm.clone(),
            sctx,
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
            // Create the prover buffer
            let (mut prover_buffer, offset) = create_prover_buffer(
                self.wcm.get_ectx(),
                self.wcm.get_sctx(),
                BINARY_EXTENSION_TABLE_AIRGROUP_ID,
                BINARY_EXTENSION_TABLE_AIR_IDS[0],
            );

            let multiplicity = self.multiplicity.lock().unwrap();

            prover_buffer[offset as usize..offset as usize + self.num_rows]
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, input)| *input = F::from_canonical_u64(multiplicity[i]));

            info!(
                "{}: ··· Creating Binary extension table instance [{} rows filled 100%]",
                Self::MY_NAME,
                self.num_rows,
            );

            let air_instance = AirInstance::new(
                self.sctx.clone(),
                BINARY_EXTENSION_TABLE_AIRGROUP_ID,
                BINARY_EXTENSION_TABLE_AIR_IDS[0],
                None,
                prover_buffer,
            );
            self.wcm.get_pctx().air_instance_repo.add_air_instance(air_instance);
        }
    }

    pub fn operations() -> Vec<u8> {
        // TODO! Review this codes
        vec![
            ZiskOp::Sll.code(),
            ZiskOp::Srl.code(),
            ZiskOp::Sra.code(),
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
            //_ => panic!("BinaryExtensionTableSM::offset_opcode() got invalid opcode={:?}", opcode),
        }
    }
}

impl<F: Send + Sync> WitnessComponent<F> for BinaryExtensionTableSM<F> {}
