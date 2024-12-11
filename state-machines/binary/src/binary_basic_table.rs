use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use log::info;
use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use rayon::prelude::*;
use zisk_core::{zisk_ops::ZiskOp, P2_16, P2_17, P2_18, P2_19, P2_8, P2_9};
use zisk_pil::{BinaryTableTrace, BINARY_TABLE_AIR_IDS, ZISK_AIRGROUP_ID};

#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BinaryBasicTableOp {
    Minu = 0x02,
    Min = 0x03,
    Maxu = 0x04,
    Max = 0x05,
    LtAbsNP = 0x06,
    LtAbsPN = 0x07,
    Ltu = 0x08,
    Lt = 0x09,
    Gt = 0x0a,
    Eq = 0x0b,
    Add = 0x0c,
    Sub = 0x0d,
    Leu = 0x0e,
    Le = 0x0f,
    And = 0x10,
    Or = 0x11,
    Xor = 0x12,
    Ext32 = 0x13,
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

    // TODO: Add new ops?
    pub fn operations() -> Vec<u8> {
        vec![
            ZiskOp::Minu.code(),
            ZiskOp::Min.code(),
            ZiskOp::Maxu.code(),
            ZiskOp::Max.code(),
            ZiskOp::Ltu.code(),
            ZiskOp::Lt.code(),
            ZiskOp::Eq.code(),
            ZiskOp::Add.code(),
            ZiskOp::Sub.code(),
            ZiskOp::Leu.code(),
            ZiskOp::Le.code(),
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
        flags: u64,
    ) -> u64 {
        debug_assert!(a <= 0xFF);
        debug_assert!(b <= 0xFF);
        debug_assert!(cin <= 0x03);
        debug_assert!(last <= 0x01);
        debug_assert!(flags <= 0x0F);

        // Calculate the different row offset contributors, according to the PIL
        if opcode == BinaryBasicTableOp::Ext32 {
            let offset_a: u64 = a;
            let offset_cin: u64 = cin * P2_8;
            let offset_result_is_a: u64 = match flags {
                0 => 0,
                2 => P2_9,
                6 => 3 * P2_9,
                _ => {
                    panic!("BinaryBasicTableSM::calculate_table_row() unexpected flags={}", flags)
                }
            };
            let offset_opcode: u64 = Self::offset_opcode(opcode);

            offset_a + offset_cin + offset_result_is_a + offset_opcode
        } else {
            let offset_a: u64 = a;
            let offset_b: u64 = b * P2_8;
            let offset_last: u64 = if Self::opcode_has_last(opcode) { last * P2_16 } else { 0 };
            let offset_cin: u64 = if Self::opcode_has_cin(opcode) { cin * P2_17 } else { 0 };
            let offset_result_is_a: u64 =
                if Self::opcode_result_is_a(opcode) && ((flags & 0x04) != 0) { P2_18 } else { 0 };
            let offset_opcode: u64 = Self::offset_opcode(opcode);

            offset_a + offset_b + offset_last + offset_cin + offset_result_is_a + offset_opcode
        }
    }

    fn opcode_has_last(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Minu |
            BinaryBasicTableOp::Min |
            BinaryBasicTableOp::Maxu |
            BinaryBasicTableOp::Max |
            BinaryBasicTableOp::LtAbsNP |
            BinaryBasicTableOp::LtAbsPN |
            BinaryBasicTableOp::Ltu |
            BinaryBasicTableOp::Lt |
            BinaryBasicTableOp::Gt |
            BinaryBasicTableOp::Eq |
            BinaryBasicTableOp::Add |
            BinaryBasicTableOp::Sub |
            BinaryBasicTableOp::Leu |
            BinaryBasicTableOp::Le |
            BinaryBasicTableOp::And |
            BinaryBasicTableOp::Or |
            BinaryBasicTableOp::Xor => true,
            BinaryBasicTableOp::Ext32 => false,
        }
    }

    fn opcode_has_cin(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Minu |
            BinaryBasicTableOp::Min |
            BinaryBasicTableOp::Maxu |
            BinaryBasicTableOp::Max |
            BinaryBasicTableOp::LtAbsNP |
            BinaryBasicTableOp::LtAbsPN |
            BinaryBasicTableOp::Ltu |
            BinaryBasicTableOp::Lt |
            BinaryBasicTableOp::Gt |
            BinaryBasicTableOp::Eq |
            BinaryBasicTableOp::Add |
            BinaryBasicTableOp::Sub => true,

            BinaryBasicTableOp::Leu |
            BinaryBasicTableOp::Le |
            BinaryBasicTableOp::And |
            BinaryBasicTableOp::Or |
            BinaryBasicTableOp::Xor |
            BinaryBasicTableOp::Ext32 => false,
        }
    }

    fn opcode_result_is_a(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Minu |
            BinaryBasicTableOp::Min |
            BinaryBasicTableOp::Maxu |
            BinaryBasicTableOp::Max => true,

            BinaryBasicTableOp::LtAbsNP |
            BinaryBasicTableOp::LtAbsPN |
            BinaryBasicTableOp::Ltu |
            BinaryBasicTableOp::Lt |
            BinaryBasicTableOp::Gt |
            BinaryBasicTableOp::Eq |
            BinaryBasicTableOp::Add |
            BinaryBasicTableOp::Sub |
            BinaryBasicTableOp::Leu |
            BinaryBasicTableOp::Le |
            BinaryBasicTableOp::And |
            BinaryBasicTableOp::Or |
            BinaryBasicTableOp::Xor |
            BinaryBasicTableOp::Ext32 => false,
        }
    }

    fn offset_opcode(opcode: BinaryBasicTableOp) -> u64 {
        match opcode {
            BinaryBasicTableOp::Minu => 0,
            BinaryBasicTableOp::Min => P2_19,
            BinaryBasicTableOp::Maxu => 2 * P2_19,
            BinaryBasicTableOp::Max => 3 * P2_19,
            BinaryBasicTableOp::LtAbsNP => 4 * P2_19,
            BinaryBasicTableOp::LtAbsPN => 5 * P2_19,
            BinaryBasicTableOp::Ltu => 6 * P2_19,
            BinaryBasicTableOp::Lt => 6 * P2_19 + P2_18,
            BinaryBasicTableOp::Gt => 6 * P2_19 + 2 * P2_18,
            BinaryBasicTableOp::Eq => 6 * P2_19 + 3 * P2_18,
            BinaryBasicTableOp::Add => 6 * P2_19 + 4 * P2_18,
            BinaryBasicTableOp::Sub => 6 * P2_19 + 5 * P2_18,
            BinaryBasicTableOp::Leu => 6 * P2_19 + 6 * P2_18,
            BinaryBasicTableOp::Le => 6 * P2_19 + 6 * P2_18 + P2_17,
            BinaryBasicTableOp::And => 6 * P2_19 + 6 * P2_18 + 2 * P2_17,
            BinaryBasicTableOp::Or => 6 * P2_19 + 6 * P2_18 + 3 * P2_17,
            BinaryBasicTableOp::Xor => 6 * P2_19 + 6 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Ext32 => 6 * P2_19 + 6 * P2_18 + 5 * P2_17,
        }
    }

    pub fn create_air_instance(&self) {
        let ectx = self.wcm.get_ectx();
        let mut dctx: std::sync::RwLockWriteGuard<'_, proofman_common::DistributionCtx> =
            ectx.dctx.write().unwrap();
        let mut multiplicity = self.multiplicity.lock().unwrap();

        let (is_myne, instance_global_idx) =
            dctx.add_instance(ZISK_AIRGROUP_ID, BINARY_TABLE_AIR_IDS[0], 1);
        let owner: usize = dctx.owner(instance_global_idx);

        let mut multiplicity_ = std::mem::take(&mut *multiplicity);
        dctx.distribute_multiplicity(&mut multiplicity_, owner);

        if is_myne {
            // Create the prover buffer
            let trace: BinaryTableTrace<'_, _> = BinaryTableTrace::new(self.num_rows);
            let mut prover_buffer = trace.buffer.unwrap();

            prover_buffer[0..self.num_rows]
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
