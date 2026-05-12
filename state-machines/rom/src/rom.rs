//! The `RomSM` module implements the ROM State Machine,
//! directly managing the ROM execution process, generating traces, and computing custom traces.
//!
//! Key components of this module include:
//! - The `RomSM` struct, which represents the ROM State Machine and encapsulates ROM-related
//!   operations.
//! - Methods for proving instances and computing traces from the ROM data.
//! - `ComponentBuilder` trait implementations for creating counters, planners, and input
//!   collectors.

use std::sync::{atomic::AtomicU64, Arc, Mutex};

use crate::{RomInstance, RomPlanner};
use asm_runner::AsmRunnerRH;
use fields::PrimeField64;
use itertools::Itertools;
use zisk_common::{create_atomic_vec, ComponentBuilder, Instance, InstanceCtx, Planner};
use zisk_core::{
    zisk_ops::ZiskOp, Riscv2zisk, ZiskRom, ROM_ADDR, ROM_ADDR_MAX, ROM_ENTRY, SRC_IMM,
};
use zisk_pil::{RomRomTrace, RomRomTraceRow, RomTrace};

use anyhow::Result;

/// Per-emulator instruction counters for ROM histogram tracking.
///
/// The two emulators produce semantically different histograms:
/// - Rust traces by *physical address*, split into two regions (BIOS / program).
/// - ASM produces a single histogram already indexed by *instruction index* via `AsmRunnerRH`.
enum RomCounters {
    /// Rust emulator: counters indexed by physical address.
    Rust {
        /// BIOS region.
        bios: Arc<Vec<AtomicU64>>,
        /// Program region.
        prog: Arc<Vec<AtomicU64>>,
    },
    /// ASM emulator: histogram is delivered by the assembly runner thread.
    Asm { rh_data: Mutex<Option<AsmRunnerRH>> },
}

/// The `RomSM` struct represents the ROM State Machine
pub struct RomSM {
    /// Zisk Rom
    zisk_rom: Mutex<Option<Arc<ZiskRom>>>,

    /// Instruction counters — depends on the emulator backend.
    counters: RomCounters,
}

impl RomSM {
    /// Creates a new instance of the `RomSM` state machine.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `RomSM`.
    pub fn new(is_asm_emulator: bool) -> Arc<Self> {
        let counters = if is_asm_emulator {
            RomCounters::Asm { rh_data: Mutex::new(None) }
        } else {
            RomCounters::Rust {
                // BIOS region: address-aligned, can divide by 4.
                bios: Arc::new(create_atomic_vec(((ROM_ADDR - ROM_ENTRY) as usize) >> 2)),
                // Program region: cannot be divided by 4.
                prog: Arc::new(create_atomic_vec((ROM_ADDR_MAX - ROM_ADDR) as usize)),
            }
        };

        Arc::new(Self { zisk_rom: Mutex::new(None), counters })
    }

    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> Result<()> {
        *self.zisk_rom.lock().map_err(|e| anyhow::anyhow!("Mutex stats lock poisoned: {e}"))? =
            Some(zisk_rom);
        Ok(())
    }

    pub fn set_rh_data(&self, handler: AsmRunnerRH) -> Result<()> {
        match &self.counters {
            RomCounters::Asm { rh_data } => {
                *rh_data.lock().map_err(|e| anyhow::anyhow!("Mutex stats lock poisoned: {e}"))? =
                    Some(handler);
                Ok(())
            }
            RomCounters::Rust { .. } => Err(anyhow::anyhow!("set_rh_data called on non-ASM RomSM")),
        }
    }

    /// Computes a custom trace ROM from the given ELF file.
    ///
    /// # Arguments
    /// * `rom_path` - The path to the ELF file.
    /// * `rom_custom_trace` - Reference to the custom ROM trace.
    pub fn compute_custom_trace_rom<F: PrimeField64>(
        elf: &[u8],
        rom_custom_trace: &mut RomRomTrace<F>,
    ) {
        tracing::info!("Computing custom trace ROM");

        // Load and parse the ELF file, and transpile it into a ZisK ROM using Riscv2zisk

        // Create an instance of the RISCV -> ZisK program converter
        let riscv2zisk = Riscv2zisk::new(elf);

        // Convert program to rom
        let rom = riscv2zisk.run().expect("RomSM::prover() failed converting elf to rom");

        let rom_len = rom.insts.len();
        let air_rom_len = RomTrace::<F>::NUM_ROWS;
        if rom_len > air_rom_len {
            panic!(
                "Error: The generated ROM has {} instructions, which exceeds the maximum supported by the Zisk PIL ROM trace ({} instructions).  Please review zisk.pil and increase the ROM trace size accordingly.",
                rom_len, air_rom_len
            );
        }

        Self::compute_trace_rom(&rom, rom_custom_trace);
    }

    /// Computes the ROM trace based on the ROM instructions.
    ///
    /// # Arguments
    /// * `rom` - Reference to the Zisk ROM.
    /// * `rom_custom_trace` - Reference to the custom ROM trace.
    fn compute_trace_rom<F: PrimeField64>(rom: &ZiskRom, rom_custom_trace: &mut RomRomTrace<F>) {
        // For every instruction in the rom, fill its corresponding ROM trace
        for key in rom.insts.keys().sorted() {
            // Get the ZisK instruction
            let inst = &rom.insts[key].i;

            // Get the ZisK instruction index
            let index = inst.index as usize;

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
            rom_custom_trace[index].line = F::from_u64(inst.paddr); // TODO: unify names: pc, paddr, line
            rom_custom_trace[index].a_offset_imm0 = a_offset_imm0;
            rom_custom_trace[index].a_imm1 =
                F::from_u64(if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0 });
            rom_custom_trace[index].b_offset_imm0 = b_offset_imm0;
            rom_custom_trace[index].b_imm1 =
                F::from_u64(if inst.b_src == SRC_IMM { inst.b_use_sp_imm1 } else { 0 });
            rom_custom_trace[index].ind_width = F::from_u64(inst.ind_width);
            // IMPORTANT: the opcodes fcall, fcall_get, and fcall_param are really a variant
            // of the copyb, use to get free-input information
            rom_custom_trace[index].op = if inst.op == ZiskOp::Fcall.code()
                || inst.op == ZiskOp::FcallGet.code()
                || inst.op == ZiskOp::FcallParam.code()
            {
                F::from_u8(ZiskOp::CopyB.code())
            } else {
                F::from_u8(inst.op)
            };
            rom_custom_trace[index].store_offset = store_offset;
            rom_custom_trace[index].jmp_offset1 = jmp_offset1;
            rom_custom_trace[index].jmp_offset2 = jmp_offset2;
            rom_custom_trace[index].flags = F::from_u64(inst.get_flags());
        }

        // Padd with zeroes
        let num_rows: usize = RomRomTrace::<F>::NUM_ROWS;
        for i in rom.insts.len()..num_rows {
            rom_custom_trace[i] = RomRomTraceRow::default();
        }
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for RomSM {
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
        let zisk_rom = self.zisk_rom.lock().unwrap().as_ref().unwrap().clone();
        match &self.counters {
            RomCounters::Rust { bios, prog } => {
                Box::new(RomInstance::new_rust(zisk_rom, ictx, bios.clone(), prog.clone()))
            }
            RomCounters::Asm { rh_data } => {
                Box::new(RomInstance::new_asm(zisk_rom, ictx, rh_data.lock().unwrap().take()))
            }
        }
    }
}
