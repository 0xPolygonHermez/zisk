use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use zisk_pil::MemAlignRomTrace;

#[derive(Debug, Clone, Copy)]
pub enum MemOp {
    OneRead,
    OneWrite,
    TwoReads,
    TwoWrites,
}

const OP_SIZES: [u64; 4] = [2, 3, 3, 5];
const ONE_WORD_COMBINATIONS: u64 = 20; // (0..4,[1,2,4]), (5,6,[1,2]), (7,[1]) -> 5*3 + 2*2 + 1*1 = 20
const TWO_WORD_COMBINATIONS: u64 = 11; // (1..4,[8]), (5,6,[4,8]), (7,[2,4,8]) -> 4*1 + 2*2 + 1*3 = 11

pub struct MemAlignRomSM {
    multiplicity: Mutex<HashMap<u64, u64>>, // row_num -> multiplicity
}

#[derive(Debug)]
pub enum ExtensionTableSMErr {
    InvalidOpcode,
}

impl MemAlignRomSM {
    // const MY_NAME: &'static str = "MemAlignRom";

    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            multiplicity: Mutex::new(HashMap::with_capacity(MemAlignRomTrace::<usize>::NUM_ROWS)),
        })
    }

    pub fn calculate_next_pc(&self, opcode: MemOp, offset: usize, width: usize) -> u64 {
        // Get the table offset
        let (table_offset, one_word) = match opcode {
            MemOp::OneRead => (1, true),

            MemOp::OneWrite => (1 + ONE_WORD_COMBINATIONS * OP_SIZES[0], true),

            MemOp::TwoReads => (
                1 + ONE_WORD_COMBINATIONS * OP_SIZES[0] + ONE_WORD_COMBINATIONS * OP_SIZES[1],
                false,
            ),

            MemOp::TwoWrites => (
                1 + ONE_WORD_COMBINATIONS * OP_SIZES[0] +
                    ONE_WORD_COMBINATIONS * OP_SIZES[1] +
                    TWO_WORD_COMBINATIONS * OP_SIZES[2],
                false,
            ),
        };

        // Get the first row index
        let first_row_idx = Self::get_first_row_idx(opcode, offset, width, table_offset, one_word);

        // Based on the program size, return the row indices
        let opcode_idx = opcode as usize;
        let op_size = OP_SIZES[opcode_idx];
        for i in 0..op_size {
            let row_idx = first_row_idx + i;
            // Check whether the row index is within the bounds
            debug_assert!(row_idx < MemAlignRomTrace::<usize>::NUM_ROWS as u64);
            // Update the multiplicity
            self.update_multiplicity_by_row_idx(row_idx, 1);
        }

        first_row_idx
    }

    fn get_first_row_idx(
        opcode: MemOp,
        offset: usize,
        width: usize,
        table_offset: u64,
        one_word: bool,
    ) -> u64 {
        let opcode_idx = opcode as usize;
        let op_size = OP_SIZES[opcode_idx];

        // Go to the actual operation
        let mut first_row_idx = table_offset;

        // Go to the actual offset
        let first_valid_offset = if one_word { 0 } else { 1 };
        for i in first_valid_offset..offset {
            let possible_widths = Self::calculate_possible_widths(one_word, i);
            first_row_idx += op_size * possible_widths.len() as u64;
        }

        // Go to the right width
        let width_idx = Self::calculate_possible_widths(one_word, offset)
            .iter()
            .position(|&w| w == width)
            .unwrap_or_else(|| panic!("Invalid width offset:{} width:{}", offset, width));
        first_row_idx += op_size * width_idx as u64;

        first_row_idx
    }

    fn calculate_possible_widths(one_word: bool, offset: usize) -> Vec<usize> {
        // Calculate the ROM rows based on the requested opcode, offset, and width
        match one_word {
            true => match offset {
                x if x <= 4 => vec![1, 2, 4],
                x if x <= 6 => vec![1, 2],
                7 => vec![1],
                _ => panic!("Invalid offset={}", offset),
            },
            false => match offset {
                0 => panic!("Invalid offset={}", offset),
                x if x <= 4 => vec![8],
                x if x <= 6 => vec![4, 8],
                7 => vec![2, 4, 8],
                _ => panic!("Invalid offset={}", offset),
            },
        }
    }

    pub fn detach_multiplicity(&self) -> Vec<u64> {
        let multiplicity = self.multiplicity.lock().unwrap();
        let mut multiplicity_vec = vec![0; MemAlignRomTrace::<usize>::NUM_ROWS];
        for (row_idx, multiplicity) in multiplicity.iter() {
            assert!(*row_idx < MemAlignRomTrace::<usize>::NUM_ROWS as u64);
            multiplicity_vec[*row_idx as usize] = *multiplicity;
        }
        multiplicity_vec
    }

    pub fn update_padding_row(&self, padding_len: u64) {
        // Update entry at the padding row (pos = 0) with the given padding length
        self.update_multiplicity_by_row_idx(0, padding_len);
    }

    pub fn update_multiplicity_by_row_idx(&self, row_idx: u64, mul: u64) {
        let mut multiplicity = self.multiplicity.lock().unwrap();
        *multiplicity.entry(row_idx).or_insert(0) += mul;
    }
}
