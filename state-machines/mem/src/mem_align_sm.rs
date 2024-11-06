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
use rayon::Scope;

use sm_common::{create_prover_buffer, OpResult, Provable};
use zisk_core::ZiskRequiredMemory;
use zisk_pil::{MemAlign3Row, MemAlign3Trace, MEM_ALIGN_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::MemAlignRomSM;

#[derive(Debug, Clone, Copy)]
pub enum MemOp {
    OneRead,
    OneWrite,
    TwoReads,
    TwoWrites,
}

const PROVE_CHUNK_SIZE: usize = 1 << 12;

const CHUNK_NUM: usize = 8;
const CHUNK_NUM_U64: u64 = CHUNK_NUM as u64;
const CHUNK_BITS: usize = 8;

pub struct MemAlignSM<F: PrimeField> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    // STD
    std: Arc<Std<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<ZiskRequiredMemory>>,

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
            inputs: Mutex::new(Vec::new()),
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
            self.mem_align_rom_sm.unregister_predecessor();
            self.std.unregister_predecessor(self.wcm.get_pctx(), None);
        }
    }

    #[inline(always)]
    pub fn get_mem_op(unaligned_input: &ZiskRequiredMemory) -> MemOp {
        let addr = unaligned_input.address;
        let width = unaligned_input.width;

        let offset = addr % 8;

        match (unaligned_input.is_write, offset + width > 8) {
            (false, false) => MemOp::OneRead,
            (true, false) => MemOp::OneWrite,
            (false, true) => MemOp::TwoReads,
            (true, true) => MemOp::TwoWrites,
        }
    }

    #[inline(always)]
    pub fn process_slice(
        input: &Vec<ZiskRequiredMemory>,
        mem_align_rom_sm: &MemAlignRomSM<F>,
        range_check: &mut HashMap<F, u64>,
    ) -> Vec<MemAlign3Row<F>> {
        // Is a write or a read operation
        let _wr = input[0].is_write;

        // Get the address
        let addr = input[0].address;
        let addr_prior = input[1].address; // addr / CHUNK_NUM;
        let addr_next = input[2].address; // addr / CHUNK_NUM + CHUNK_NUM;

        // Get the value
        let value = input[0].value.to_be_bytes();
        let value_first_read = input[1].value.to_be_bytes();
        let value_first_write = input[2].value.to_be_bytes();
        let value_second_read = input[3].value.to_be_bytes();
        let value_second_write = input[4].value.to_be_bytes();

        // Get the step
        let step = input[0].step;
        let step_first_read = input[1].step;
        let step_first_write = input[2].step;
        let step_second_read = input[3].step;
        let step_second_write = input[4].step;

        // Get the offset
        let offset = addr % CHUNK_NUM_U64;
        let offset = if offset <= usize::MAX as u64 {
            offset as usize
        } else {
            panic!("Invalid offset={}", offset);
        };

        // Get the width
        let width = input[0].width;
        let width = if width <= CHUNK_NUM_U64 {
            width as usize
        } else {
            panic!("Invalid width={}", width);
        };

        // Compute the shift
        let shift = (offset + width) % CHUNK_NUM;

        // Get the op to be executed, its size and the pc to jump to
        let op = Self::get_mem_op(&input[0]);
        let op_size = MemAlignRomSM::<F>::get_mem_align_op_size(op);
        let next_pc = MemAlignRomSM::<F>::calculate_next_pc(op, offset, width);

        // Initialize and set the rows of the corresponding op
        let mut rows: Vec<MemAlign3Row<F>> = Vec::with_capacity(op_size);
        // TODO: Can I detatch the "shape" of the program from the mem_align and do it in the mem_align_rom?
        match op {
            MemOp::OneRead => {
                // RV
                let mut read_row = MemAlign3Row::<F> {
                    step: F::from_canonical_u64(step_first_read),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlign3Row::<F> {
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
                    read_row.reg[i] = F::from_canonical_u8(value_first_read[i]);
                    read_row.sel[i] = F::from_bool(true);

                    value_row.reg[i] = F::from_canonical_u8(value[shift + i]);
                    value_row.sel[i] = F::from_bool(i == offset);

                    // Store the range check
                    *range_check.entry(read_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                }

                // Store the rows
                rows.push(read_row);
                rows.push(value_row);
            }
            MemOp::OneWrite => {
                // RWV
                let mut read_row = MemAlign3Row::<F> {
                    step: F::from_canonical_u64(step_first_read),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNK_NUM_U64),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut write_row = MemAlign3Row::<F> {
                    step: F::from_canonical_u64(step_first_write),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNK_NUM_U64),
                    wr: F::from_bool(true),
                    pc: F::from_canonical_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlign3Row::<F> {
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
                    read_row.reg[i] = F::from_canonical_u8(value_first_read[i]);
                    read_row.sel[i] = F::from_bool(i < offset);

                    write_row.reg[i] = F::from_canonical_u8(value_first_write[i]);
                    write_row.sel[i] = F::from_bool(i >= offset);

                    value_row.reg[i] = F::from_canonical_u8(value[shift + i]);
                    value_row.sel[i] = F::from_bool(i == offset);

                    // Store the range check
                    *range_check.entry(read_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(write_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                }

                // Store the rows
                rows.push(read_row);
                rows.push(write_row);
                rows.push(value_row);
            }
            MemOp::TwoReads => {
                // RVR
                let mut first_read_row = MemAlign3Row::<F> {
                    step: F::from_canonical_u64(step_first_read),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNK_NUM_U64),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlign3Row::<F> {
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

                let mut second_read_row = MemAlign3Row::<F> {
                    step: F::from_canonical_u64(step_second_read),
                    addr: F::from_canonical_u64(addr_next),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNK_NUM_U64),
                    // wr: F::from_bool(false),
                    pc: F::from_canonical_u64(next_pc + 1),
                    // reset: F::from_bool(false),
                    sel_down_to_up: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNK_NUM {
                    first_read_row.reg[i] = F::from_canonical_u8(value_first_read[i]);
                    first_read_row.sel[i] = F::from_bool(true);

                    value_row.reg[i] = F::from_canonical_u8(value[shift + i]);
                    value_row.sel[i] = F::from_bool(i == offset);

                    second_read_row.reg[i] = F::from_canonical_u8(value_second_read[i]);
                    second_read_row.sel[i] = F::from_bool(true);

                    // Store the range check
                    *range_check.entry(first_read_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(second_read_row.reg[i]).or_insert(0) += 1;
                }

                // Store the rows
                rows.push(first_read_row);
                rows.push(value_row);
                rows.push(second_read_row);
            }
            MemOp::TwoWrites => {
                // RWVWR
                let mut first_read_row = MemAlign3Row::<F> {
                    step: F::from_canonical_u64(step_first_read),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNK_NUM_U64),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut first_write_row = MemAlign3Row::<F> {
                    step: F::from_canonical_u64(step_first_write),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNK_NUM_U64),
                    wr: F::from_bool(true),
                    pc: F::from_canonical_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemAlign3Row::<F> {
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

                let mut second_write_row = MemAlign3Row::<F> {
                    step: F::from_canonical_u64(step_second_write),
                    addr: F::from_canonical_u64(addr_next),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNK_NUM_U64),
                    wr: F::from_bool(true),
                    pc: F::from_canonical_u64(next_pc + 2),
                    // reset: F::from_bool(false),
                    sel_down_to_up: F::from_bool(true),
                    ..Default::default()
                };

                let mut second_read_row = MemAlign3Row::<F> {
                    step: F::from_canonical_u64(step_second_read),
                    addr: F::from_canonical_u64(addr_next),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNK_NUM_U64),
                    // wr: F::from_bool(false),
                    pc: F::from_canonical_u64(next_pc + 3),
                    reset: F::from_bool(false),
                    sel_down_to_up: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNK_NUM {
                    first_read_row.reg[i] = F::from_canonical_u8(value_first_read[i]);
                    first_read_row.sel[i] = F::from_bool(i < offset);

                    first_write_row.reg[i] = F::from_canonical_u8(value_first_write[i]);
                    first_write_row.sel[i] = F::from_bool(i >= offset);

                    value_row.reg[i] = F::from_canonical_u8(value[shift + i]);
                    value_row.sel[i] = F::from_bool(i == offset);

                    second_write_row.reg[i] = F::from_canonical_u8(value_second_write[i]);
                    second_write_row.sel[i] = F::from_bool(i < shift);

                    second_read_row.reg[i] = F::from_canonical_u8(value_second_read[i]);
                    second_read_row.sel[i] = F::from_bool(i >= shift);

                    // Store the range check
                    *range_check.entry(first_read_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(first_write_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(second_write_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(second_read_row.reg[i]).or_insert(0) += 1;
                }

                // Store the rows
                rows.push(first_read_row);
                rows.push(first_write_row);
                rows.push(value_row);
                rows.push(second_write_row);
                rows.push(second_read_row);
            }
        }

        // Update the ROM row multiplicity
        mem_align_rom_sm.update_multiplicity_by_input(op, offset, width);

        // Return successfully
        rows
    }

    pub fn prove_instance(
        &self,
        inputs: Vec<ZiskRequiredMemory>,
        prover_buffer: &mut [F],
        offset: u64,
    ) {
        Self::prove_internal(
            &self.wcm,
            &self.mem_align_rom_sm,
            &self.std,
            inputs,
            prover_buffer,
            offset,
        );
    }

    fn prove_internal(
        wcm: &WitnessManager<F>,
        mem_align_rom_sm: &MemAlignRomSM<F>,
        std: &Std<F>,
        inputs: Vec<ZiskRequiredMemory>,
        prover_buffer: &mut [F],
        offset: u64,
    ) {
        let pctx = wcm.get_pctx();

        let air_mem_align = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_ALIGN_AIR_IDS[0]);
        assert!(inputs.len() <= air_mem_align.num_rows());

        info!(
            "{}: ··· Creating Mem Align instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            inputs.len(),
            air_mem_align.num_rows(),
            inputs.len() as f64 / air_mem_align.num_rows() as f64 * 100.0
        );

        let mut reg_range_check: HashMap<F, u64> = HashMap::new();
        let mut trace_buffer = MemAlign3Trace::<F>::map_buffer(
            prover_buffer,
            air_mem_align.num_rows(),
            offset as usize,
        )
        .unwrap();

        // Process the inputs while saving the multiplcities and range checks
        let mut rows_processed = 0;
        let rows = Self::process_slice(&inputs, mem_align_rom_sm, &mut reg_range_check);
        for (i, &row) in rows.iter().enumerate() {
            trace_buffer[rows_processed + i] = row;
        }
        rows_processed += rows.len();

        // Pad the remaining rows with trivailly satisfying rows
        let padding_row = MemAlign3Row::<F>::default();

        for i in rows_processed..air_mem_align.num_rows() {
            trace_buffer[i] = padding_row;
        }

        // TODO: Store the padding multiplicity
        let _padding_size = air_mem_align.num_rows() - rows_processed;
        // for i in 0..8 {
        //     let multiplicity = padding_size as u64;
        //     let row = MemAlignRomSM::<F>::calculate_rom_row(
        //         op, offset, width
        //     );
        //     rom_multiplicity[row as usize] += multiplicity;
        // }

        // Perform the range checks
        let range_id = std.get_range(BigInt::from(0), BigInt::from((1 << CHUNK_BITS) - 1), None);
        for (&value, &multiplicity) in reg_range_check.iter() {
            std.range_check(value, F::from_canonical_u64(multiplicity), range_id);
        }

        // std::thread::spawn(move || {
        //     drop(inputs);
        //     drop(reg_range_check);
        // });
    }
}

impl<F: PrimeField> WitnessComponent<F> for MemAlignSM<F> {}

impl<F: PrimeField> Provable<ZiskRequiredMemory, OpResult> for MemAlignSM<F> {
    fn prove(&self, operations: &[ZiskRequiredMemory], drain: bool, _scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            let pctx = self.wcm.get_pctx();
            let air_mem_align = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_ALIGN_AIR_IDS[0]);

            while inputs.len() >= air_mem_align.num_rows() || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(air_mem_align.num_rows(), inputs.len());
                let drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                let mem_align_rom_sm = self.mem_align_rom_sm.clone();
                let wcm = self.wcm.clone();
                let std = self.std.clone();
                let sctx = self.wcm.get_sctx().clone();

                let (mut prover_buffer, offset) = create_prover_buffer(
                    &wcm.get_ectx(),
                    &wcm.get_sctx(),
                    ZISK_AIRGROUP_ID,
                    MEM_ALIGN_AIR_IDS[0],
                );

                Self::prove_internal(
                    &wcm,
                    &mem_align_rom_sm,
                    &std,
                    drained_inputs,
                    &mut prover_buffer,
                    offset,
                );

                let air_instance = AirInstance::new(
                    sctx,
                    ZISK_AIRGROUP_ID,
                    MEM_ALIGN_AIR_IDS[0],
                    None,
                    prover_buffer,
                );
                wcm.get_pctx().air_instance_repo.add_air_instance(air_instance, None);
            }
        }
    }
}
