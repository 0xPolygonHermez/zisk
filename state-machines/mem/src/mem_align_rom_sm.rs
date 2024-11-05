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
use zisk_pil::{MEM_ALIGN_ROM_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::MemOps;

const CHUNKS: usize = 8;
const MEM_WIDTHS: [u64; 4] = [1, 2, 4, 8];
const OP_SIZES: [usize; 4] = [2, 3, 3, 5];

pub struct MemAlignRomSM<F> {
    // Witness computation manager
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

impl<F: Field> MemAlignRomSM<F> {
    const MY_NAME: &'static str = "MemAlignRom";

    pub fn new(wcm: Arc<WitnessManager<F>>, airgroup_id: usize, air_ids: &[usize]) -> Arc<Self> {
        let pctx = wcm.get_pctx();
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_ALIGN_ROM_AIR_IDS[0]);

        let mem_align_rom = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            num_rows: air.num_rows(),
            line: Mutex::new(0),
            multiplicity: Mutex::new(vec![0; air.num_rows()]),
        };
        let mem_align_rom = Arc::new(mem_align_rom);
        wcm.register_component(mem_align_rom.clone(), Some(airgroup_id), Some(air_ids));

        mem_align_rom
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.create_air_instance();
        }
    }

    pub fn get_op_size(op: MemOps) -> usize {
        OP_SIZES[op as usize]
    }

    pub fn calculate_rom_rows(opcode: MemOps, offset: usize, width: usize) -> Vec<usize> {
        match opcode {
            MemOps::OneRead | MemOps::OneWrite => {
                // Sanity check
                assert!(offset + width <= CHUNKS);
                let possible_widths = match offset {
                    x if x <= 4 => vec![1, 2, 4],
                    x if x <= 6 => vec![1, 2],
                    x if x == 7 => vec![1],
                    _ => panic!("Invalid offset={}", offset),
                };
                Self::get_rows(opcode, possible_widths, offset, width)
            }
            MemOps::TwoReads | MemOps::TwoWrites => {
                // Sanity check
                assert!(offset + width > CHUNKS);
                let possible_widths = match offset {
                    x if x == 0 => panic!("Invalid offset={}", offset),
                    x if x <= 4 => vec![8],
                    x if x <= 6 => vec![4, 8],
                    x if x == 7 => vec![2, 4, 8],
                    _ => panic!("Invalid offset={}", offset),
                };
                Self::get_rows(opcode, possible_widths, offset, width)
            }
        }
    }

    fn get_rows(
        opcode: MemOps,
        possible_widths: Vec<usize>,
        offset: usize,
        width: usize,
    ) -> Vec<usize> {
        // Sanity check
        assert!(possible_widths.contains(&width));

        let width_idx = possible_widths.iter().position(|&w| w == width).unwrap();
        let opcode_idx = opcode as usize;
        match opcode {
            MemOps::OneRead | MemOps::OneWrite => {
                let value_row = offset * possible_widths.len() * OP_SIZES[opcode_idx]
                    + (offset + width_idx + 1) * OP_SIZES[opcode_idx]
                    - 1;
                match opcode {
                    MemOps::OneRead => vec![value_row - 1, value_row],
                    MemOps::OneWrite => vec![value_row - 2, value_row - 1, value_row],
                    _ => unreachable!(),
                }
            }
            MemOps::TwoReads => {
                let value_row = offset * possible_widths.len() * OP_SIZES[opcode_idx]
                    + (offset + width_idx + 1) * OP_SIZES[opcode_idx]
                    - 2;
                return vec![value_row - 1, value_row, value_row + 1];
            }
            MemOps::TwoWrites => {
                let value_row = offset * possible_widths.len() * OP_SIZES[opcode_idx]
                    + (offset + width_idx + 1) * OP_SIZES[opcode_idx]
                    - 3;
                return vec![value_row - 2, value_row - 1, value_row, value_row + 1, value_row + 2];
            }
        }
    }

    pub fn calculate_next_pc(op: MemOps, offset: usize, width: usize) -> usize {
        let rows = Self::calculate_rom_rows(op, offset, width);
        rows[1]
    }

    pub fn process_slice(&self, input: &[u8]) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (i, val) in input.iter().enumerate() {
            multiplicity[i] += *val as u64;
        }
    }

    pub fn create_air_instance(&self) {
        let ectx = self.wcm.get_ectx();
        let mut dctx: std::sync::RwLockWriteGuard<'_, proofman_common::DistributionCtx> =
            ectx.dctx.write().unwrap();

        let mut multiplicity = self.multiplicity.lock().unwrap();

        let (is_myne, instance_global_idx) =
            dctx.add_instance(ZISK_AIRGROUP_ID, MEM_ALIGN_ROM_AIR_IDS[0], 1);
        let owner = dctx.owner(instance_global_idx);

        let mut multiplicity_ = std::mem::take(&mut *multiplicity);
        dctx.distribute_multiplicity(&mut multiplicity_, owner);

        if is_myne {
            // Create the prover buffer
            let (mut prover_buffer, offset) = create_prover_buffer(
                &self.wcm.get_ectx(),
                &self.wcm.get_sctx(),
                ZISK_AIRGROUP_ID,
                MEM_ALIGN_ROM_AIR_IDS[0],
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
                ZISK_AIRGROUP_ID,
                MEM_ALIGN_ROM_AIR_IDS[0],
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

impl<F: Send + Sync> WitnessComponent<F> for MemAlignRomSM<F> {}
