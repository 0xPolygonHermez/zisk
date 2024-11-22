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
const OP_SIZES: [u64; 4] = [2, 3, 3, 5];
const ONE_WORD_COMBINATIONS: u64 = 20; // (0..4,[1,2,4]), (5,6,[1,2]), (7,[1]) -> 5*3 + 2*2 + 1*1 = 20
const TWO_WORD_COMBINATIONS: u64 = 11; // (1..4,[8]), (5,6,[4,8]), (7,[2,4,8]) -> 4*1 + 2*2 + 1*3 = 11

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

    pub fn calculate_next_pc(&self, opcode: MemOp, offset: usize, width: usize) -> u64 {
        let row_idxs = Self::get_row_idxs(&self, opcode, offset, width);

        // Update the multiplicity
        let ones: Vec<u64> = vec![1; row_idxs.len()];
        self.update_multiplicity_by_row_idx(&row_idxs, &ones);

        row_idxs[0]
    }

    fn get_row_idxs(&self, opcode: MemOp, offset: usize, width: usize) -> Vec<u64> {
        let opcode_idx = opcode as usize;
        let op_size = OP_SIZES[opcode_idx];
        match opcode {
            MemOp::OneRead | MemOp::OneWrite => {
                // Sanity check
                assert!(offset + width <= CHUNK_NUM);

                // Go to the actual operation
                let mut value_row = match opcode {
                    MemOp::OneRead => 1,
                    MemOp::OneWrite => 1 + ONE_WORD_COMBINATIONS * OP_SIZES[0],
                    _ => unreachable!(),
                };

                match opcode {
                    MemOp::OneRead => {
                        if offset == 7 && width == 1
                        {
                            println!("OneRead value_row: {}", value_row);
                        }
                    },
                    MemOp::OneWrite => {
                        if offset == 3 && width == 1
                        {
                            println!("OneWrite value_row: {}", value_row);
                        }
                    },
                    _ => {}
                }

                // Go to the actual offset
                for i in 0..offset {
                    let possible_widths = Self::calculate_possible_widths(true, i);
                    value_row += op_size * possible_widths.len() as u64;
                }

                match opcode {
                    MemOp::OneRead => {
                        if offset == 7 && width == 1
                        {
                            println!("OneRead value_row: {}", value_row);
                        }
                    },
                    MemOp::OneWrite => {
                        if offset == 3 && width == 1
                        {
                            println!("OneWrite value_row: {}", value_row);
                        }
                    },
                    _ => {}
                }

                // Go to the right width
                let width_idx = Self::calculate_possible_widths(true, offset)
                    .iter()
                    .position(|&w| w == width)
                    .expect("Invalid width");
                value_row += op_size * width_idx as u64;

                match opcode {
                    MemOp::OneRead => {
                        if offset == 7 && width == 1
                        {
                            println!("OneRead value_row: {}", value_row);
                        }
                    },
                    MemOp::OneWrite => {
                        if offset == 3 && width == 1
                        {
                            println!("opsizes: {:?}", op_size);
                            println!("width_idx: {:?}", width_idx);
                            println!("OneWrite value_row: {}", value_row);
                        }
                    },
                    _ => {}
                }

                assert!(value_row < self.num_rows as u64);

                match opcode {
                    MemOp::OneRead => vec![value_row, value_row + 1],
                    MemOp::OneWrite => vec![value_row, value_row + 1, value_row + 2],
                    _ => unreachable!(),
                }
            }
            MemOp::TwoReads | MemOp::TwoWrites => {
                // Sanity check
                assert!(offset + width > CHUNK_NUM);

                // Go to the actual operation
                let mut value_row = match opcode {
                    MemOp::TwoReads => {
                        1 + ONE_WORD_COMBINATIONS * OP_SIZES[0] + ONE_WORD_COMBINATIONS * OP_SIZES[1]
                    }
                    MemOp::TwoWrites => {
                        1 + ONE_WORD_COMBINATIONS * OP_SIZES[0] +
                            ONE_WORD_COMBINATIONS * OP_SIZES[1] +
                            TWO_WORD_COMBINATIONS * OP_SIZES[2]
                    }
                    _ => unreachable!(),
                };

                // Go to the actual offset
                for i in 1..offset {
                    let possible_widths = Self::calculate_possible_widths(false, i);
                    value_row += op_size * possible_widths.len() as u64;
                }

                assert!(value_row < self.num_rows as u64);

                // Go to the right width
                let width_idx = Self::calculate_possible_widths(false, offset)
                    .iter()
                    .position(|&w| w == width)
                    .expect("Invalid width");
                value_row += op_size * width_idx as u64;

                match opcode {
                    MemOp::TwoReads => vec![value_row, value_row + 1, value_row + 2],
                    MemOp::TwoWrites => {
                        vec![value_row, value_row + 1, value_row + 2, value_row + 3, value_row + 4]
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn calculate_possible_widths(one_word: bool, offset: usize) -> Vec<usize> {
        // Calculate the ROM rows based on the requested opcode, offset, and width
        match one_word {
            true => match offset {
                x if x <= 4 => vec![1, 2, 4],
                x if x <= 6 => vec![1, 2],
                x if x == 7 => vec![1],
                _ => panic!("Invalid offset={}", offset),
            },
            false => match offset {
                x if x == 0 => panic!("Invalid offset={}", offset),
                x if x <= 4 => vec![8],
                x if x <= 6 => vec![4, 8],
                x if x == 7 => vec![2, 4, 8],
                _ => panic!("Invalid offset={}", offset),
            },
        }
    }

    pub fn update_padding_row(&self, padding_len: u64) {
        // Update entry at the padding row (pos = 0) with the given padding length
        self.update_multiplicity_by_row_idx(&[0], &[padding_len]);
    }

    pub fn update_multiplicity_by_row_idx(&self, row_idxs: &[u64], muls: &[u64]) {
        if row_idxs.len() != muls.len() {
            panic!("The number of indices and multiplicities must be the same");
        }

        let mut multiplicity = self.multiplicity.lock().unwrap();

        for (i, &idx) in row_idxs.iter().enumerate() {
            *multiplicity.entry(idx).or_insert(0) += muls[i];
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

        // Initialize the trace buffer to zero
        for i in 0..air_mem_align_rom_rows {
            trace_buffer[i] = MemAlignRomRow { multiplicity: F::zero() };
        }

        // Fill the trace buffer with the multiplicity values
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
