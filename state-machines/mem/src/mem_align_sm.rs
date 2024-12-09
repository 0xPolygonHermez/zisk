use core::panic;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use log::info;
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;

use zisk_pil::{MemAlignRow, MemAlignTrace, MEM_ALIGN_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::{MemAlignInput, MemAlignRomSM, MemOp};

const RC: usize = 2;
const CHUNK_NUM: usize = 8;
const CHUNKS_BY_RC: usize = CHUNK_NUM / RC;
const CHUNK_BITS: usize = 8;
const RC_BITS: u64 = (CHUNKS_BY_RC * CHUNK_BITS) as u64;
const RC_MASK: u64 = (1 << RC_BITS) - 1;
const OFFSET_MASK: u32 = 0x07;
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

pub struct MemAlignResponse {
    pub more_addr: bool,
    pub step: u64,
    pub value: Option<u64>,
}
pub struct MemAlignSM<F: PrimeField> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    // STD
    std: Arc<Std<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Computed row information
    rows: Mutex<Vec<MemAlignRow<F>>>,
    #[cfg(feature = "debug_mem_align")]
    num_computed_rows: Mutex<usize>,

    // Secondary State machines
    mem_align_rom_sm: Arc<MemAlignRomSM<F>>,
}

macro_rules! debug_info {
    ($prefix:expr, $($arg:tt)*) => {
        #[cfg(feature = "debug_mem_align")]
        {
            info!(concat!("MemAlign: ",$prefix), $($arg)*);
        }
    };
}

