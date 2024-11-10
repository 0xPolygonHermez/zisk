use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

use log::info;
use p3_field::PrimeField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;

use sm_common::create_prover_buffer;
use zisk_pil::{MemAlignRomRow, MemAlignRomTrace, MEM_ALIGN_ROM_AIR_IDS, ZISK_AIRGROUP_ID};

#[derive(Debug, Clone, Copy)]
pub enum MemOp {
    OneRead,
    OneWrite,
    TwoReads,
    TwoWrites,
}

const CHUNK_NUM: usize = 8;
const OP_SIZES: [usize; 4] = [2, 3, 3, 5];

pub struct MemAlignRomSM<F> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Rom data
    num_rows: usize,
    multiplicity: Mutex<HashMap<u64, u64>>, // row_num -> multiplicity
}

#[derive(Debug)]
pub enum ExtensionTableSMErr {
    InvalidOpcode,
}

impl<F: PrimeField> MemAlignRomSM<F> {
    const MY_NAME: &'static str = "MemAlignRom";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let pctx = wcm.get_pctx();
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_ALIGN_ROM_AIR_IDS[0]);
        let num_rows = air.num_rows();

        let mem_align_rom = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            num_rows,
            multiplicity: Mutex::new(HashMap::with_capacity(num_rows)),
        };
        let mem_align_rom = Arc::new(mem_align_rom);
        wcm.register_component(
            mem_align_rom.clone(),
            Some(ZISK_AIRGROUP_ID),
            Some(MEM_ALIGN_ROM_AIR_IDS),
        );

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

    pub fn get_mem_align_op_size(op: MemOp) -> usize {
        OP_SIZES[op as usize]
    }

    fn calculate_rom_rows(opcode: MemOp, offset: usize, width: usize) -> Vec<u64> {
        // Calculate the ROM rows based on the requested opcode, offset, and width
        match opcode {
            MemOp::OneRead | MemOp::OneWrite => {
                // Sanity check
                assert!(offset + width <= CHUNK_NUM);
                let possible_widths = match offset {
                    x if x <= 4 => vec![1, 2, 4],
                    x if x <= 6 => vec![1, 2],
                    x if x == 7 => vec![1],
                    _ => panic!("Invalid offset={}", offset),
                };
                Self::get_row_idxs(opcode, possible_widths, offset, width)
            }
            MemOp::TwoReads | MemOp::TwoWrites => {
                // Sanity check
                assert!(offset + width > CHUNK_NUM);
                let possible_widths = match offset {
                    x if x == 0 => panic!("Invalid offset={}", offset),
                    x if x <= 4 => vec![8],
                    x if x <= 6 => vec![4, 8],
                    x if x == 7 => vec![2, 4, 8],
                    _ => panic!("Invalid offset={}", offset),
                };
                Self::get_row_idxs(opcode, possible_widths, offset, width)
            }
        }
    }

    fn get_row_idxs(
        opcode: MemOp,
        possible_widths: Vec<usize>,
        offset: usize,
        width: usize,
    ) -> Vec<u64> {
        // Sanity check
        assert!(possible_widths.contains(&width));

        let width_idx = possible_widths.iter().position(|&w| w == width).unwrap();
        let opcode_idx = opcode as usize;
        match opcode {
            MemOp::OneRead | MemOp::OneWrite => {
                let value_row = (offset * possible_widths.len() * OP_SIZES[opcode_idx]
                    + (offset + width_idx + 1) * OP_SIZES[opcode_idx]
                    - 1) as u64;
                match opcode {
                    MemOp::OneRead => vec![value_row - 1, value_row],
                    MemOp::OneWrite => vec![value_row - 2, value_row - 1, value_row],
                    _ => unreachable!(),
                }
            }
            MemOp::TwoReads => {
                let value_row = (offset * possible_widths.len() * OP_SIZES[opcode_idx]
                    + (offset + width_idx + 1) * OP_SIZES[opcode_idx]
                    - 2) as u64;
                return vec![value_row - 1, value_row, value_row + 1];
            }
            MemOp::TwoWrites => {
                let value_row = (offset * possible_widths.len() * OP_SIZES[opcode_idx]
                    + (offset + width_idx + 1) * OP_SIZES[opcode_idx]
                    - 3) as u64;
                return vec![value_row - 2, value_row - 1, value_row, value_row + 1, value_row + 2];
            }
        }
    }

    pub fn calculate_next_pc(&self, op: MemOp, offset: usize, width: usize) -> u64 {
        let row_idxs = Self::calculate_rom_rows(op, offset, width);

        // Update the multiplicity
        self.update_multiplicity_by_idx(&row_idxs);

        // The "next" pc is always found on the second row of the program being executed
        row_idxs[1]
    }

    pub fn update_padding_row(&self, padding_len: u64) {
        // Update entry at the padding row (pos = 0) with the given padding length
        self.update_multiplicity(&[padding_len]);
    }

    pub fn update_multiplicity_by_input(&self, opcode: MemOp, offset: usize, width: usize) {
        let row_idxs = Self::calculate_rom_rows(opcode, offset, width);
        self.update_multiplicity_by_idx(&row_idxs);
    }

    pub fn update_multiplicity_by_idx(&self, idxs: &[u64]) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for &i in idxs {
            *multiplicity.entry(i).or_insert(0) += 1;
        }
    }

    pub fn update_multiplicity(&self, inputs: &[u64]) {
        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (idx, mul) in inputs.iter().enumerate() {
            *multiplicity.entry(idx as u64).or_insert(0) += *mul;
        }
    }

    pub fn create_air_instance(&self) {
        // Get the contexts
        let wcm = self.wcm.clone();
        let pctx = wcm.get_pctx();
        let ectx = wcm.get_ectx();
        let sctx = wcm.get_sctx();

        // Get the Mem Align ROM AIR
        let air_mem_align_rom = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_ALIGN_ROM_AIR_IDS[0]);
        let air_mem_align_rom_rows = air_mem_align_rom.num_rows();

        // Create a prover buffer
        let (mut prover_buffer, offset) =
            create_prover_buffer(&ectx, &sctx, ZISK_AIRGROUP_ID, MEM_ALIGN_ROM_AIR_IDS[0]);

        // Create the Mem Align ROM trace buffer
        let mut trace_buffer = MemAlignRomTrace::<F>::map_buffer(
            &mut prover_buffer,
            air_mem_align_rom_rows,
            offset as usize,
        )
        .unwrap();

        if let Ok(multiplicity) = self.multiplicity.lock() {
            for (row_idx, multiplicity) in multiplicity.iter() {
                trace_buffer[*row_idx as usize] =
                    MemAlignRomRow { multiplicity: F::from_canonical_u64(*multiplicity) };
            }
        }

        info!(
            "{}: ··· Creating Mem Align ROM instance [{} rows filled 100%]",
            Self::MY_NAME,
            self.num_rows,
        );

        let air_instance =
            AirInstance::new(sctx, ZISK_AIRGROUP_ID, MEM_ALIGN_ROM_AIR_IDS[0], None, prover_buffer);
        pctx.air_instance_repo.add_air_instance(air_instance, None);
    }
}

impl<F: Send + Sync> WitnessComponent<F> for MemAlignRomSM<F> {}
