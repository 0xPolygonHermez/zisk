use std::sync::Arc;

#[cfg(feature = "debug_mem_align")]
use std::sync::Mutex;

use fields::PrimeField64;
use pil_std_lib::Std;

use crate::{MemAlignInput, MemAlignRomSM};
use proofman_common::{AirInstance, FromTrace};
use rayon::prelude::*;
use zisk_pil::{MemAlignTrace, MemAlignTraceRow};

const RC: usize = 2;
pub const CHUNK_NUM: usize = 8;
const CHUNKS_BY_RC: usize = CHUNK_NUM / RC;
const CHUNK_BITS: usize = 8;
const RC_BITS: u64 = (CHUNKS_BY_RC * CHUNK_BITS) as u64;
const RC_MASK: u64 = (1 << RC_BITS) - 1;
pub const OFFSET_MASK: u32 = 0x07;
const OFFSET_BITS: u32 = 3;
const CHUNK_BITS_MASK: u64 = (1 << CHUNK_BITS) - 1;

const fn generate_allowed_offsets() -> [u8; CHUNK_NUM] {
    let mut offsets = [0; CHUNK_NUM];
    let mut i = 0;
    while i < CHUNK_NUM {
        offsets[i] = i as u8;
        i += 1;
    }
    offsets
}

const ALLOWED_OFFSETS: [u8; CHUNK_NUM] = generate_allowed_offsets();
const ALLOWED_WIDTHS: [u8; 4] = [1, 2, 4, 8];
const DEFAULT_OFFSET: u64 = 0;
const DEFAULT_WIDTH: u64 = 8;

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

pub struct MemAlignResponse {
    pub more_addr: bool,
    pub step: u64,
    pub value: Option<u64>,
}
pub struct MemAlignSM<F: PrimeField64> {
    /// PIL2 standard library
    std: Arc<Std<F>>,

    #[cfg(feature = "debug_mem_align")]
    num_computed_rows: Mutex<usize>,

    // Secondary State machines
    mem_align_rom_sm: Arc<MemAlignRomSM>,
}

macro_rules! debug_info {
    ($prefix:expr, $($arg:tt)*) => {
        #[cfg(feature = "debug_mem_align")]
        {
            tracing::info!(concat!("MemAlign: ",$prefix), $($arg)*);
        }
    };
}

impl<F: PrimeField64> MemAlignSM<F> {
    pub fn new(std: Arc<Std<F>>, mem_align_rom_sm: Arc<MemAlignRomSM>) -> Arc<Self> {
        Arc::new(Self {
            std: std.clone(),
            #[cfg(feature = "debug_mem_align")]
            num_computed_rows: Mutex::new(0),
            mem_align_rom_sm,
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
                1 + ONE_WORD_COMBINATIONS * OP_SIZES[0]
                    + ONE_WORD_COMBINATIONS * OP_SIZES[1]
                    + TWO_WORD_COMBINATIONS * OP_SIZES[2],
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
            // Update the multiplicity
            self.mem_align_rom_sm.update_multiplicity_by_row_idx(row_idx, 1);
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
            .unwrap_or_else(|| panic!("Invalid width offset:{offset} width:{width}"));
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
                _ => panic!("Invalid offset={offset}"),
            },
            false => match offset {
                0 => panic!("Invalid offset={offset}"),
                x if x <= 4 => vec![8],
                x if x <= 6 => vec![4, 8],
                7 => vec![2, 4, 8],
                _ => panic!("Invalid offset={offset}"),
            },
        }
    }