impl<F: PrimeField> MemAlignSM<F> {
    const MY_NAME: &'static str = "MemAlign";

    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        std: Arc<Std<F>>,
        mem_align_rom_sm: Arc<MemAlignRomSM<F>>,
    ) -> Arc<Self> {
        let mem_align_sm = Self {
            wcm: wcm.clone(),
            std: std.clone(),
            registered_predecessors: AtomicU32::new(0),
            rows: Mutex::new(Vec::new()),
            #[cfg(feature = "debug_mem_align")]
            num_computed_rows: Mutex::new(0),
            mem_align_rom_sm,
        };
        let mem_align_sm = Arc::new(mem_align_sm);

        wcm.register_component(
            mem_align_sm.clone(),
            Some(ZISK_AIRGROUP_ID),
            Some(MEM_ALIGN_AIR_IDS),
        );

        // Register the predecessors
        std.register_predecessor();
        mem_align_sm.mem_align_rom_sm.register_predecessor();

        mem_align_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            let pctx = self.wcm.get_pctx();

            // If there are remaining rows, generate the last instance
            if let Ok(mut rows) = self.rows.lock() {
                // Get the Mem Align AIR
                let air_mem_align = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_ALIGN_AIR_IDS[0]);

                let rows_len = rows.len();
                debug_assert!(rows_len <= air_mem_align.num_rows());

                let drained_rows = rows.drain(..rows_len).collect::<Vec<_>>();

                self.fill_new_air_instance(&drained_rows);
            }

            self.mem_align_rom_sm.unregister_predecessor();
            self.std.unregister_predecessor(pctx, None);
        }
    }

    #[inline(always)]
    pub fn get_mem_op(&self, input: &MemAlignInput, phase: usize) -> MemAlignResponse {
        let addr = input.addr;
        let width = input.width;

        // Compute the width
        debug_assert!(
            ALLOWED_WIDTHS.contains(&width),
            "Width={} is not allowed. Allowed widths are {:?}",
            width,
            ALLOWED_WIDTHS
        );
        let width = width as usize;

        // Compute the offset
        let offset = (addr & OFFSET_MASK) as u8;
        debug_assert!(
            ALLOWED_OFFSETS.contains(&offset),
            "Offset={} is not allowed. Allowed offsets are {:?}",
            offset,
            ALLOWED_OFFSETS
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
                debug_assert!(phase == 0);

                // Unaligned memory op information thrown into the bus
                let step = input.step;
                let value = input.value;

                // Get the aligned address
                let addr_read = addr >> OFFSET_BITS;

                // Get the aligned value
                let value_read = input.mem_values[phase];

                // Get the next pc
                let next_pc =
                    self.mem_align_rom_sm.calculate_next_pc(MemOp::OneRead, offset, width);

                let mut read_row = MemAlignRow::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u32(addr_read),
                    // delta_addr: F::zero(),
                    offset: F::from_canonical_u64(DEFAULT_OFFSET),
                    width: F::from_canonical_u64(DEFAULT_WIDTH),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlignRow::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u32(addr_read),
                    // delta_addr: F::zero(),
                    offset: F::from_canonical_usize(offset),
                    width: F::from_canonical_usize(width),
                    // wr: F::from_bool(false),
                    pc: F::from_canonical_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNK_NUM {
                    read_row.reg[i] = F::from_canonical_u64(Self::get_byte(value_read, i, 0));
                    if i >= offset && i < offset + width {
                        read_row.sel[i] = F::from_bool(true);
                    }

                    value_row.reg[i] =
                        F::from_canonical_u64(Self::get_byte(value, i, CHUNK_NUM - offset));
                    if i == offset {
                        value_row.sel[i] = F::from_bool(true);
                    }
                }

                let mut _value_read = value_read;
                let mut _value = value;
                for i in 0..RC {
                    read_row.value[i] = F::from_canonical_u64(_value_read & RC_MASK);
                    value_row.value[i] = F::from_canonical_u64(_value & RC_MASK);
                    _value_read >>= RC_BITS;
                    _value >>= RC_BITS;
                }

                #[rustfmt::skip]
                debug_info!(
                    "\nOne Word Read\n\
                     Num Rows: {:?}\n\
                     Input: {:?}\n\
                     Phase: {:?}\n\
                     Value Read: {:?}\n\
                     Value: {:?}\n\
                     Flags Read: {:?}\n\
                     Flags Value: {:?}",
                    [*num_rows, *num_rows + 1],
                    input,
                    phase,
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
                self.prove(&[read_row, value_row]);

                MemAlignResponse { more_addr: false, step, value: None }
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
                debug_assert!(phase == 0);

                // Unaligned memory op information thrown into the bus
                let step = input.step;
                let value = input.value;

                // Get the aligned address
                let addr_read = addr >> OFFSET_BITS;

                // Get the aligned value
                let value_read = input.mem_values[phase];

                // Get the next pc
                let next_pc =
                    self.mem_align_rom_sm.calculate_next_pc(MemOp::OneWrite, offset, width);

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

                let mut read_row = MemAlignRow::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u32(addr_read),
                    // delta_addr: F::zero(),
                    offset: F::from_canonical_u64(DEFAULT_OFFSET),
                    width: F::from_canonical_u64(DEFAULT_WIDTH),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut write_row = MemAlignRow::<F> {
                    step: F::from_canonical_u64(step + 1),
                    addr: F::from_canonical_u32(addr_read),
                    // delta_addr: F::zero(),
                    offset: F::from_canonical_u64(DEFAULT_OFFSET),
                    width: F::from_canonical_u64(DEFAULT_WIDTH),
                    wr: F::from_bool(true),
                    pc: F::from_canonical_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlignRow::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u32(addr_read),
                    // delta_addr: F::zero(),
                    offset: F::from_canonical_usize(offset),
                    width: F::from_canonical_usize(width),
                    wr: F::from_bool(true),
                    pc: F::from_canonical_u64(next_pc + 1),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNK_NUM {
                    read_row.reg[i] = F::from_canonical_u64(Self::get_byte(value_read, i, 0));
                    if i < offset || i >= offset + width {
                        read_row.sel[i] = F::from_bool(true);
                    }

                    write_row.reg[i] = F::from_canonical_u64(Self::get_byte(value_write, i, 0));
                    if i >= offset && i < offset + width {
                        write_row.sel[i] = F::from_bool(true);
                    }

                    value_row.reg[i] = {
                        if i >= offset && i < offset + width {
                            write_row.reg[i]
                        } else {
                            F::from_canonical_u64(Self::get_byte(value, i, CHUNK_NUM - offset))
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
                    read_row.value[i] = F::from_canonical_u64(_value_read & RC_MASK);
                    write_row.value[i] = F::from_canonical_u64(_value_write & RC_MASK);
                    value_row.value[i] = F::from_canonical_u64(_value & RC_MASK);
                    _value_read >>= RC_BITS;
                    _value_write >>= RC_BITS;
                    _value >>= RC_BITS;
                }

                #[rustfmt::skip]
                debug_info!(
                    "\nOne Word Write\n\
                     Num Rows: {:?}\n\
                     Input: {:?}\n\
                     Phase: {:?}\n\
                     Value Read: {:?}\n\
                     Value Write: {:?}\n\
                     Value: {:?}\n\
                     Flags Read: {:?}\n\
                     Flags Write: {:?}\n\
                     Flags Value: {:?}",
                    [*num_rows, *num_rows + 2],
                    input,
                    phase,
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
                self.prove(&[read_row, write_row, value_row]);

                MemAlignResponse { more_addr: false, step, value: Some(value_write) }
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
                debug_assert!(phase == 0 || phase == 1);

                match phase {
                    // If phase == 0, do nothing, just ask for more
                    0 => MemAlignResponse { more_addr: true, step: input.step, value: None },

                    // Otherwise, do the RVR
                    1 => {
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
                        let next_pc =
                            self.mem_align_rom_sm.calculate_next_pc(MemOp::TwoReads, offset, width);

                        let mut first_read_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step),
                            addr: F::from_canonical_u32(addr_first_read),
                            // delta_addr: F::zero(),
                            offset: F::from_canonical_u64(DEFAULT_OFFSET),
                            width: F::from_canonical_u64(DEFAULT_WIDTH),
                            // wr: F::from_bool(false),
                            // pc: F::from_canonical_u64(0),
                            reset: F::from_bool(true),
                            sel_up_to_down: F::from_bool(true),
                            ..Default::default()
                        };

                        let mut value_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step),
                            addr: F::from_canonical_u32(addr_first_read),
                            // delta_addr: F::zero(),
                            offset: F::from_canonical_usize(offset),
                            width: F::from_canonical_usize(width),
                            // wr: F::from_bool(false),
                            pc: F::from_canonical_u64(next_pc),
                            // reset: F::from_bool(false),
                            sel_prove: F::from_bool(true),
                            ..Default::default()
                        };

                        let mut second_read_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step),
                            addr: F::from_canonical_u32(addr_second_read),
                            delta_addr: F::one(),
                            offset: F::from_canonical_u64(DEFAULT_OFFSET),
                            width: F::from_canonical_u64(DEFAULT_WIDTH),
                            // wr: F::from_bool(false),
                            pc: F::from_canonical_u64(next_pc + 1),
                            // reset: F::from_bool(false),
                            sel_down_to_up: F::from_bool(true),
                            ..Default::default()
                        };

                        for i in 0..CHUNK_NUM {
                            first_read_row.reg[i] =
                                F::from_canonical_u64(Self::get_byte(value_first_read, i, 0));
                            if i >= offset {
                                first_read_row.sel[i] = F::from_bool(true);
                            }

                            value_row.reg[i] =
                                F::from_canonical_u64(Self::get_byte(value, i, CHUNK_NUM - offset));

                            if i == offset {
                                value_row.sel[i] = F::from_bool(true);
                            }

                            second_read_row.reg[i] =
                                F::from_canonical_u64(Self::get_byte(value_second_read, i, 0));
                            if i < rem_bytes {
                                second_read_row.sel[i] = F::from_bool(true);
                            }
                        }

                        let mut _value_first_read = value_first_read;
                        let mut _value = value;
                        let mut _value_second_read = value_second_read;
                        for i in 0..RC {
                            first_read_row.value[i] =
                                F::from_canonical_u64(_value_first_read & RC_MASK);
                            value_row.value[i] = F::from_canonical_u64(_value & RC_MASK);
                            second_read_row.value[i] =
                                F::from_canonical_u64(_value_second_read & RC_MASK);
                            _value_first_read >>= RC_BITS;
                            _value >>= RC_BITS;
                            _value_second_read >>= RC_BITS;
                        }

                        #[rustfmt::skip]
                        debug_info!(
                            "\nTwo Words Read\n\
                             Num Rows: {:?}\n\
                             Input: {:?}\n\
                             Phase: {:?}\n\
                             Value First Read: {:?}\n\
                             Value: {:?}\n\
                             Value Second Read: {:?}\n\
                             Flags First Read: {:?}\n\
                             Flags Value: {:?}\n\
                             Flags Second Read: {:?}",
                            [*num_rows, *num_rows + 2],
                            input,
                            phase,
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
                        self.prove(&[first_read_row, value_row, second_read_row]);

                        MemAlignResponse { more_addr: false, step, value: None }
                    }
                    _ => panic!("Invalid phase={}", phase),
                }
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
                debug_assert!(phase == 0 || phase == 1);

                match phase {
                    // If phase == 0, compute the resulting write value and ask for more
                    0 => {
                        // Unaligned memory op information thrown into the bus
                        let value = input.value;
                        let step = input.step;

                        // Get the aligned value
                        let value_first_read = input.mem_values[0];

                        // Compute the write value
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

                        MemAlignResponse { more_addr: true, step, value: Some(value_first_write) }
                    }
                    // Otherwise, do the RWVRW
                    1 => {
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
                        let next_pc = self.mem_align_rom_sm.calculate_next_pc(
                            MemOp::TwoWrites,
                            offset,
                            width,
                        );

                        // RWVWR
                        let mut first_read_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step),
                            addr: F::from_canonical_u32(addr_first_read_write),
                            // delta_addr: F::zero(),
                            offset: F::from_canonical_u64(DEFAULT_OFFSET),
                            width: F::from_canonical_u64(DEFAULT_WIDTH),
                            // wr: F::from_bool(false),
                            // pc: F::from_canonical_u64(0),
                            reset: F::from_bool(true),
                            sel_up_to_down: F::from_bool(true),
                            ..Default::default()
                        };

                        let mut first_write_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step + 1),
                            addr: F::from_canonical_u32(addr_first_read_write),
                            // delta_addr: F::zero(),
                            offset: F::from_canonical_u64(DEFAULT_OFFSET),
                            width: F::from_canonical_u64(DEFAULT_WIDTH),
                            wr: F::from_bool(true),
                            pc: F::from_canonical_u64(next_pc),
                            // reset: F::from_bool(false),
                            sel_up_to_down: F::from_bool(true),
                            ..Default::default()
                        };

                        let mut value_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step),
                            addr: F::from_canonical_u32(addr_first_read_write),
                            // delta_addr: F::zero(),
                            offset: F::from_canonical_usize(offset),
                            width: F::from_canonical_usize(width),
                            wr: F::from_bool(true),
                            pc: F::from_canonical_u64(next_pc + 1),
                            // reset: F::from_bool(false),
                            sel_prove: F::from_bool(true),
                            ..Default::default()
                        };

                        let mut second_write_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step + 1),
                            addr: F::from_canonical_u32(addr_second_read_write),
                            delta_addr: F::one(),
                            offset: F::from_canonical_u64(DEFAULT_OFFSET),
                            width: F::from_canonical_u64(DEFAULT_WIDTH),
                            wr: F::from_bool(true),
                            pc: F::from_canonical_u64(next_pc + 2),
                            // reset: F::from_bool(false),
                            sel_down_to_up: F::from_bool(true),
                            ..Default::default()
                        };

                        let mut second_read_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step),
                            addr: F::from_canonical_u32(addr_second_read_write),
                            // delta_addr: F::zero(),
                            offset: F::from_canonical_u64(DEFAULT_OFFSET),
                            width: F::from_canonical_u64(DEFAULT_WIDTH),
                            // wr: F::from_bool(false),
                            pc: F::from_canonical_u64(next_pc + 3),
                            reset: F::from_bool(false),
                            sel_down_to_up: F::from_bool(true),
                            ..Default::default()
                        };

                        for i in 0..CHUNK_NUM {
                            first_read_row.reg[i] =
                                F::from_canonical_u64(Self::get_byte(value_first_read, i, 0));
                            if i < offset {
                                first_read_row.sel[i] = F::from_bool(true);
                            }

                            first_write_row.reg[i] =
                                F::from_canonical_u64(Self::get_byte(value_first_write, i, 0));
                            if i >= offset {
                                first_write_row.sel[i] = F::from_bool(true);
                            }

                            value_row.reg[i] = {
                                if i < rem_bytes {
                                    second_write_row.reg[i]
                                } else if i >= offset {
                                    first_write_row.reg[i]
                                } else {
                                    F::from_canonical_u64(Self::get_byte(
                                        value,
                                        i,
                                        CHUNK_NUM - offset,
                                    ))
                                }
                            };
                            if i == offset {
                                value_row.sel[i] = F::from_bool(true);
                            }

                            second_write_row.reg[i] =
                                F::from_canonical_u64(Self::get_byte(value_second_write, i, 0));
                            if i < rem_bytes {
                                second_write_row.sel[i] = F::from_bool(true);
                            }

                            second_read_row.reg[i] =
                                F::from_canonical_u64(Self::get_byte(value_second_read, i, 0));
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
                            first_read_row.value[i] =
                                F::from_canonical_u64(_value_first_read & RC_MASK);
                            first_write_row.value[i] =
                                F::from_canonical_u64(_value_first_write & RC_MASK);
                            value_row.value[i] = F::from_canonical_u64(_value & RC_MASK);
                            second_write_row.value[i] =
                                F::from_canonical_u64(_value_second_write & RC_MASK);
                            second_read_row.value[i] =
                                F::from_canonical_u64(_value_second_read & RC_MASK);
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
                             Phase: {:?}\n\
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
                            phase,
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
                        self.prove(&[
                            first_read_row,
                            first_write_row,
                            value_row,
                            second_write_row,
                            second_read_row,
                        ]);

                        MemAlignResponse { more_addr: false, step, value: Some(value_second_write) }
                    }
                    _ => panic!("Invalid phase={}", phase),
                }
            }
        }
    }

    fn get_byte(value: u64, index: usize, offset: usize) -> u64 {
        let chunk = (offset + index) % CHUNK_NUM;
        (value >> (chunk * CHUNK_BITS)) & CHUNK_BITS_MASK
    }

    pub fn prove(&self, computed_rows: &[MemAlignRow<F>]) {
        if let Ok(mut rows) = self.rows.lock() {
            rows.extend_from_slice(computed_rows);

            #[cfg(feature = "debug_mem_align")]
            {
                let mut num_rows = self.num_computed_rows.lock().unwrap();
                *num_rows += computed_rows.len();
                drop(num_rows);
            }

            let pctx = self.wcm.get_pctx();
            let air_mem_align = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_ALIGN_AIR_IDS[0]);

            while rows.len() >= air_mem_align.num_rows() {
                let num_drained = std::cmp::min(air_mem_align.num_rows(), rows.len());
                let drained_rows = rows.drain(..num_drained).collect::<Vec<_>>();

                self.fill_new_air_instance(&drained_rows);
            }
        }
    }

    fn fill_new_air_instance(&self, rows: &[MemAlignRow<F>]) {
        // Get the proof context
        let wcm = self.wcm.clone();
        let pctx = wcm.get_pctx();

        // Get the Mem Align AIR
        let air_mem_align = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_ALIGN_AIR_IDS[0]);
        let air_mem_align_rows = air_mem_align.num_rows();
        let rows_len = rows.len();

        // You cannot feed to the AIR more rows than it has
        debug_assert!(rows_len <= air_mem_align_rows);

        // Get the execution and setup context
        let sctx = wcm.get_sctx();

        let mut trace_buffer: MemAlignTrace<'_, _> = MemAlignTrace::new(air_mem_align_rows);

        let mut reg_range_check: Vec<u64> = vec![0; 1 << CHUNK_BITS];
        println!("ROW 0 mem_align {:?}", rows[0]);
        println!("ROW 1 mem_align {:?}", rows[1]);
        // Add the input rows to the trace
        for (i, &row) in rows.iter().enumerate() {
            // Store the entire row
            trace_buffer[i] = row;
            // Store the value of all reg columns so that they can be range checked
            for j in 0..CHUNK_NUM {
                let element =
                    row.reg[j].as_canonical_biguint().to_usize().expect("Cannot convert to usize");
                reg_range_check[element] += 1;
            }
            if i < 2 {
                println!("ROW_{} mem_align {:?}", i, trace_buffer[i]);
            }
        }

        // Pad the remaining rows with trivially satisfying rows
        let padding_row = MemAlignRow::<F> { reset: F::from_bool(true), ..Default::default() };
        let padding_size = air_mem_align_rows - rows_len;

        println!("ROW PADDING mem_align {:?}", padding_row);

        // Store the padding rows
        for i in rows_len..air_mem_align_rows {
            trace_buffer[i] = padding_row;
        }

        // Store the value of all padding reg columns so that they can be range checked
        for _ in 0..CHUNK_NUM {
            reg_range_check[0] += padding_size as u64;
        }

        // Perform the range checks
        let std = self.std.clone();
        let range_id = std.get_range(BigInt::from(0), BigInt::from(CHUNK_BITS_MASK), None);
        for (value, &multiplicity) in reg_range_check.iter().enumerate() {
            std.range_check(
                F::from_canonical_usize(value),
                F::from_canonical_u64(multiplicity),
                range_id,
            );
        }

        // Compute the program multiplicity
        let mem_align_rom_sm = self.mem_align_rom_sm.clone();
        mem_align_rom_sm.update_padding_row(padding_size as u64);

        info!(
            "{}: ··· Creating Mem Align instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            rows_len,
            air_mem_align_rows,
            rows_len as f64 / air_mem_align_rows as f64 * 100.0
        );

        // Add a new Mem Align instance
        let air_instance = AirInstance::new(
            sctx,
            ZISK_AIRGROUP_ID,
            MEM_ALIGN_AIR_IDS[0],
            None,
            trace_buffer.buffer.unwrap(),
        );
        pctx.air_instance_repo.add_air_instance(air_instance, None);
    }
}

impl<F: PrimeField> WitnessComponent<F> for MemAlignSM<F> {}
