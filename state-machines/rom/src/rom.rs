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
    sync::{atomic::AtomicU32, Arc, Mutex},
};

use asm_runner::AsmRHData;
use itertools::Itertools;
use log::info;
use p3_field::PrimeField;
use proofman_common::{AirInstance, FromTrace};
use sm_common::{
    create_atomic_vec, BusDeviceMetrics, ComponentBuilder, CounterStats, InstanceCtx, Planner,
};

use crate::{rom_asm_worker::RomAsmWorker, RomCounter, RomInstance, RomPlanner};
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

    /// The ROM assembly worker
    rom_asm_worker: Mutex<Option<RomAsmWorker>>,

    asm_rom_path: Option<PathBuf>,
}

impl RomSM {
    const MY_NAME: &'static str = "RomSM   ";

    /// Creates a new instance of the `RomSM` state machine.
    ///
    /// # Arguments
    /// * `zisk_rom` - The Zisk ROM representation.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `RomSM`.
    pub fn new(zisk_rom: Arc<ZiskRom>, asm_rom_path: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Self {
            zisk_rom,
            // No atomics, we can fivide by 4
            bios_inst_count: Arc::new(create_atomic_vec(((ROM_ADDR - ROM_ENTRY) as usize) >> 2)),
            // Cannot be dividede by 4
            prog_inst_count: Arc::new(create_atomic_vec((ROM_ADDR_MAX - ROM_ADDR) as usize)),
            asm_rom_path,
            rom_asm_worker: Mutex::new(None),
        })
    }

    pub fn set_asm_rom_worker(&self, input_data_path: Option<PathBuf>) {
        let rom_asm_worker = self
            .asm_rom_path
            .as_ref()
            .map(|asm_rom_path| {
                let mut worker = RomAsmWorker::new();
                worker.launch_task(asm_rom_path.clone(), input_data_path);
                worker
            })
            .unwrap();
        *self.rom_asm_worker.lock().unwrap() = Some(rom_asm_worker);
    }

    /// Computes the witness for the provided plan using the given ROM.
    ///
    /// # Arguments
    /// * `rom` - Reference to the Zisk ROM.
    /// * `plan` - The execution plan for computing the witness.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness trace data.
    pub fn compute_witness<F: PrimeField>(
        rom: &ZiskRom,
        counter_stats: &CounterStats,
    ) -> AirInstance<F> {
        let mut rom_trace = RomTrace::new_zeroes();

        let main_trace_len = MainTrace::<F>::NUM_ROWS as u64;

        info!("{}: ··· Creating Rom instance [{} rows]", Self::MY_NAME, rom_trace.num_rows());

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
                    multiplicity = counter_stats.bios_inst_count
                        [((inst.paddr - ROM_ENTRY) as usize) >> 2]
                        .load(std::sync::atomic::Ordering::Relaxed)
                        as u64;

                    if multiplicity == 0 {
                        continue;
                    }
                    if inst.paddr == counter_stats.end_pc {
                        multiplicity += main_trace_len - counter_stats.steps % main_trace_len;
                    }
                }
            } else {
                multiplicity = counter_stats.prog_inst_count[(inst.paddr - ROM_ADDR) as usize]
                    .load(std::sync::atomic::Ordering::Relaxed)
                    as u64;
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

    pub fn compute_witness_from_asm<F: PrimeField>(
        rom: &ZiskRom,
        asm_romh: &AsmRHData,
    ) -> AirInstance<F> {
        let mut rom_trace = RomTrace::new_zeroes();

        info!("{}: ··· Creating Rom instance [{} rows]", Self::MY_NAME, rom_trace.num_rows());

        const MAIN_TRACE_LEN: u64 = MainTrace::<usize>::NUM_ROWS as u64;

        // if asm_romh.bios_inst_count.is_empty() {
        //     for (i, _) in rom.rom_entry_instructions.iter().enumerate() {
        //         rom_trace[i].multiplicity = F::ONE;
        //     }
        // } else {
        //     let extra = MAIN_TRACE_LEN - asm_romh.header.steps % MAIN_TRACE_LEN;

        //     for (i, inst) in rom.rom_entry_instructions.iter().enumerate() {
        //         let idx = ((inst.paddr - ROM_ENTRY) as usize) >> 2;

        //         let mut multiplicity = asm_romh.bios_inst_count[idx];

        //         if multiplicity != 0 {
        //             if inst.paddr == ROM_EXIT {
        //                 multiplicity += extra;
        //             }
        //             rom_trace[i].multiplicity = F::from_u64(multiplicity);
        //         }
        //     }
        // }

        // for (i, inst) in rom.rom_instructions.iter().enumerate() {
        //     let idx = (inst.paddr - ROM_ADDR) as usize;
        //     let multiplicity = asm_romh.prog_inst_count[idx];

        //     if multiplicity != 0 {
        //         rom_trace[i].multiplicity = F::from_u64(multiplicity);
        //     }
        // }

        // For every instruction in the rom, fill its corresponding ROM trace
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
                        multiplicity += MAIN_TRACE_LEN - asm_romh.header.steps % MAIN_TRACE_LEN;
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
    fn compute_trace_rom<F: PrimeField>(rom: &ZiskRom, rom_custom_trace: &mut RomRomTrace<F>) {
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
        for i in rom.insts.len()..rom_custom_trace.num_rows() {
            rom_custom_trace[i] = RomRomTraceRow::default();
        }
    }

    /// Computes a custom trace ROM from the given ELF file.
    ///
    /// # Arguments
    /// * `rom_path` - The path to the ELF file.
    /// * `rom_custom_trace` - Reference to the custom ROM trace.
    pub fn compute_custom_trace_rom<F: PrimeField>(
        rom_path: PathBuf,
        rom_custom_trace: &mut RomRomTrace<F>,
    ) {
        // Get the ELF file path as a string
        let elf_filename: String = rom_path.to_str().unwrap().into();
        info!("Computing custom trace ROM");

        // Load and parse the ELF file, and transpile it into a ZisK ROM using Riscv2zisk

        // Create an instance of the RISCV -> ZisK program converter
        let riscv2zisk = Riscv2zisk::new(elf_filename);

        // Convert program to rom
        let rom = riscv2zisk.run().expect("RomSM::prover() failed converting elf to rom");

        Self::compute_trace_rom(&rom, rom_custom_trace);
    }
}

impl<F: PrimeField> ComponentBuilder<F> for RomSM {
    /// Builds and returns a new counter for monitoring ROM operations.
    ///
    /// # Returns
    /// A boxed implementation of `RomCounter`.
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(RomCounter::new(self.bios_inst_count.clone(), self.prog_inst_count.clone()))
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
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn sm_common::Instance<F>> {
        let mut worker_guard = self.rom_asm_worker.lock().unwrap();
        let worker = worker_guard.take();

        Box::new(RomInstance::new(
            self.zisk_rom.clone(),
            ictx,
            self.bios_inst_count.clone(),
            self.prog_inst_count.clone(),
            worker,
        ))
    }
}