    pub fn prove_mem_align_op(
        &self,
        input: &MemAlignInput,
        trace: &mut [MemAlignTraceRow<F>],
    ) -> usize {
        let addr = input.addr;
        let width = input.width;

        // Compute the width
        debug_assert!(
            ALLOWED_WIDTHS.contains(&width),
            "Width={width} is not allowed. Allowed widths are {ALLOWED_WIDTHS:?}"
        );
        let width = width as usize;

        // Compute the offset
        let offset = (addr & OFFSET_MASK) as u8;
        debug_assert!(
            ALLOWED_OFFSETS.contains(&offset),
            "Offset={offset} is not allowed. Allowed offsets are {ALLOWED_OFFSETS:?}"
        );
        let offset = offset as usize;

        #[cfg(feature = "debug_mem_align")]
        let num_rows = self.num_computed_rows.lock().unwrap();
        match (input.is_write, offset + width > CHUNK_NUM) {
            (false, false) => {
                /*  RV with offset=2, width=4
                +----+----+====+====+====+====+----+----+
                | R0 | R1 | R2 | R3 | R4 | R5 | R6 | R7 |
                +----+----+====+====+====+====+----+----+
                                ⇓
                +----+----+====+====+====+====+----+----+
                | V6 | V7 | V0 | V1 | V2 | V3 | V4 | V5 |
                +----+----+====+====+====+====+----+----+
                */
                // Unaligned memory op information thrown into the bus
                let step = input.step;
                let value = input.value;

                // Get the aligned address
                let addr_read = addr >> OFFSET_BITS;

                // Get the aligned value
                let value_read = input.mem_values[0];

                // Get the next pc
                let next_pc = self.calculate_next_pc(MemOp::OneRead, offset, width);

                let mut read_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step),
                    addr: F::from_u32(addr_read),
                    // delta_addr: F::ZERO,
                    offset: F::from_u64(DEFAULT_OFFSET),
                    width: F::from_u64(DEFAULT_WIDTH),
                    // wr: F::from_bool(false),
                    // pc: F::from_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step),
                    addr: F::from_u32(addr_read),
                    // delta_addr: F::ZERO,
                    offset: F::from_usize(offset),
                    width: F::from_usize(width),
                    // wr: F::from_bool(false),
                    pc: F::from_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNK_NUM {
                    read_row.reg[i] = F::from_u64(Self::get_byte(value_read, i, 0));
                    if i >= offset && i < offset + width {
                        read_row.sel[i] = F::from_bool(true);
                    }

                    value_row.reg[i] = F::from_u64(Self::get_byte(value, i, CHUNK_NUM - offset));
                    if i == offset {
                        value_row.sel[i] = F::from_bool(true);
                    }
                }

                let mut _value_read = value_read;
                let mut _value = value;
                for i in 0..RC {
                    read_row.value[i] = F::from_u64(_value_read & RC_MASK);
                    value_row.value[i] = F::from_u64(_value & RC_MASK);
                    _value_read >>= RC_BITS;
                    _value >>= RC_BITS;
                }

                #[rustfmt::skip]
                debug_info!(
                    "\nOne Word Read\n\
                     Num Rows: {:?}\n\
                     Input: {:?}\n\
                     Value Read: {:?}\n\
                     Value: {:?}\n\
                     Flags Read: {:?}\n\
                     Flags Value: {:?}",
                    [*num_rows, *num_rows + 1],
                    input,
                    value_read.to_le_bytes(),
                    value.to_le_bytes(),
                    [
                        read_row.sel[0], read_row.sel[1], read_row.sel[2], read_row.sel[3],
                        read_row.sel[4], read_row.sel[5], read_row.sel[6], read_row.sel[7],
                        read_row.wr, read_row.reset, read_row.sel_up_to_down, read_row.sel_down_to_up
                    ],
                    [
                        value_row.sel[0], value_row.sel[1], value_row.sel[2], value_row.sel[3],
                        value_row.sel[4], value_row.sel[5], value_row.sel[6], value_row.sel[7],
                        value_row.wr, value_row.reset, value_row.sel_up_to_down, value_row.sel_down_to_up
                    ]
                );

                #[cfg(feature = "debug_mem_align")]
                drop(num_rows);

                // Prove the generated rows
                trace[0] = read_row;
                trace[1] = value_row;
                2
            }
            (true, false) => {
                /* RWV with offset=3, width=4
                +----+----+----+====+====+====+====+----+
                | R0 | R1 | R2 | R3 | R4 | R5 | R6 | R7 |
                +----+----+----+====+====+====+====+----+
                                ⇓
                +----+----+----+====+====+====+====+----+
                | W0 | W1 | W2 | W3 | W4 | W5 | W6 | W7 |
                +----+----+----+====+====+====+====+----+
                                ⇓
                +----+----+----+====+====+====+====+----+
                | V5 | V6 | V7 | V0 | V1 | V2 | V3 | V4 |
                +----+----+----+====+====+====+====+----+
                */

                // Unaligned memory op information thrown into the bus
                let step = input.step;
                let value = input.value;

                // Get the aligned address
                let addr_read = addr >> OFFSET_BITS;

                // Get the aligned value
                let value_read = input.mem_values[0];

                // Get the next pc
                let next_pc = self.calculate_next_pc(MemOp::OneWrite, offset, width);

                // Compute the write value
                let value_write = {
                    // with:1 offset:4
                    let width_bytes: u64 = (1 << (width * CHUNK_BITS)) - 1;

                    let mask: u64 = width_bytes << (offset * CHUNK_BITS);

                    // Get the first width bytes of the unaligned value
                    let value_to_write = (value & width_bytes) << (offset * CHUNK_BITS);

                    // Write zeroes to value_read from offset to offset + width
                    // and add the value to write to the value read
                    (value_read & !mask) | value_to_write
                };

                let mut read_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step),
                    addr: F::from_u32(addr_read),
                    // delta_addr: F::ZERO,
                    offset: F::from_u64(DEFAULT_OFFSET),
                    width: F::from_u64(DEFAULT_WIDTH),
                    // wr: F::from_bool(false),
                    // pc: F::from_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut write_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step + 1),
                    addr: F::from_u32(addr_read),
                    // delta_addr: F::ZERO,
                    offset: F::from_u64(DEFAULT_OFFSET),
                    width: F::from_u64(DEFAULT_WIDTH),
                    wr: F::from_bool(true),
                    pc: F::from_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step),
                    addr: F::from_u32(addr_read),
                    // delta_addr: F::ZERO,
                    offset: F::from_usize(offset),
                    width: F::from_usize(width),
                    wr: F::from_bool(true),
                    pc: F::from_u64(next_pc + 1),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNK_NUM {
                    read_row.reg[i] = F::from_u64(Self::get_byte(value_read, i, 0));
                    if i < offset || i >= offset + width {
                        read_row.sel[i] = F::from_bool(true);
                    }

                    write_row.reg[i] = F::from_u64(Self::get_byte(value_write, i, 0));
                    if i >= offset && i < offset + width {
                        write_row.sel[i] = F::from_bool(true);
                    }

                    value_row.reg[i] = {
                        if i >= offset && i < offset + width {
                            write_row.reg[i]
                        } else {
                            F::from_u64(Self::get_byte(value, i, CHUNK_NUM - offset))
                        }
                    };
                    if i == offset {
                        value_row.sel[i] = F::from_bool(true);
                    }
                }

