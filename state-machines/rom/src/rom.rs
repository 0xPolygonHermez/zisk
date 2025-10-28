//! The `RomSM` module implements the ROM State Machine,
//! directly managing the ROM execution process, generating traces, and computing custom traces.
//!
//! Key components of this module include:
//! - The `RomSM` struct, which represents the ROM State Machine and encapsulates ROM-related
//!   operations.
//! - Methods for proving instances and computing traces from the ROM data.
//! - `ComponentBuilder` trait implementations for creating counters, planners, and input
//!   collectors.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

use crate::{RomInstance, RomPlanner};
use asm_runner::{AsmRHData, AsmRunnerRH};
use fields::PrimeField64;
use itertools::Itertools;
use proofman_common::{AirInstance, FromTrace};
use zisk_common::{
    create_atomic_vec, BusDeviceMetrics, ComponentBuilder, CounterStats, Instance, InstanceCtx,
    Planner,
};
use zisk_core::{
    zisk_ops::ZiskOp, Riscv2zisk, ZiskRom, ROM_ADDR, ROM_ADDR_MAX, ROM_ENTRY, ROM_EXIT, SRC_IMM,
};
use zisk_pil::{MainTrace, RomRomTrace, RomRomTraceRow, RomTrace};

/// The `RomSM` struct represents the ROM State Machine
pub struct RomSM {
    /// Zisk Rom
    zisk_rom: Arc<ZiskRom>,

    /// Shared biod instruction counter for monitoring ROM operations.
    bios_inst_count: Arc<Vec<AtomicU32>>,

    /// Shared program instruction counter for monitoring ROM operations.
    prog_inst_count: Arc<Vec<AtomicU32>>,

    asm_runner_handler: Mutex<Option<JoinHandle<AsmRunnerRH>>>,
}

impl RomSM {
    /// Creates a new instance of the `RomSM` state machine.
    ///
    /// # Arguments
    /// * `zisk_rom` - The Zisk ROM representation.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `RomSM`.
    pub fn new(zisk_rom: Arc<ZiskRom>, asm_rom_path: Option<PathBuf>) -> Arc<Self> {
        let (bios_inst_count, prog_inst_count) = if asm_rom_path.is_some() {
            (vec![], vec![])
        } else {
            (
                create_atomic_vec(((ROM_ADDR - ROM_ENTRY) as usize) >> 2), // No atomics, we can divide by 4
                create_atomic_vec((ROM_ADDR_MAX - ROM_ADDR) as usize), // Cannot be dividede by 4
            )
        };

        Arc::new(Self {
            zisk_rom,
            bios_inst_count: Arc::new(bios_inst_count),
            prog_inst_count: Arc::new(prog_inst_count),
            asm_runner_handler: Mutex::new(None),
        })
    }

    pub fn set_asm_runner_handler(&self, handler: JoinHandle<AsmRunnerRH>) {
        *self.asm_runner_handler.lock().unwrap() = Some(handler);
    }

