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
use zisk_core::{zisk_ops::ZiskOp, P2_11, P2_19, P2_8};
use zisk_pil::{MEM_UNALIGNED_ROM_AIRGROUP_ID, MEM_UNALIGNED_ROM_AIR_IDS};

const MEM_WIDTHS: [u64; 4] = [1, 2, 4, 8];
const PROGRAM_SIZES: [u64; 4] = [2, 3, 3, 5];

pub struct MemUnalignedRomSM<F> {
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Rom data
    num_rows: usize,
    line: Mutex<u64>,
    multiplicity: Mutex<Vec<u64>>,
}

#[derive(Debug)]
pub enum ExtensionTableSMErr {
    InvalidOpcode,
}

impl<F: Field> MemUnalignedRomSM<F> {
    const MY_NAME: &'static str = "MemUnalignedRom";

    pub fn new(wcm: Arc<WitnessManager<F>>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let pctx = wcm.get_pctx();
        let air = pctx
            .pilout
            .get_air(MEM_UNALIGNED_ROM_AIRGROUP_ID, MEM_UNALIGNED_ROM_AIR_IDS[0]);

        let mem_unaligned_rom = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            num_rows: air.num_rows(),
            line: 0,
            multiplicity: Mutex::new(vec![0; air.num_rows()]),
        };
        let mem_unaligned_rom = Arc::new(mem_unaligned_rom);
        wcm.register_component(mem_unaligned_rom.clone(), Some(airgroup_id), Some(air_ids));

        mem_unaligned_rom
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.create_air_instance();
        }
    }

    pub fn process_slice(&self, input: &[u64]) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (i, val) in input.iter().enumerate() {
            multiplicity[i] += *val;
        }
    }

    //lookup_proves(MEM_UNALIGNED_ROM_ID, [OP, OFFSET, A, B, C0, C1], multiplicity);
    // lookup_proves(MEM_ALIGN_ROM_ID, [PC, DELTA_PC, DELTA_ADDR, OFFSET, WIDTH, FLAGS], multiplicity);
    pub fn calculate_rom_row(opcode: MemUnalignedRomOp, offset: u64, a: u64, b: u64) -> u64 {
        // Calculate the different row offset contributors, according to the PIL
        assert!(a <= 0xff);
        let offset_a: u64 = a;
        assert!(offset < 0x08);
        let offset_offset: u64 = offset * P2_8;
        assert!(b <= 0x3f);
        let offset_b: u64 = b * P2_11;
        let offset_opcode: u64 = Self::offset_opcode(opcode);

        offset_a + offset_offset + offset_b + offset_opcode
    }

    pub fn get_program(offset: u64, width: u64, is_wr: bool) -> usize {
        match (is_wr, offset + width > 8) {
            (false, false) => 0, // RV // TODO: Use an enum instead!
            (true, false) => 1,  // RWV
            (false, true) => 2,  // RVR
            (true, true) => 3,   // RWVWR
        }
    }

    pub fn get_program_size(offset: u64, width: u64, is_wr: bool) -> usize {
        PROGRAM_SIZES[Self::get_program(offset, width, is_wr)]
    }

    // TODO
    pub fn calculate_next_pc(offset: u8, width: u8, is_wr: bool) -> u64 {
        match (offset, width) {
            (x,1) if x < 5 => (x+1) * PROGRAM_SIZES[0] - 1,
            (x,2) => 2 * PROGRAM_SIZES[0] - 1,
            (x,4) => 3 * PROGRAM_SIZES[0] - 1,
            (x,8) => panic!("Aligned Memory access: offset=0, width=8"),

            (1,1) => 4 * PROGRAM_SIZES[0] - 1,
            (1,2) => 5 * PROGRAM_SIZES[0] - 1,
            (1,4) => 6 * PROGRAM_SIZES[0] - 1,
            // (1,8) => 7 * PROGRAM_SIZES[0] - 1, // Two words

            (2,1) => 4 * PROGRAM_SIZES[0] - 1,
            (2,2) => 5 * PROGRAM_SIZES[0] - 1,
            (2,4) => 6 * PROGRAM_SIZES[0] - 1,
            // (2,8) => 7 * PROGRAM_SIZES[0] - 1, // Two words
        }
    }

    fn offset_opcode(opcode: MemUnalignedRomOp) -> u64 {
        match opcode {
            MemUnalignedRomOp::Sll => 0,
            MemUnalignedRomOp::Srl => P2_19,
            MemUnalignedRomOp::Sra => 2 * P2_19,
            MemUnalignedRomOp::SllW => 3 * P2_19,
            MemUnalignedRomOp::SrlW => 4 * P2_19,
            MemUnalignedRomOp::SraW => 5 * P2_19,
            MemUnalignedRomOp::SignExtendB => 6 * P2_19,
            MemUnalignedRomOp::SignExtendH => 6 * P2_19 + P2_11,
            MemUnalignedRomOp::SignExtendW => 6 * P2_19 + 2 * P2_11,
            //_ => panic!("MemUnalignedRomSM::offset_opcode() got invalid opcode={:?}", opcode),
        }
    }

    pub fn create_air_instance(&self) {
        let ectx = self.wcm.get_ectx();
        let mut dctx: std::sync::RwLockWriteGuard<'_, proofman_common::DistributionCtx> =
            ectx.dctx.write().unwrap();

        let mut multiplicity = self.multiplicity.lock().unwrap();

        let (is_myne, instance_global_idx) = dctx.add_instance(
            MEM_UNALIGNED_ROM_AIRGROUP_ID,
            MEM_UNALIGNED_ROM_AIR_IDS[0],
            1,
        );
        let owner = dctx.owner(instance_global_idx);

        let mut multiplicity_ = std::mem::take(&mut *multiplicity);
        dctx.distribute_multiplicity(&mut multiplicity_, owner);

        if is_myne {
            // Create the prover buffer
            let (mut prover_buffer, offset) = create_prover_buffer(
                &self.wcm.get_ectx(),
                &self.wcm.get_sctx(),
                MEM_UNALIGNED_ROM_AIRGROUP_ID,
                MEM_UNALIGNED_ROM_AIR_IDS[0],
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
                self.wcm.get_sctx(),
                MEM_UNALIGNED_ROM_AIRGROUP_ID,
                MEM_UNALIGNED_ROM_AIR_IDS[0],
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

impl<F: Send + Sync> WitnessComponent<F> for MemUnalignedRomSM<F> {}