                let mut _value_read = value_read;
                let mut _value_write = value_write;
                let mut _value = value;
                for i in 0..RC {
                    read_row.value[i] = F::from_u64(_value_read & RC_MASK);
                    write_row.value[i] = F::from_u64(_value_write & RC_MASK);
                    value_row.value[i] = F::from_u64(_value & RC_MASK);
                    _value_read >>= RC_BITS;
                    _value_write >>= RC_BITS;
                    _value >>= RC_BITS;
                }

                #[rustfmt::skip]
                debug_info!(
                    "\nOne Word Write\n\
                     Num Rows: {:?}\n\
                     Input: {:?}\n\
                     Value Read: {:?}\n\
                     Value Write: {:?}\n\
                     Value: {:?}\n\
                     Flags Read: {:?}\n\
                     Flags Write: {:?}\n\
                     Flags Value: {:?}",
                    [*num_rows, *num_rows + 2],
                    input,
                    value_read.to_le_bytes(),
                    value_write.to_le_bytes(),
                    value.to_le_bytes(),
                    [
                        read_row.sel[0], read_row.sel[1], read_row.sel[2], read_row.sel[3],
                        read_row.sel[4], read_row.sel[5], read_row.sel[6], read_row.sel[7],
                        read_row.wr, read_row.reset, read_row.sel_up_to_down, read_row.sel_down_to_up
                    ],
                    [
                        write_row.sel[0], write_row.sel[1], write_row.sel[2], write_row.sel[3],
                        write_row.sel[4], write_row.sel[5], write_row.sel[6], write_row.sel[7],
                        write_row.wr, write_row.reset, write_row.sel_up_to_down, write_row.sel_down_to_up
                    ],
                    [
                        value_row.sel[0], value_row.sel[1], value_row.sel[2], value_row.sel[3],
                        value_row.sel[4], value_row.sel[5], value_row.sel[6], value_row.sel[7],
                        value_row.wr, value_row.reset, value_row.sel_up_to_down, value_row.sel_down_to_up
                    ]
                );