    /// Computes the witness for the provided plan using the given ROM.
    ///
    /// # Arguments
    /// * `rom` - Reference to the Zisk ROM.
    /// * `plan` - The execution plan for computing the witness.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness trace data.
    pub fn compute_witness<F: PrimeField64>(
        rom: &ZiskRom,
        counter_stats: &CounterStats,
        calculated: &AtomicBool,
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut rom_trace = RomTrace::new_from_vec_zeroes(trace_buffer);

        let main_trace_len = MainTrace::<F>::NUM_ROWS as u64;

        tracing::info!("··· Creating Rom instance [{} rows]", RomTrace::<F>::NUM_ROWS);

        // For every instruction in the rom, fill its corresponding ROM trace
        for (i, key) in rom.insts.keys().sorted().enumerate() {
            // Get the Zisk instruction
            let inst = &rom.insts[key].i;

            // Calculate the multiplicity, i.e. the number of times this pc is used in this
            // execution
            let mut multiplicity: u64;
            if inst.paddr < ROM_ADDR {
                if counter_stats.bios_inst_count.is_empty() {
                    multiplicity = 1; // If the histogram is empty, we use 1 for all pc's
                } else {
                    match calculated.load(std::sync::atomic::Ordering::Relaxed) {
                        true => {
                            multiplicity = counter_stats.bios_inst_count
                                [((inst.paddr - ROM_ENTRY) as usize) >> 2]
                                .swap(0, std::sync::atomic::Ordering::Relaxed)
                                as u64;
                        }
                        false => {
                            multiplicity = counter_stats.bios_inst_count
                                [((inst.paddr - ROM_ENTRY) as usize) >> 2]
                                .load(std::sync::atomic::Ordering::Relaxed)
                                as u64;
                        }
                    }

                    if multiplicity == 0 {
                        continue;
                    }
                    if inst.paddr == counter_stats.end_pc {
                        multiplicity += main_trace_len - counter_stats.steps % main_trace_len;
                    }
                }
            } else {
                match calculated.load(std::sync::atomic::Ordering::Relaxed) {
                    true => {
                        multiplicity = counter_stats.prog_inst_count
                            [(inst.paddr - ROM_ADDR) as usize]
                            .swap(0, std::sync::atomic::Ordering::Relaxed)
                            as u64
                    }
                    false => {
                        multiplicity = counter_stats.prog_inst_count
                            [(inst.paddr - ROM_ADDR) as usize]
                            .load(std::sync::atomic::Ordering::Relaxed)
                            as u64
                    }
                }
                if multiplicity == 0 {
                    continue;
                }
                if inst.paddr == counter_stats.end_pc {
                    multiplicity += main_trace_len - counter_stats.steps % main_trace_len;
                }
            }
            rom_trace[i].multiplicity = F::from_u64(multiplicity);
        }

        AirInstance::new_from_trace(FromTrace::new(&mut rom_trace))
    }

    pub fn compute_witness_from_asm<F: PrimeField64>(
        rom: &ZiskRom,
        asm_romh: &AsmRHData,
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut rom_trace = RomTrace::new_from_vec_zeroes(trace_buffer);

        tracing::info!("··· Creating Rom instance [{} rows]", RomTrace::<F>::NUM_ROWS);

        let main_trace_len = MainTrace::<F>::NUM_ROWS as u64;

        for (i, key) in rom.insts.keys().sorted().enumerate() {
            // Get the Zisk instruction
            let inst = &rom.insts[key].i;

            // Calculate the multiplicity, i.e. the number of times this pc is used in this
            // execution
            let mut multiplicity: u64;
            if inst.paddr < ROM_ADDR {
                if asm_romh.bios_inst_count.is_empty() {
                    multiplicity = 1; // If the histogram is empty, we use 1 for all pc's
                } else {
                    let idx = ((inst.paddr - ROM_ENTRY) as usize) >> 2;

                    multiplicity = asm_romh.bios_inst_count[idx];

                    if multiplicity == 0 {
                        continue;
                    }

                    if inst.paddr == ROM_EXIT {
                        multiplicity += main_trace_len - asm_romh.steps % main_trace_len;
                    }
                }
            } else {
                let idx = (inst.paddr - ROM_ADDR) as usize;
                multiplicity = asm_romh.prog_inst_count[idx];

                if multiplicity == 0 {
                    continue;
                }
            }

            rom_trace[i].multiplicity = F::from_u64(multiplicity);
        }

        AirInstance::new_from_trace(FromTrace::new(&mut rom_trace))
    }

