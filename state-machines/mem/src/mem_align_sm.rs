use core::panic;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

use log::info;
use num_bigint::BigInt;
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;

use sm_common::create_prover_buffer;
use zisk_core::ZiskRequiredMemory;
use zisk_pil::{MemAlignRow, MemAlignTrace, MEM_ALIGN_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::{MemAlignRomSM, MemOp};

const CHUNK_NUM: usize = 8;
const CHUNK_NUM_U64: u64 = CHUNK_NUM as u64;
const CHUNK_BITS: usize = 8;
const CHUNK_BITS_U64: u64 = CHUNK_BITS as u64;
const CHUNK_BITS_MASK: u64 = (1 << CHUNK_BITS) - 1;

const ALLOWED_WIDTHS: [u64; 4] = [1, 2, 4, 8];

pub struct MemAlignResponse {
    pub more_address: bool,
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

    // Computed rows
    rows: Mutex<Vec<MemAlignRow<F>>>,

    // Secondary State machines
    mem_align_rom_sm: Arc<MemAlignRomSM<F>>,
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

            // TODO: Fix this...
            // If there are remaining rows, generate the last instance
            if let Ok(mut rows) = self.rows.lock() {
                // Get the Mem Align AIR
                let air_mem_align = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_ALIGN_AIR_IDS[0]);

                let rows_len = rows.len();
                assert!(rows_len <= air_mem_align.num_rows());

                let drained_rows = rows.drain(..rows_len).collect::<Vec<_>>();

                self.fill_new_air_instance(&drained_rows);
            }

            self.mem_align_rom_sm.unregister_predecessor();
            self.std.unregister_predecessor(pctx, None);
        }
    }

    #[inline(always)]
    pub fn get_mem_op(
        &self,
        input: &ZiskRequiredMemory,
        mem_values: Vec<u64>,
        phase: usize,
    ) -> MemAlignResponse {
        // Sanity check
        assert!(mem_values.len() == phase + 1); // TODO: Debug mode

        let addr = input.address;
        let width = input.width;
        let width = if ALLOWED_WIDTHS.contains(&width) {
            width as usize
        } else {
            panic!("Width={} is not allowed. Allowed widths are {:?}", width, ALLOWED_WIDTHS);
        };

        // Compute the offset
        let offset = addr & CHUNK_BITS_MASK;
        let offset = if offset <= usize::MAX as u64 {
            offset as usize
        } else {
            panic!("Offset={} is too large", offset);
        };

        // main:      [mem_op, addr, 1 + MAX_MEM_OPS_PER_MAIN_STEP * step + 2 * step_offset, bytes, ...value]
        // mem:       [wr * (MEMORY_STORE_OP - MEMORY_LOAD_OP) + MEMORY_LOAD_OP, addr * MEM_BYTES, step, MEM_BYTES, ...value]
        // mem_align: [wr * (MEMORY_STORE_OP - MEMORY_LOAD_OP) + MEMORY_LOAD_OP, addr * CHUNK_NUM + offset, step, width, ...prove_val]
        match (input.is_write, offset + width > CHUNK_NUM) {
            (false, false) => {
                // RV
                assert!(phase == 0); // TODO: Debug mode

                // Unaligned memory op information thrown into the bus
                let step = input.step;
                let value = input.value;

                // Compute the shift
                let shift = ((offset + width) % CHUNK_NUM) as u64;

                // Get the aligned address
                let addr_read = addr >> CHUNK_BITS;

                // Get the aligned value
                let value_read = mem_values[phase];

                // Get the next pc
                let next_pc = MemAlignRomSM::<F>::calculate_next_pc(MemOp::OneRead, offset, width);

                let mut read_row = MemAlignRow::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u64(addr_read),
                    // offset: F::from_canonical_u64(0),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlignRow::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u64(addr),
                    offset: F::from_canonical_usize(offset),
                    width: F::from_canonical_usize(width),
                    // wr: F::from_bool(false),
                    pc: F::from_canonical_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNK_NUM {
                    let pos = i as u64;

                    read_row.reg[i] = {
                        F::from_canonical_u64(
                            value_read & (CHUNK_BITS_MASK << (pos * CHUNK_BITS_U64)),
                        )
                    };
                    read_row.sel[i] = F::from_bool(true);

                    value_row.reg[i] = {
                        F::from_canonical_u64(
                            value
                                & (CHUNK_BITS_MASK
                                    << (((shift + pos) % CHUNK_NUM_U64) * CHUNK_BITS_U64)),
                        )
                    };
                    value_row.sel[i] = F::from_bool(i == offset as usize);

                    // Store the range check
                    // *range_check.entry(read_row.reg[i]).or_insert(0) += 1;
                    // *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                }

                // Prove the generated rows
                self.prove(&[read_row, value_row]);

                MemAlignResponse { more_address: false, step, value: None }
            }
            (true, false) => {
                // RWV
                assert!(phase == 0); // TODO: Debug mode

                // Unaligned memory op information thrown into the bus
                let step = input.step;
                let value = input.value;

                // Compute the shift
                let shift = ((offset + width) % CHUNK_NUM) as u64;

                // Get the aligned address
                let addr_read = addr >> CHUNK_BITS;

                // Get the aligned value
                let value_read = mem_values[phase];

                // Get the next pc
                let next_pc = MemAlignRomSM::<F>::calculate_next_pc(MemOp::OneWrite, offset, width);

                // Compute the write value
                let value_write = {
                    let width_bytes: u64 = (1 << (width * CHUNK_BITS)) - 1;

                    // Get the first width bytes of the unaligned value
                    let value_to_write = value & width_bytes;

                    // Write zeroes to value_read from offset to offset + width
                    let mask: u64 = width_bytes << (offset * CHUNK_BITS);

                    // Add the value to write to the value read
                    (value_read & !mask) | value_to_write
                };

                let mut read_row = MemAlignRow::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u64(addr_read),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNK_NUM_U64),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut write_row = MemAlignRow::<F> {
                    step: F::from_canonical_u64(step + 1),
                    addr: F::from_canonical_u64(addr_read),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNK_NUM_U64),
                    wr: F::from_bool(true),
                    pc: F::from_canonical_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlignRow::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u64(addr),
                    offset: F::from_canonical_usize(offset),
                    width: F::from_canonical_usize(width),
                    // wr: F::from_bool(false),
                    pc: F::from_canonical_u64(next_pc + 1),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNK_NUM {
                    let pos = i as u64;

                    read_row.reg[i] = {
                        F::from_canonical_u64(
                            value_read & (CHUNK_BITS_MASK << (pos * CHUNK_BITS_U64)),
                        )
                    };
                    read_row.sel[i] = F::from_bool(i >= width);

                    write_row.reg[i] = {
                        F::from_canonical_u64(
                            value_write & (CHUNK_BITS_MASK << (pos * CHUNK_BITS_U64)),
                        )
                    };
                    write_row.sel[i] = F::from_bool(i < width);

                    value_row.reg[i] = {
                        F::from_canonical_u64(
                            value
                                & (CHUNK_BITS_MASK
                                    << (((shift + pos) % CHUNK_NUM_U64) * CHUNK_BITS_U64)),
                        )
                    };
                    value_row.sel[i] = F::from_bool(i == offset as usize);

                    // Store the range check
                    // *range_check.entry(read_row.reg[i]).or_insert(0) += 1;
                    // *range_check.entry(write_row.reg[i]).or_insert(0) += 1;
                    // *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                }

                // Prove the generated rows
                self.prove(&[read_row, write_row, value_row]);

                MemAlignResponse { more_address: false, step, value: Some(value_write) }
            }
            (false, true) => {
                // RVR
                assert!(phase == 0 || phase == 1); // TODO: Debug mode

                match phase {
                    // If phase == 0, do nothing, just ask for more
                    0 => MemAlignResponse { more_address: true, step: input.step, value: None },

                    // Otherwise, do the RVR
                    1 => {
                        assert!(mem_values.len() == 2); // TODO: Debug mode

                        // Unaligned memory op information thrown into the bus
                        let step = input.step;
                        let value = input.value;

                        // Compute the shift
                        let shift = ((offset + width) % CHUNK_NUM) as u64;

                        // Get the aligned address
                        let addr_first_read = addr >> CHUNK_BITS;
                        let addr_second_read = addr >> CHUNK_BITS + CHUNK_BITS;

                        // Get the aligned value
                        let value_first_read = mem_values[0];
                        let value_second_read = mem_values[1];

                        // Get the next pc
                        let next_pc =
                            MemAlignRomSM::<F>::calculate_next_pc(MemOp::TwoReads, offset, width);

                        let mut first_read_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step),
                            addr: F::from_canonical_u64(addr_first_read),
                            // offset: F::from_canonical_u64(0),
                            width: F::from_canonical_u64(CHUNK_NUM_U64),
                            // wr: F::from_bool(false),
                            // pc: F::from_canonical_u64(0),
                            reset: F::from_bool(true),
                            sel_up_to_down: F::from_bool(true),
                            ..Default::default()
                        };

                        let mut value_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step),
                            addr: F::from_canonical_u64(addr),
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
                            addr: F::from_canonical_u64(addr_second_read),
                            // offset: F::from_canonical_u64(0),
                            width: F::from_canonical_u64(CHUNK_NUM_U64),
                            // wr: F::from_bool(false),
                            pc: F::from_canonical_u64(next_pc + 1),
                            // reset: F::from_bool(false),
                            sel_down_to_up: F::from_bool(true),
                            ..Default::default()
                        };

                        for i in 0..CHUNK_NUM {
                            let pos = i as u64;

                            first_read_row.reg[i] = {
                                F::from_canonical_u64(
                                    value_first_read & (CHUNK_BITS_MASK << (pos * CHUNK_BITS_U64)),
                                )
                            };
                            first_read_row.sel[i] = F::from_bool(true);

                            value_row.reg[i] = {
                                F::from_canonical_u64(
                                    value
                                        & (CHUNK_BITS_MASK
                                            << (((shift + pos) % CHUNK_NUM_U64) * CHUNK_BITS_U64)),
                                )
                            };
                            value_row.sel[i] = F::from_bool(i == offset);

                            second_read_row.reg[i] = {
                                F::from_canonical_u64(
                                    value_second_read & (CHUNK_BITS_MASK << (pos * CHUNK_BITS_U64)),
                                )
                            };
                            second_read_row.sel[i] = F::from_bool(true);

                            // Store the range check
                            // *range_check.entry(first_read_row.reg[i]).or_insert(0) += 1;
                            // *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                            // *range_check.entry(second_read_row.reg[i]).or_insert(0) += 1;
                        }

                        // Prove the generated rows
                        self.prove(&[first_read_row, value_row, second_read_row]);

                        MemAlignResponse { more_address: false, step, value: None }
                    }
                    _ => panic!("Invalid phase={}", phase),
                }
            }
            (true, true) => {
                // RWVWR
                assert!(phase == 0 || phase == 1); // TODO: Debug mode

                match phase {
                    // If phase == 0, compute the resulting write value and ask for more
                    0 => {
                        assert!(mem_values.len() == 1); // TODO: Debug mode

                        // Unaligned memory op information thrown into the bus
                        let value = input.value;
                        let step = input.step;

                        // Get the aligned value
                        let value_first_read = mem_values[0];

                        // Compute the write value
                        let value_first_write = {
                            let width_bytes: u64 = (1 << (width * CHUNK_BITS)) - 1;

                            // Get the first width bytes of the unaligned value
                            let value_to_write = value & width_bytes;

                            // Write zeroes to value_read from offset to offset + width
                            let mask = width_bytes << (offset * CHUNK_BITS);

                            // Add the value to write to the value read
                            (value_first_read & !mask) | value_to_write
                        };

                        MemAlignResponse {
                            more_address: true,
                            step,
                            value: Some(value_first_write),
                        }
                    }
                    // Otherwise, do the RWVRW
                    1 => {
                        assert!(mem_values.len() == 2); // TODO: Debug mode

                        // Unaligned memory op information thrown into the bus
                        let step = input.step;
                        let value = input.value;

                        // Compute the shift
                        let shift = ((offset + width) % CHUNK_NUM) as u64;

                        // Get the aligned address
                        let addr_first_read_write = addr >> CHUNK_BITS;
                        let addr_second_read_write = addr >> CHUNK_BITS + CHUNK_BITS;

                        // Get the first aligned value
                        let value_first_read = mem_values[0];

                        // Recompute the first write value
                        let value_first_write = {
                            let width_bytes = (1 << (width * CHUNK_BITS)) - 1;

                            // Get the first width bytes of the unaligned value
                            let value_to_write = value & width_bytes;

                            // Write zeroes to value_read from offset to offset + width
                            let mask = width_bytes << (offset * CHUNK_BITS);

                            // Add the value to write to the value read
                            (value_first_read & !mask) | value_to_write
                        };

                        // Get the second aligned value
                        let value_second_read = mem_values[1];

                        // Compute the second write value
                        let value_second_write = {
                            let width_bytes = (1 << (width * CHUNK_BITS)) - 1;

                            // Get the first width bytes of the unaligned value
                            let value_to_write = value & width_bytes;

                            // Write zeroes to value_read from offset to offset + width
                            let mask = width_bytes << (offset * CHUNK_BITS);

                            // Add the value to write to the value read
                            (value_second_read & !mask) | value_to_write
                        };

                        // Get the next pc
                        let next_pc =
                            MemAlignRomSM::<F>::calculate_next_pc(MemOp::TwoWrites, offset, width);

                        // RWVWR
                        let mut first_read_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step),
                            addr: F::from_canonical_u64(addr_first_read_write),
                            // offset: F::from_canonical_u64(0),
                            width: F::from_canonical_u64(CHUNK_NUM_U64),
                            // wr: F::from_bool(false),
                            // pc: F::from_canonical_u64(0),
                            reset: F::from_bool(true),
                            sel_up_to_down: F::from_bool(true),
                            ..Default::default()
                        };

                        let mut first_write_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step + 1),
                            addr: F::from_canonical_u64(addr_first_read_write),
                            // offset: F::from_canonical_u64(0),
                            width: F::from_canonical_u64(CHUNK_NUM_U64),
                            wr: F::from_bool(true),
                            pc: F::from_canonical_u64(next_pc),
                            // reset: F::from_bool(false),
                            sel_up_to_down: F::from_bool(true),
                            ..Default::default()
                        };

                        let mut value_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step),
                            addr: F::from_canonical_u64(addr),
                            offset: F::from_canonical_usize(offset),
                            width: F::from_canonical_usize(width),
                            // wr: F::from_bool(false),
                            pc: F::from_canonical_u64(next_pc + 1),
                            // reset: F::from_bool(false),
                            sel_prove: F::from_bool(true),
                            ..Default::default()
                        };

                        let mut second_write_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step),
                            addr: F::from_canonical_u64(addr_second_read_write),
                            // offset: F::from_canonical_u64(0),
                            width: F::from_canonical_u64(CHUNK_NUM_U64),
                            wr: F::from_bool(true),
                            pc: F::from_canonical_u64(next_pc + 2),
                            // reset: F::from_bool(false),
                            sel_down_to_up: F::from_bool(true),
                            ..Default::default()
                        };

                        let mut second_read_row = MemAlignRow::<F> {
                            step: F::from_canonical_u64(step + 1),
                            addr: F::from_canonical_u64(addr_second_read_write),
                            // offset: F::from_canonical_u64(0),
                            width: F::from_canonical_u64(CHUNK_NUM_U64),
                            // wr: F::from_bool(false),
                            pc: F::from_canonical_u64(next_pc + 3),
                            reset: F::from_bool(false),
                            sel_down_to_up: F::from_bool(true),
                            ..Default::default()
                        };

                        for i in 0..CHUNK_NUM {
                            let pos = i as u64;

                            first_read_row.reg[i] = {
                                F::from_canonical_u64(
                                    value_first_read & (CHUNK_BITS_MASK << (pos * CHUNK_BITS_U64)),
                                )
                            };
                            first_read_row.sel[i] = F::from_bool(i < offset);

                            first_write_row.reg[i] = {
                                F::from_canonical_u64(
                                    value_first_write & (CHUNK_BITS_MASK << (pos * CHUNK_BITS_U64)),
                                )
                            };
                            first_write_row.sel[i] = F::from_bool(i >= offset);

                            value_row.reg[i] = {
                                F::from_canonical_u64(
                                    value
                                        & (CHUNK_BITS_MASK
                                            << (((shift + pos) % CHUNK_NUM_U64) * CHUNK_BITS_U64)),
                                )
                            };
                            value_row.sel[i] = F::from_bool(i == offset);

                            second_write_row.reg[i] = {
                                F::from_canonical_u64(
                                    value_second_write
                                        & (CHUNK_BITS_MASK << (pos * CHUNK_BITS_U64)),
                                )
                            };
                            second_write_row.sel[i] = F::from_bool(pos < shift);

                            second_read_row.reg[i] = {
                                F::from_canonical_u64(
                                    value_second_read & (CHUNK_BITS_MASK << (pos * CHUNK_BITS_U64)),
                                )
                            };
                            second_read_row.sel[i] = F::from_bool(pos >= shift);
                        }

                        // Store the range check
                        // *range_check.entry(first_read_row.reg[i]).or_insert(0) += 1;
                        // *range_check.entry(first_write_row.reg[i]).or_insert(0) += 1;
                        // *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                        // *range_check.entry(second_write_row.reg[i]).or_insert(0) += 1;
                        // *range_check.entry(second_read_row.reg[i]).or_insert(0) += 1;

                        // Prove the generated rows
                        self.prove(&[
                            first_read_row,
                            first_write_row,
                            value_row,
                            second_write_row,
                            second_read_row,
                        ]);

                        MemAlignResponse {
                            more_address: false,
                            step,
                            value: Some(value_second_write),
                        }
                    }
                    _ => panic!("Invalid phase={}", phase),
                }
            }
        }
    }

    pub fn prove(&self, computed_rows: &[MemAlignRow<F>]) {
        if let Ok(mut rows) = self.rows.lock() {
            rows.extend_from_slice(computed_rows);

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
        assert!(rows_len <= air_mem_align_rows);

        // Get the execution and setup context
        let ectx = self.wcm.get_ectx();
        let sctx = self.wcm.get_sctx();

        // Create a prover buffer
        let (mut prover_buffer, offset) =
            create_prover_buffer(&ectx, &sctx, ZISK_AIRGROUP_ID, MEM_ALIGN_AIR_IDS[0]);

        // Create a Mem Align trace buffer
        let mut trace_buffer =
            MemAlignTrace::<F>::map_buffer(&mut prover_buffer, air_mem_align_rows, offset as usize)
                .unwrap();

        let mut reg_range_check: HashMap<F, u64> = HashMap::new();

        // Add the input rows to the trace
        for (i, &row) in rows.iter().enumerate() {
            // Store the entire row 
            trace_buffer[i] = row;

            // Store the value of all reg columns so that they can be range checked
            for j in 0..CHUNK_NUM {
                *reg_range_check.entry(row.reg[j]).or_insert(0) += 1;
            }
        }

        // Pad the remaining rows with trivially satisfying rows
        let padding_row = MemAlignRow::<F>::default();
        let padding_size = air_mem_align_rows - rows_len;

        // Store the padding rows
        for i in rows_len..air_mem_align_rows {
            trace_buffer[i] = padding_row;
        }

        // Store the value of all reg columns so that they can be range checked
        for j in 0..CHUNK_NUM {
            *reg_range_check.entry(padding_row.reg[j]).or_insert(0) += padding_size as u64;
        }

        // Perform the range checks
        let std = self.std.clone();
        let range_id = std.get_range(BigInt::from(0), BigInt::from(CHUNK_BITS_MASK), None);
        for (&value, &multiplicity) in reg_range_check.iter() {
            std.range_check(value, F::from_canonical_u64(multiplicity), range_id);
        }

        // TODO: Treate the ROM multiplicity

        // TODO: Store the padding multiplicity
        // let mem_align_rom_sm = self.mem_align_rom_sm.clone();
        // let _padding_size = air_mem_align.num_rows() - rows_processed;
        // for i in 0..8 {
        //     let multiplicity = padding_size as u64;
        //     let row = MemAlignRomSM::<F>::calculate_rom_row(
        //         op, offset, width
        //     );
        //     rom_multiplicity[row as usize] += multiplicity;
        // }

        info!(
            "{}: ··· Creating Mem Align instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            rows_len,
            air_mem_align.num_rows(),
            rows_len as f64 / air_mem_align.num_rows() as f64 * 100.0
        );

        // Add a new Mem Align instance
        let air_instance =
            AirInstance::new(sctx, ZISK_AIRGROUP_ID, MEM_ALIGN_AIR_IDS[0], None, prover_buffer);
        wcm.get_pctx().air_instance_repo.add_air_instance(air_instance, None);
    }

    // #[inline(always)]
    // pub fn process_input(
    //     unaligned_input: &ZiskRequiredMemory,
    //     aligned_inputs: &[ZiskRequiredMemory],
    //     mem_align_rom_sm: &MemAlignRomSM<F>,
    //     range_check: &mut HashMap<F, u64>,
    // ) -> Vec<MemAlignRow<F>> {
    //     // Get the unaligned address
    //     let addr = unaligned_input.address;

    //     // Get the unaligned value
    //     let value = unaligned_input.value.to_le_bytes();

    //     // Get the unaligned step
    //     let step = unaligned_input.step;

    //     // Get the unaligned width
    //     let width = unaligned_input.width;
    //     let width = if width <= CHUNK_NUM_U64 {
    //         width as usize
    //     } else {
    //         panic!("Invalid width={}", width);
    //     };

    //     // Compute the offset
    //     let offset = addr % CHUNK_NUM_U64;
    //     let offset = if offset <= usize::MAX as u64 {
    //         offset as usize
    //     } else {
    //         panic!("Invalid offset={}", offset);
    //     };

    //     // Compute the shift
    //     let shift = (offset + width) % CHUNK_NUM;

    //     // Get the op to be executed, its size and the pc to jump to
    //     let op = Self::get_mem_op(&unaligned_input);
    //     let op_size = MemAlignRomSM::<F>::get_mem_align_op_size(op);
    //     let next_pc = MemAlignRomSM::<F>::calculate_next_pc(op, offset, width);

    //     println!("OP: {:?}", op);
    //     println!("UNALIGNED INPUT:\n  {:?}", unaligned_input);
    //     println!("  OFFSET: {:?}", offset);
    //     println!("  value: {:?}", unaligned_input.value.to_le_bytes());
    //     println!("ALIGNED INPUTS:");
    //     for aligned_input in aligned_inputs {
    //         println!("  {:?}", aligned_input);
    //         println!("  value: {:?}", aligned_input.value.to_le_bytes());
    //     }
    //     println!("");

    //     // Initialize and set the rows of the corresponding op
    //     let mut rows: Vec<MemAlignRow<F>> = Vec::with_capacity(op_size);
    //     // TODO: Can I detatch the "shape" of the program from the mem_align and do it in the mem_align_rom?
    //     match op {
    //         MemOp::OneRead => {
    //             // RV
    //             // Sanity check
    //             assert!(aligned_inputs.len() == 1);

    //             // Get the aligned address
    //             let addr_read = aligned_inputs[0].address;

    //             // Get the aligned values
    //             let value_read = aligned_inputs[0].value.to_le_bytes();

    //             // Get the aligned step
    //             let step_read = aligned_inputs[0].step;

    //             let mut read_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step_read),
    //                 addr: F::from_canonical_u64(addr_read),
    //                 // offset: F::from_canonical_u64(0),
    //                 // wr: F::from_bool(false),
    //                 // pc: F::from_canonical_u64(0),
    //                 reset: F::from_bool(true),
    //                 sel_up_to_down: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             let mut value_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step),
    //                 addr: F::from_canonical_u64(addr),
    //                 offset: F::from_canonical_usize(offset),
    //                 width: F::from_canonical_usize(width),
    //                 // wr: F::from_bool(false),
    //                 pc: F::from_canonical_u64(next_pc),
    //                 // reset: F::from_bool(false),
    //                 sel_prove: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             for i in 0..CHUNK_NUM {
    //                 read_row.reg[i] = F::from_canonical_u8(value_read[i]);
    //                 read_row.sel[i] = F::from_bool(true);

    //                 value_row.reg[i] = F::from_canonical_u8(value[(shift + i) % CHUNK_NUM]);
    //                 value_row.sel[i] = F::from_bool(i == offset);

    //                 // Store the range check
    //                 *range_check.entry(read_row.reg[i]).or_insert(0) += 1;
    //                 *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
    //             }

    //             // Store the rows
    //             rows.push(read_row);
    //             rows.push(value_row);
    //         }
    //         MemOp::OneWrite => {
    //             // RWV
    //             // Sanity check
    //             assert!(aligned_inputs.len() == 2);

    //             // Get the aligned address
    //             let addr_read_write = aligned_inputs[0].address;

    //             // Get the aligned values
    //             let value_read = aligned_inputs[0].value.to_le_bytes();
    //             let value_write = aligned_inputs[1].value.to_le_bytes();

    //             // Get the aligned step
    //             let step_read = aligned_inputs[0].step;
    //             let step_write = aligned_inputs[1].step;

    //             // RWV
    //             let mut read_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step_read),
    //                 addr: F::from_canonical_u64(addr_read_write),
    //                 // offset: F::from_canonical_u64(0),
    //                 width: F::from_canonical_u64(CHUNK_NUM_U64),
    //                 // wr: F::from_bool(false),
    //                 // pc: F::from_canonical_u64(0),
    //                 reset: F::from_bool(true),
    //                 sel_up_to_down: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             let mut write_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step_write),
    //                 addr: F::from_canonical_u64(addr_read_write),
    //                 // offset: F::from_canonical_u64(0),
    //                 width: F::from_canonical_u64(CHUNK_NUM_U64),
    //                 wr: F::from_bool(true),
    //                 pc: F::from_canonical_u64(next_pc),
    //                 // reset: F::from_bool(false),
    //                 sel_up_to_down: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             let mut value_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step),
    //                 addr: F::from_canonical_u64(addr),
    //                 offset: F::from_canonical_usize(offset),
    //                 width: F::from_canonical_usize(width),
    //                 // wr: F::from_bool(false),
    //                 pc: F::from_canonical_u64(next_pc + 1),
    //                 // reset: F::from_bool(false),
    //                 sel_prove: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             for i in 0..CHUNK_NUM {
    //                 read_row.reg[i] = F::from_canonical_u8(value_read[i]);
    //                 read_row.sel[i] = F::from_bool(i >= width);

    //                 write_row.reg[i] = F::from_canonical_u8(value_write[i]);
    //                 write_row.sel[i] = F::from_bool(i < width);

    //                 value_row.reg[i] = F::from_canonical_u8(value[(shift + i) % CHUNK_NUM]);
    //                 value_row.sel[i] = F::from_bool(i == offset);

    //                 // Store the range check
    //                 *range_check.entry(read_row.reg[i]).or_insert(0) += 1;
    //                 *range_check.entry(write_row.reg[i]).or_insert(0) += 1;
    //                 *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
    //             }

    //             // Store the rows
    //             rows.push(read_row);
    //             rows.push(write_row);
    //             rows.push(value_row);
    //         }
    //         MemOp::TwoReads => {
    //             // RVR
    //             // Sanity check
    //             assert!(aligned_inputs.len() == 2);

    //             // Get the aligned address
    //             let addr_first_read = aligned_inputs[0].address;
    //             let addr_second_read = aligned_inputs[1].address;

    //             // Get the aligned values
    //             let value_first_read = aligned_inputs[0].value.to_le_bytes();
    //             let value_second_read = aligned_inputs[1].value.to_le_bytes();

    //             // Get the aligned step
    //             let step_first_read = aligned_inputs[0].step;
    //             let step_second_read = aligned_inputs[1].step;

    //             // RVR
    //             let mut first_read_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step_first_read),
    //                 addr: F::from_canonical_u64(addr_first_read),
    //                 // offset: F::from_canonical_u64(0),
    //                 width: F::from_canonical_u64(CHUNK_NUM_U64),
    //                 // wr: F::from_bool(false),
    //                 // pc: F::from_canonical_u64(0),
    //                 reset: F::from_bool(true),
    //                 sel_up_to_down: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             let mut value_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step),
    //                 addr: F::from_canonical_u64(addr),
    //                 offset: F::from_canonical_usize(offset),
    //                 width: F::from_canonical_usize(width),
    //                 // wr: F::from_bool(false),
    //                 pc: F::from_canonical_u64(next_pc),
    //                 // reset: F::from_bool(false),
    //                 sel_prove: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             let mut second_read_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step_second_read),
    //                 addr: F::from_canonical_u64(addr_second_read),
    //                 // offset: F::from_canonical_u64(0),
    //                 width: F::from_canonical_u64(CHUNK_NUM_U64),
    //                 // wr: F::from_bool(false),
    //                 pc: F::from_canonical_u64(next_pc + 1),
    //                 // reset: F::from_bool(false),
    //                 sel_down_to_up: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             for i in 0..CHUNK_NUM {
    //                 first_read_row.reg[i] = F::from_canonical_u8(value_first_read[i]);
    //                 first_read_row.sel[i] = F::from_bool(true);

    //                 value_row.reg[i] = F::from_canonical_u8(value[(shift + i) % CHUNK_NUM]);
    //                 value_row.sel[i] = F::from_bool(i == offset);

    //                 second_read_row.reg[i] = F::from_canonical_u8(value_second_read[i]);
    //                 second_read_row.sel[i] = F::from_bool(true);

    //                 // Store the range check
    //                 *range_check.entry(first_read_row.reg[i]).or_insert(0) += 1;
    //                 *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
    //                 *range_check.entry(second_read_row.reg[i]).or_insert(0) += 1;
    //             }

    //             // Store the rows
    //             rows.push(first_read_row);
    //             rows.push(value_row);
    //             rows.push(second_read_row);
    //         }
    //         MemOp::TwoWrites => {
    //             // RWVWR
    //             // Sanity check
    //             assert!(aligned_inputs.len() == 4);

    //             // Get the aligned address
    //             let addr_first_read_write = aligned_inputs[0].address;
    //             let addr_second_read_write = aligned_inputs[2].address;

    //             // Get the aligned values
    //             // TODO: I do not need to establish an order, I can use the field is_write!!!
    //             let value_first_read = aligned_inputs[0].value.to_le_bytes();
    //             let value_first_write = aligned_inputs[1].value.to_le_bytes();
    //             let value_second_read = aligned_inputs[2].value.to_le_bytes();
    //             let value_second_write = aligned_inputs[3].value.to_le_bytes();

    //             // Get the aligned step
    //             let step_first_read = aligned_inputs[0].step;
    //             let step_first_write = aligned_inputs[1].step;
    //             let step_second_read = aligned_inputs[2].step;
    //             let step_second_write = aligned_inputs[3].step;

    //             // RWVWR
    //             let mut first_read_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step_first_read),
    //                 addr: F::from_canonical_u64(addr_first_read_write),
    //                 // offset: F::from_canonical_u64(0),
    //                 width: F::from_canonical_u64(CHUNK_NUM_U64),
    //                 // wr: F::from_bool(false),
    //                 // pc: F::from_canonical_u64(0),
    //                 reset: F::from_bool(true),
    //                 sel_up_to_down: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             let mut first_write_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step_first_write),
    //                 addr: F::from_canonical_u64(addr_first_read_write),
    //                 // offset: F::from_canonical_u64(0),
    //                 width: F::from_canonical_u64(CHUNK_NUM_U64),
    //                 wr: F::from_bool(true),
    //                 pc: F::from_canonical_u64(next_pc),
    //                 // reset: F::from_bool(false),
    //                 sel_up_to_down: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             let mut value_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step),
    //                 addr: F::from_canonical_u64(addr),
    //                 offset: F::from_canonical_usize(offset),
    //                 width: F::from_canonical_usize(width),
    //                 // wr: F::from_bool(false),
    //                 pc: F::from_canonical_u64(next_pc + 1),
    //                 // reset: F::from_bool(false),
    //                 sel_prove: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             let mut second_write_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step_second_write),
    //                 addr: F::from_canonical_u64(addr_second_read_write),
    //                 // offset: F::from_canonical_u64(0),
    //                 width: F::from_canonical_u64(CHUNK_NUM_U64),
    //                 wr: F::from_bool(true),
    //                 pc: F::from_canonical_u64(next_pc + 2),
    //                 // reset: F::from_bool(false),
    //                 sel_down_to_up: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             let mut second_read_row = MemAlignRow::<F> {
    //                 step: F::from_canonical_u64(step_second_read),
    //                 addr: F::from_canonical_u64(addr_second_read_write),
    //                 // offset: F::from_canonical_u64(0),
    //                 width: F::from_canonical_u64(CHUNK_NUM_U64),
    //                 // wr: F::from_bool(false),
    //                 pc: F::from_canonical_u64(next_pc + 3),
    //                 reset: F::from_bool(false),
    //                 sel_down_to_up: F::from_bool(true),
    //                 ..Default::default()
    //             };

    //             for i in 0..CHUNK_NUM {
    //                 first_read_row.reg[i] = F::from_canonical_u8(value_first_read[i]);
    //                 first_read_row.sel[i] = F::from_bool(i < offset);

    //                 first_write_row.reg[i] = F::from_canonical_u8(value_first_write[i]);
    //                 first_write_row.sel[i] = F::from_bool(i >= offset);

    //                 value_row.reg[i] = F::from_canonical_u8(value[(shift + i) % CHUNK_NUM]);
    //                 value_row.sel[i] = F::from_bool(i == offset);

    //                 second_write_row.reg[i] = F::from_canonical_u8(value_second_write[i]);
    //                 second_write_row.sel[i] = F::from_bool(i < shift);

    //                 second_read_row.reg[i] = F::from_canonical_u8(value_second_read[i]);
    //                 second_read_row.sel[i] = F::from_bool(i >= shift);

    //                 // Store the range check
    //                 *range_check.entry(first_read_row.reg[i]).or_insert(0) += 1;
    //                 *range_check.entry(first_write_row.reg[i]).or_insert(0) += 1;
    //                 *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
    //                 *range_check.entry(second_write_row.reg[i]).or_insert(0) += 1;
    //                 *range_check.entry(second_read_row.reg[i]).or_insert(0) += 1;
    //             }

    //             // Store the rows
    //             rows.push(first_read_row);
    //             rows.push(first_write_row);
    //             rows.push(value_row);
    //             rows.push(second_write_row);
    //             rows.push(second_read_row);
    //         }
    //     }

    //     // Update the ROM row multiplicity
    //     mem_align_rom_sm.update_multiplicity_by_input(op, offset, width);

    //     // Return successfully
    //     rows
    // }
}

impl<F: PrimeField> WitnessComponent<F> for MemAlignSM<F> {}