                #[cfg(feature = "debug_mem_align")]
                drop(num_rows);

                // Prove the generated rows
                trace[0] = read_row;
                trace[1] = write_row;
                trace[2] = value_row;
                3
            }
            (false, true) => {
                /* RVR with offset=5, width=8
                +----+----+----+----+----+====+====+====+
                | R0 | R1 | R2 | R3 | R4 | R5 | R6 | R7 |
                +----+----+----+----+----+====+====+====+
                                ⇓
                +====+====+====+====+====+====+====+====+
                | V3 | V4 | V5 | V6 | V7 | V0 | V1 | V2 |
                +====+====+====+====+====+====+====+====+
                                ⇓
                +====+====+====+====+====+----+----+----+
                | R0 | R1 | R2 | R3 | R4 | R5 | R6 | R7 |
                +====+====+====+====+====+----+----+----+
                */

                // Unaligned memory op information thrown into the bus
                let step = input.step;
                let value = input.value;

                // Compute the remaining bytes
                let rem_bytes = (offset + width) % CHUNK_NUM;

                // Get the aligned address
                let addr_first_read = addr >> OFFSET_BITS;
                let addr_second_read = addr_first_read + 1;

                // Get the aligned value
                let value_first_read = input.mem_values[0];
                let value_second_read = input.mem_values[1];

                // Get the next pc
                let next_pc = self.calculate_next_pc(MemOp::TwoReads, offset, width);

                let mut first_read_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step),
                    addr: F::from_u32(addr_first_read),
                    // delta_addr: F::ZERO,
                    offset: F::from_u64(DEFAULT_OFFSET),
                    width: F::from_u64(DEFAULT_WIDTH),
                    // wr: F::from_bool(false),
                    // pc: F::from_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step),
                    addr: F::from_u32(addr_first_read),
                    // delta_addr: F::ZERO,
                    offset: F::from_usize(offset),
                    width: F::from_usize(width),
                    // wr: F::from_bool(false),
                    pc: F::from_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                let mut second_read_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step),
                    addr: F::from_u32(addr_second_read),
                    delta_addr: F::ONE,
                    offset: F::from_u64(DEFAULT_OFFSET),
                    width: F::from_u64(DEFAULT_WIDTH),
                    // wr: F::from_bool(false),
                    pc: F::from_u64(next_pc + 1),
                    // reset: F::from_bool(false),
                    sel_down_to_up: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNK_NUM {
                    first_read_row.reg[i] = F::from_u64(Self::get_byte(value_first_read, i, 0));
                    if i >= offset {
                        first_read_row.sel[i] = F::from_bool(true);
                    }

                    value_row.reg[i] = F::from_u64(Self::get_byte(value, i, CHUNK_NUM - offset));

                    if i == offset {
                        value_row.sel[i] = F::from_bool(true);
                    }

                    second_read_row.reg[i] = F::from_u64(Self::get_byte(value_second_read, i, 0));
                    if i < rem_bytes {
                        second_read_row.sel[i] = F::from_bool(true);
                    }
                }

                let mut _value_first_read = value_first_read;
                let mut _value = value;
                let mut _value_second_read = value_second_read;
                for i in 0..RC {
                    first_read_row.value[i] = F::from_u64(_value_first_read & RC_MASK);
                    value_row.value[i] = F::from_u64(_value & RC_MASK);
                    second_read_row.value[i] = F::from_u64(_value_second_read & RC_MASK);
                    _value_first_read >>= RC_BITS;
                    _value >>= RC_BITS;
                    _value_second_read >>= RC_BITS;
                }

                #[rustfmt::skip]
                        debug_info!(
                            "\nTwo Words Read\n\
                             Num Rows: {:?}\n\
                             Input: {:?}\n\
                             Value First Read: {:?}\n\
                             Value: {:?}\n\
                             Value Second Read: {:?}\n\
                             Flags First Read: {:?}\n\
                             Flags Value: {:?}\n\
                             Flags Second Read: {:?}",
                            [*num_rows, *num_rows + 2],
                            input,
                            value_first_read.to_le_bytes(),
                            value.to_le_bytes(),
                            value_second_read.to_le_bytes(),
                            [
                                first_read_row.sel[0], first_read_row.sel[1], first_read_row.sel[2], first_read_row.sel[3],
                                first_read_row.sel[4], first_read_row.sel[5], first_read_row.sel[6], first_read_row.sel[7],
                                first_read_row.wr, first_read_row.reset, first_read_row.sel_up_to_down, first_read_row.sel_down_to_up
                            ],
                            [
                                value_row.sel[0], value_row.sel[1], value_row.sel[2], value_row.sel[3],
                                value_row.sel[4], value_row.sel[5], value_row.sel[6], value_row.sel[7],
                                value_row.wr, value_row.reset, value_row.sel_up_to_down, value_row.sel_down_to_up
                            ],
                            [
                                second_read_row.sel[0], second_read_row.sel[1], second_read_row.sel[2], second_read_row.sel[3],
                                second_read_row.sel[4], second_read_row.sel[5], second_read_row.sel[6], second_read_row.sel[7],
                                second_read_row.wr, second_read_row.reset, second_read_row.sel_up_to_down, second_read_row.sel_down_to_up
                            ]
                        );

                #[cfg(feature = "debug_mem_align")]
                drop(num_rows);

                // Prove the generated rows
                trace[0] = first_read_row;
                trace[1] = value_row;
                trace[2] = second_read_row;
                3
            }
            (true, true) => {
                /* RWVWR with offset=6, width=4
                +----+----+----+----+----+----+====+====+
                | R0 | R1 | R2 | R3 | R4 | R5 | R6 | R7 |
                +----+----+----+----+----+----+====+====+
                                ⇓
                +----+----+----+----+----+----+====+====+
                | W0 | W1 | W2 | W3 | W4 | W5 | W6 | W7 |
                +----+----+----+----+----+----+====+====+
                                ⇓
                +====+====+----+----+----+----+====+====+
                | V2 | V3 | V4 | V5 | V6 | V7 | V0 | V1 |
                +====+====+----+----+----+----+====+====+
                                ⇓
                +====+====+----+----+----+----+----+----+
                | W0 | W1 | W2 | W3 | W4 | W5 | W6 | W7 |
                +====+====+----+----+----+----+----+----+
                                ⇓
                +====+====+----+----+----+----+----+----+
                | R0 | R1 | R2 | R3 | R4 | R5 | R6 | R7 |
                +====+====+----+----+----+----+----+----+
                */
                // Unaligned memory op information thrown into the bus
                let step = input.step;
                let value = input.value;

                // Compute the shift
                let rem_bytes = (offset + width) % CHUNK_NUM;

                // Get the aligned address
                let addr_first_read_write = addr >> OFFSET_BITS;
                let addr_second_read_write = addr_first_read_write + 1;

                // Get the first aligned value
                let value_first_read = input.mem_values[0];

                // Recompute the first write value
                let value_first_write = {
                    // Normalize the width
                    let width_norm = CHUNK_NUM - offset;

                    let width_bytes: u64 = (1 << (width_norm * CHUNK_BITS)) - 1;

                    let mask: u64 = width_bytes << (offset * CHUNK_BITS);

                    // Get the first width bytes of the unaligned value
                    let value_to_write = (value & width_bytes) << (offset * CHUNK_BITS);

                    // Write zeroes to value_read from offset to offset + width
                    // and add the value to write to the value read
                    (value_first_read & !mask) | value_to_write
                };

                // Get the second aligned value
                let value_second_read = input.mem_values[1];

                // Compute the second write value
                let value_second_write = {
                    // Normalize the width
                    let width_norm = CHUNK_NUM - offset;

                    let mask: u64 = (1 << (rem_bytes * CHUNK_BITS)) - 1;

                    // Get the first width bytes of the unaligned value
                    let value_to_write = (value >> (width_norm * CHUNK_BITS)) & mask;

                    // Write zeroes to value_read from 0 to offset + width
                    // and add the value to write to the value read
                    (value_second_read & !mask) | value_to_write
                };

                // Get the next pc
                let next_pc = self.calculate_next_pc(MemOp::TwoWrites, offset, width);

                // RWVWR
                let mut first_read_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step),
                    addr: F::from_u32(addr_first_read_write),
                    // delta_addr: F::ZERO,
                    offset: F::from_u64(DEFAULT_OFFSET),
                    width: F::from_u64(DEFAULT_WIDTH),
                    // wr: F::from_bool(false),
                    // pc: F::from_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut first_write_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step + 1),
                    addr: F::from_u32(addr_first_read_write),
                    // delta_addr: F::ZERO,
                    offset: F::from_u64(DEFAULT_OFFSET),
                    width: F::from_u64(DEFAULT_WIDTH),
                    wr: F::from_bool(true),
                    pc: F::from_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step),
                    addr: F::from_u32(addr_first_read_write),
                    // delta_addr: F::ZERO,
                    offset: F::from_usize(offset),
                    width: F::from_usize(width),
                    wr: F::from_bool(true),
                    pc: F::from_u64(next_pc + 1),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                let mut second_write_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step + 1),
                    addr: F::from_u32(addr_second_read_write),
                    delta_addr: F::ONE,
                    offset: F::from_u64(DEFAULT_OFFSET),
                    width: F::from_u64(DEFAULT_WIDTH),
                    wr: F::from_bool(true),
                    pc: F::from_u64(next_pc + 2),
                    // reset: F::from_bool(false),
                    sel_down_to_up: F::from_bool(true),
                    ..Default::default()
                };

                let mut second_read_row = MemAlignTraceRow::<F> {
                    step: F::from_u64(step),
                    addr: F::from_u32(addr_second_read_write),
                    // delta_addr: F::ZERO,
                    offset: F::from_u64(DEFAULT_OFFSET),
                    width: F::from_u64(DEFAULT_WIDTH),
                    // wr: F::from_bool(false),
                    pc: F::from_u64(next_pc + 3),
                    reset: F::from_bool(false),
                    sel_down_to_up: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNK_NUM {
                    first_read_row.reg[i] = F::from_u64(Self::get_byte(value_first_read, i, 0));
                    if i < offset {
                        first_read_row.sel[i] = F::from_bool(true);
                    }

                    first_write_row.reg[i] = F::from_u64(Self::get_byte(value_first_write, i, 0));
                    if i >= offset {
                        first_write_row.sel[i] = F::from_bool(true);
                    }

                    value_row.reg[i] = {
                        if i < rem_bytes {
                            second_write_row.reg[i]
                        } else if i >= offset {
                            first_write_row.reg[i]
                        } else {
                            F::from_u64(Self::get_byte(value, i, CHUNK_NUM - offset))
                        }
                    };
                    if i == offset {
                        value_row.sel[i] = F::from_bool(true);
                    }

                    second_write_row.reg[i] = F::from_u64(Self::get_byte(value_second_write, i, 0));
                    if i < rem_bytes {
                        second_write_row.sel[i] = F::from_bool(true);
                    }

                    second_read_row.reg[i] = F::from_u64(Self::get_byte(value_second_read, i, 0));
                    if i >= rem_bytes {
                        second_read_row.sel[i] = F::from_bool(true);
                    }
                }

                let mut _value_first_read = value_first_read;
                let mut _value_first_write = value_first_write;
                let mut _value = value;
                let mut _value_second_write = value_second_write;
                let mut _value_second_read = value_second_read;
                for i in 0..RC {
                    first_read_row.value[i] = F::from_u64(_value_first_read & RC_MASK);
                    first_write_row.value[i] = F::from_u64(_value_first_write & RC_MASK);
                    value_row.value[i] = F::from_u64(_value & RC_MASK);
                    second_write_row.value[i] = F::from_u64(_value_second_write & RC_MASK);
                    second_read_row.value[i] = F::from_u64(_value_second_read & RC_MASK);
                    _value_first_read >>= RC_BITS;
                    _value_first_write >>= RC_BITS;
                    _value >>= RC_BITS;
                    _value_second_write >>= RC_BITS;
                    _value_second_read >>= RC_BITS;
                }

                #[rustfmt::skip]
                        debug_info!(
                            "\nTwo Words Write\n\
                             Num Rows: {:?}\n\
                             Input: {:?}\n\
                             Value First Read: {:?}\n\
                             Value First Write: {:?}\n\
                             Value: {:?}\n\
                             Value Second Read: {:?}\n\
                             Value Second Write: {:?}\n\
                             Flags First Read: {:?}\n\
                             Flags First Write: {:?}\n\
                             Flags Value: {:?}\n\
                             Flags Second Write: {:?}\n\
                             Flags Second Read: {:?}",
                            [*num_rows, *num_rows + 4],
                            input,
                            value_first_read.to_le_bytes(),
                            value_first_write.to_le_bytes(),
                            value.to_le_bytes(),
                            value_second_write.to_le_bytes(),
                            value_second_read.to_le_bytes(),
                            [
                                first_read_row.sel[0], first_read_row.sel[1], first_read_row.sel[2], first_read_row.sel[3],
                                first_read_row.sel[4], first_read_row.sel[5], first_read_row.sel[6], first_read_row.sel[7],
                                first_read_row.wr, first_read_row.reset, first_read_row.sel_up_to_down, first_read_row.sel_down_to_up
                            ],
                            [
                                first_write_row.sel[0], first_write_row.sel[1], first_write_row.sel[2], first_write_row.sel[3],
                                first_write_row.sel[4], first_write_row.sel[5], first_write_row.sel[6], first_write_row.sel[7],
                                first_write_row.wr, first_write_row.reset, first_write_row.sel_up_to_down, first_write_row.sel_down_to_up
                            ],
                            [
                                value_row.sel[0], value_row.sel[1], value_row.sel[2], value_row.sel[3],
                                value_row.sel[4], value_row.sel[5], value_row.sel[6], value_row.sel[7],
                                value_row.wr, value_row.reset, value_row.sel_up_to_down, value_row.sel_down_to_up
                            ],
                            [
                                second_write_row.sel[0], second_write_row.sel[1], second_write_row.sel[2], second_write_row.sel[3],
                                second_write_row.sel[4], second_write_row.sel[5], second_write_row.sel[6], second_write_row.sel[7],
                                second_write_row.wr, second_write_row.reset, second_write_row.sel_up_to_down, second_write_row.sel_down_to_up
                            ],
                            [
                                second_read_row.sel[0], second_read_row.sel[1], second_read_row.sel[2], second_read_row.sel[3],
                                second_read_row.sel[4], second_read_row.sel[5], second_read_row.sel[6], second_read_row.sel[7],
                                second_read_row.wr, second_read_row.reset, second_read_row.sel_up_to_down, second_read_row.sel_down_to_up
                            ]
                        );

                #[cfg(feature = "debug_mem_align")]
                drop(num_rows);

                // Prove the generated rows
                trace[0] = first_read_row;
                trace[1] = first_write_row;
                trace[2] = value_row;
                trace[3] = second_write_row;
                trace[4] = second_read_row;
                5
            }
        }
    }

    fn get_byte(value: u64, index: usize, offset: usize) -> u64 {
        let chunk = (offset + index) % CHUNK_NUM;
        (value >> (chunk * CHUNK_BITS)) & CHUNK_BITS_MASK
    }

    pub fn compute_witness(
        &self,
        inputs: &[Vec<MemAlignInput>],
        trace_buffer: Option<Vec<F>>,
    ) -> AirInstance<F> {
        let mut trace = if let Some(buffer) = trace_buffer {
            tracing::trace!("··· Using provided trace buffer");
            MemAlignTrace::new_from_vec(buffer)
        } else {
            tracing::trace!("··· Creating new trace buffer");
            MemAlignTrace::new()
        };

        let mut reg_range_check = vec![0u32; 1 << CHUNK_BITS];

        let num_rows = trace.num_rows();

        let mut trace_rows = trace.row_slice_mut();
        let mut par_traces = Vec::new();
        let mut inputs_indexes = Vec::new();
        let mut total_index = 0;
        for (i, inner_memp_ops) in inputs.iter().enumerate() {
            for (j, input) in inner_memp_ops.iter().enumerate() {
                let addr = input.addr;
                let width = input.width as usize;
                let offset = (addr & OFFSET_MASK) as usize;
                let n_rows = match (input.is_write, offset + width > CHUNK_NUM) {
                    (false, false) => 2,
                    (true, false) => 3,
                    (false, true) => 3,
                    (true, true) => 5,
                };
                total_index += n_rows;
                let (head, tail) = trace_rows.split_at_mut(n_rows);
                par_traces.push(head);
                inputs_indexes.push((i, j));
                trace_rows = tail;
            }
        }

        // Prove the memory operations in parallel
        par_traces.into_par_iter().enumerate().for_each(|(index, trace)| {
            let input_index = inputs_indexes[index];
            let input = &inputs[input_index.0][input_index.1];
            self.prove_mem_align_op(input, trace);
        });

        // Iterate over all traces to set range checks
        trace.row_slice_mut()[0..total_index].iter_mut().for_each(|row| {
            for j in 0..CHUNK_NUM {
                let element = row.reg[j].as_canonical_u64() as usize;
                reg_range_check[element] += 1;
            }
        });

        let padding_size = num_rows - total_index;
        let padding_row = MemAlignTraceRow::<F> { reset: F::from_bool(true), ..Default::default() };

        // Store the padding rows
        trace.row_slice_mut()[total_index..num_rows]
            .par_iter_mut()
            .for_each(|slot| *slot = padding_row);

        // Compute the program multiplicity
        let mem_align_rom_sm = self.mem_align_rom_sm.clone();
        mem_align_rom_sm.update_padding_row(padding_size as u64);

        reg_range_check[0] += CHUNK_NUM as u32 * padding_size as u32;
        self.update_std_range_check(reg_range_check);

        AirInstance::new_from_trace(FromTrace::new(&mut trace))
    }

    fn update_std_range_check(&self, reg_range_check: Vec<u32>) {
        // Perform the range checks
        let range_id = self.std.get_range(0, CHUNK_BITS_MASK as i64, None);
        self.std.range_checks(reg_range_check, range_id);
    }
}