    /// Computes the ROM trace based on the ROM instructions.
    ///
    /// # Arguments
    /// * `rom` - Reference to the Zisk ROM.
    /// * `rom_custom_trace` - Reference to the custom ROM trace.
    fn compute_trace_rom<F: PrimeField64>(rom: &ZiskRom, rom_custom_trace: &mut RomRomTrace<F>) {
        // For every instruction in the rom, fill its corresponding ROM trace
        for (i, key) in rom.insts.keys().sorted().enumerate() {
            // Get the Zisk instruction
            let inst = &rom.insts[key].i;

            // Convert the i64 offsets to F
            let jmp_offset1 = if inst.jmp_offset1 >= 0 {
                F::from_u64(inst.jmp_offset1 as u64)
            } else {
                F::neg(F::from_u64((-inst.jmp_offset1) as u64))
            };
            let jmp_offset2 = if inst.jmp_offset2 >= 0 {
                F::from_u64(inst.jmp_offset2 as u64)
            } else {
                F::neg(F::from_u64((-inst.jmp_offset2) as u64))
            };
            let store_offset = if inst.store_offset >= 0 {
                F::from_u64(inst.store_offset as u64)
            } else {
                F::neg(F::from_u64((-inst.store_offset) as u64))
            };
            let a_offset_imm0 = if inst.a_offset_imm0 as i64 >= 0 {
                F::from_u64(inst.a_offset_imm0)
            } else {
                F::neg(F::from_u64((-(inst.a_offset_imm0 as i64)) as u64))
            };
            let b_offset_imm0 = if inst.b_offset_imm0 as i64 >= 0 {
                F::from_u64(inst.b_offset_imm0)
            } else {
                F::neg(F::from_u64((-(inst.b_offset_imm0 as i64)) as u64))
            };

            // Fill the rom trace row fields
            rom_custom_trace[i].line = F::from_u64(inst.paddr); // TODO: unify names: pc, paddr, line
            rom_custom_trace[i].a_offset_imm0 = a_offset_imm0;
            rom_custom_trace[i].a_imm1 =
                F::from_u64(if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0 });
            rom_custom_trace[i].b_offset_imm0 = b_offset_imm0;
            rom_custom_trace[i].b_imm1 =
                F::from_u64(if inst.b_src == SRC_IMM { inst.b_use_sp_imm1 } else { 0 });
            rom_custom_trace[i].ind_width = F::from_u64(inst.ind_width);
            // IMPORTANT: the opcodes fcall, fcall_get, and fcall_param are really a variant
            // of the copyb, use to get free-input information
            rom_custom_trace[i].op = if inst.op == ZiskOp::Fcall.code()
                || inst.op == ZiskOp::FcallGet.code()
                || inst.op == ZiskOp::FcallParam.code()
            {
                F::from_u8(ZiskOp::CopyB.code())
            } else {
                F::from_u8(inst.op)
            };
            rom_custom_trace[i].store_offset = store_offset;
            rom_custom_trace[i].jmp_offset1 = jmp_offset1;
            rom_custom_trace[i].jmp_offset2 = jmp_offset2;
            rom_custom_trace[i].flags = F::from_u64(inst.get_flags());
        }

        // Padd with zeroes
        let num_rows: usize = RomRomTrace::<F>::NUM_ROWS;
        for i in rom.insts.len()..num_rows {
            rom_custom_trace[i] = RomRomTraceRow::default();
        }
    }

    /// Computes a custom trace ROM from the given ELF file.
    ///
    /// # Arguments
    /// * `rom_path` - The path to the ELF file.
    /// * `rom_custom_trace` - Reference to the custom ROM trace.
    pub fn compute_custom_trace_rom<F: PrimeField64>(
        rom_path: PathBuf,
        rom_custom_trace: &mut RomRomTrace<F>,
    ) {
        // Get the ELF file path as a string
        let elf_filename: String = rom_path.to_str().unwrap().into();
        tracing::info!("Computing custom trace ROM");

        // Load and parse the ELF file, and transpile it into a ZisK ROM using Riscv2zisk

        // Create an instance of the RISCV -> ZisK program converter
        let riscv2zisk = Riscv2zisk::new(elf_filename);

        // Convert program to rom
        let rom = riscv2zisk.run().expect("RomSM::prover() failed converting elf to rom");

        Self::compute_trace_rom(&rom, rom_custom_trace);
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for RomSM {
    /// Builds and returns a new counter for monitoring ROM operations.
    ///
    /// # Returns
    /// A boxed implementation of `RomCounter`.
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        None
    }

    /// Builds a planner for ROM-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RomPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(RomPlanner)
    }

    /// Builds an instance of the ROM state machine.
    ///
    /// # Arguments
    /// * `ictx` - The context of the instance, containing the plan and its associated
    ///
    /// # Returns
    /// A boxed implementation of `RomInstance`.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        let mut handle_rh_guard = self.asm_runner_handler.lock().unwrap();
        let handle_rh = handle_rh_guard.take();

        Box::new(RomInstance::new(
            self.zisk_rom.clone(),
            ictx,
            self.bios_inst_count.clone(),
            self.prog_inst_count.clone(),
            handle_rh,
        ))
    }
}
