use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use log::info;
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use rayon::prelude::*;
use sm_common::create_prover_buffer;
use zisk_core::{zisk_ops::ZiskOp, P2_16, P2_17, P2_18, P2_19, P2_8};
use zisk_pil::{BINARY_TABLE_AIR_IDS, ZISK_AIRGROUP_ID};

#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BinaryBasicTableOp {
    Add = 0x02,
    Sub = 0x03,
    Ltu = 0x04,
    Lt = 0x05,
    Leu = 0x06,
    Le = 0x07,
    Eq = 0x08,
    Minu = 0x09,
    Min = 0x0a,
    Maxu = 0x0b,
    Max = 0x0c,
    And = 0x20,
    Or = 0x21,
    Xor = 0x22,
    Ext32 = 0x23,
}

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
        let pctx = wcm.get_pctx();
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, BINARY_TABLE_AIR_IDS[0]);

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

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.create_air_instance();
        }
    }

    pub fn operations() -> Vec<u8> {
        vec![
            ZiskOp::Add.code(),
            ZiskOp::Sub.code(),
            ZiskOp::Ltu.code(),
            ZiskOp::Lt.code(),
            ZiskOp::Leu.code(),
            ZiskOp::Le.code(),
            ZiskOp::Eq.code(),
            ZiskOp::Minu.code(),
            ZiskOp::Min.code(),
            ZiskOp::Maxu.code(),
            ZiskOp::Max.code(),
            ZiskOp::And.code(),
            ZiskOp::Or.code(),
            ZiskOp::Xor.code(),
        ]
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
        opcode: BinaryBasicTableOp,
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

    fn opcode_has_last(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Add |
            BinaryBasicTableOp::Sub |
            BinaryBasicTableOp::Ltu |
            BinaryBasicTableOp::Lt |
            BinaryBasicTableOp::Leu |
            BinaryBasicTableOp::Le |
            BinaryBasicTableOp::Eq |
            BinaryBasicTableOp::Minu |
            BinaryBasicTableOp::Min |
            BinaryBasicTableOp::Maxu |
            BinaryBasicTableOp::Max |
            BinaryBasicTableOp::And |
            BinaryBasicTableOp::Or |
            BinaryBasicTableOp::Xor => true,
            BinaryBasicTableOp::Ext32 => false,
            //_ => panic!("BinaryBasicTableSM::opcode_has_last() got invalid opcode={:?}", opcode),
        }
    }

    fn opcode_has_cin(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Add |
            BinaryBasicTableOp::Sub |
            BinaryBasicTableOp::Ltu |
            BinaryBasicTableOp::Lt |
            BinaryBasicTableOp::Eq |
            BinaryBasicTableOp::Minu |
            BinaryBasicTableOp::Min |
            BinaryBasicTableOp::Maxu |
            BinaryBasicTableOp::Max => true,

            BinaryBasicTableOp::Leu |
            BinaryBasicTableOp::Le |
            BinaryBasicTableOp::And |
            BinaryBasicTableOp::Or |
            BinaryBasicTableOp::Xor |
            BinaryBasicTableOp::Ext32 => false,
            //_ => panic!("BinaryBasicTableSM::opcode_has_cin() got invalid opcode={:?}", opcode),
        }
    }

    fn opcode_result_is_a(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max => true,

            BinaryBasicTableOp::Add
            | BinaryBasicTableOp::Sub
            | BinaryBasicTableOp::Ltu
            | BinaryBasicTableOp::Lt
            | BinaryBasicTableOp::Leu
            | BinaryBasicTableOp::Le
            | BinaryBasicTableOp::Eq
            | BinaryBasicTableOp::And
            | BinaryBasicTableOp::Or
            | BinaryBasicTableOp::Xor
            | BinaryBasicTableOp::Ext32 => false,
            //_ => panic!("BinaryBasicTableSM::opcode_result_is_a() got invalid opcode={:?}", opcode),
        }
    }

    fn offset_opcode(opcode: BinaryBasicTableOp) -> u64 {
        match opcode {
            BinaryBasicTableOp::Minu => 0,
            BinaryBasicTableOp::Min => P2_19,
            BinaryBasicTableOp::Maxu => 2 * P2_19,
            BinaryBasicTableOp::Max => 3 * P2_19,
            BinaryBasicTableOp::Ltu => 4 * P2_19,
            BinaryBasicTableOp::Lt => 4 * P2_19 + P2_18,
            BinaryBasicTableOp::Eq => 4 * P2_19 + 2 * P2_18,
            BinaryBasicTableOp::Add => 4 * P2_19 + 3 * P2_18,
            BinaryBasicTableOp::Sub => 4 * P2_19 + 4 * P2_18,
            BinaryBasicTableOp::Leu => 4 * P2_19 + 5 * P2_18,
            BinaryBasicTableOp::Le => 4 * P2_19 + 5 * P2_18 + P2_17,
            BinaryBasicTableOp::And => 4 * P2_19 + 5 * P2_18 + 2 * P2_17,
            BinaryBasicTableOp::Or => 4 * P2_19 + 5 * P2_18 + 3 * P2_17,
            BinaryBasicTableOp::Xor => 4 * P2_19 + 5 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Ext32 => 4 * P2_19 + 5 * P2_18 + 5 * P2_17,
            //_ => panic!("BinaryBasicTableSM::offset_opcode() got invalid opcode={:?}", opcode),
        }
    }

    pub fn create_air_instance(&self) {
        let ectx = self.wcm.get_ectx();
        let mut dctx: std::sync::RwLockWriteGuard<'_, proofman_common::DistributionCtx> =
            ectx.dctx.write().unwrap();
        let mut multiplicity = self.multiplicity.lock().unwrap();

        let (is_mine, instance_global_idx) =
            dctx.add_instance(ZISK_AIRGROUP_ID, BINARY_TABLE_AIR_IDS[0], 1);
        let owner: usize = dctx.owner(instance_global_idx);

        let mut multiplicity_ = std::mem::take(&mut *multiplicity);
        dctx.distribute_multiplicity(&mut multiplicity_, owner);

        if is_mine {
            // Create the prover buffer
            let (mut prover_buffer, offset) = create_prover_buffer(
                &self.wcm.get_ectx(),
                &self.wcm.get_sctx(),
                ZISK_AIRGROUP_ID,
                BINARY_TABLE_AIR_IDS[0],
            );
            prover_buffer[offset as usize..offset as usize + self.num_rows]
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, input)| *input = F::from_canonical_u64(multiplicity_[i]));

            info!(
                "{}: ··· Creating Binary basic table instance [{} rows filled 100%]",
                Self::MY_NAME,
                self.num_rows,
            );
            let air_instance = AirInstance::new(
                self.wcm.get_sctx(),
                ZISK_AIRGROUP_ID,
                BINARY_TABLE_AIR_IDS[0],
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

impl<F: Send + Sync> WitnessComponent<F> for BinaryBasicTableSM<F> {}
