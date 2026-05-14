//! The `RomSM` module implements the ROM State Machine,
//! directly managing the ROM execution process, generating traces, and computing custom traces.
//!
//! Key components of this module include:
//! - The `RomSM` struct, which represents the ROM State Machine and encapsulates ROM-related
//!   operations.
//! - Methods for proving instances and computing traces from the ROM data.
//! - `ComponentBuilder` trait implementations for creating counters, planners, and input
//!   collectors.

use std::sync::{atomic::AtomicU64, Arc, Mutex, OnceLock};

use crate::{RomInstance, RomPlanner};
use asm_runner::{AsmRHData, AsmRunnerRH};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofmanResult, TraceInfo};
use zisk_common::{
    create_atomic_vec, ComponentBuilder, CounterStats, Instance, InstanceCtx, Planner,
};
use zisk_core::{
    zisk_ops::ZiskOp, Riscv2zisk, ZiskRom, ROM_ADDR, ROM_ADDR_MAX, ROM_ENTRY, ROM_EXIT, SRC_IMM,
};
use zisk_pil::{MainTrace, RomRomTrace, RomRomTraceRow, RomTrace};

use anyhow::Result;

/// The `RomSM` struct represents the ROM State Machine
pub struct RomSM {
    /// Zisk Rom
    zisk_rom: Mutex<Option<Arc<ZiskRom>>>,

    /// Shared bios instruction counter for monitoring ROM operations.
    /// Lazy-allocated on first emulator-mode `build_instance` call so that
    /// pure-ASM workflows don't pay the memory cost.
    bios_inst_count: OnceLock<Arc<Vec<AtomicU64>>>,

    /// Shared program instruction counter for monitoring ROM operations.
    /// Lazy-allocated alongside `bios_inst_count`.
    prog_inst_count: OnceLock<Arc<Vec<AtomicU64>>>,

    rh_data: Mutex<Option<AsmRunnerRH>>,
}

impl RomSM {
    /// Creates a new instance of the `RomSM` state machine.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `RomSM`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            zisk_rom: Mutex::new(None),
            bios_inst_count: OnceLock::new(),
            prog_inst_count: OnceLock::new(),
            rh_data: Mutex::new(None),
        })
    }

    /// Allocates the shared inst-count vectors on first use; returns Arc clones.
    /// Called from `build_instance` only when an emulator-mode (non-ASM) run is
    /// about to need them.
    fn ensure_inst_counts(&self) -> (Arc<Vec<AtomicU64>>, Arc<Vec<AtomicU64>>) {
        let bios = self
            .bios_inst_count
            .get_or_init(|| Arc::new(create_atomic_vec(((ROM_ADDR - ROM_ENTRY) as usize) >> 2)))
            .clone();
        let prog = self
            .prog_inst_count
            .get_or_init(|| Arc::new(create_atomic_vec((ROM_ADDR_MAX - ROM_ADDR) as usize)))
            .clone();
        (bios, prog)
    }

    pub fn set_rh_data(&self, handler: AsmRunnerRH) -> Result<()> {
        *self.rh_data.lock().map_err(|e| anyhow::anyhow!("Mutex stats lock poisoned: {e}"))? =
            Some(handler);
        Ok(())
    }

    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> Result<()> {
        *self.zisk_rom.lock().map_err(|e| anyhow::anyhow!("Mutex stats lock poisoned: {e}"))? =
            Some(zisk_rom);
        Ok(())
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
        mut trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let main_trace_len = MainTrace::<()>::NUM_ROWS as u64;

        tracing::debug!("··· Creating Rom instance [{} rows]", RomTrace::<F>::NUM_ROWS);

        // For every instruction in the rom, fill its corresponding ROM trace
        for zib in rom.insts.values() {
            // Get the Zisk instruction
            let inst = &zib.i;

            // Calculate the multiplicity, i.e. the number of times this pc is used in this
            // execution
            let mut multiplicity: u64;
            if inst.paddr < ROM_ADDR {
                if counter_stats.bios_inst_count.is_empty() {
                    multiplicity = 1; // If the histogram is empty, we use 1 for all pc's
                } else {
                    multiplicity = counter_stats.bios_inst_count
                        [((inst.paddr - ROM_ENTRY) as usize) >> 2]
                        .load(std::sync::atomic::Ordering::Relaxed);

                    if multiplicity == 0 {
                        continue;
                    }
                    if inst.paddr == counter_stats.end_pc {
                        multiplicity += main_trace_len - counter_stats.steps % main_trace_len;
                    }
                }
            } else {
                multiplicity = counter_stats.prog_inst_count[(inst.paddr - ROM_ADDR) as usize]
                    .load(std::sync::atomic::Ordering::Relaxed);
                if multiplicity == 0 {
                    continue;
                }
                if inst.paddr == counter_stats.end_pc {
                    multiplicity += main_trace_len - counter_stats.steps % main_trace_len;
                }
            }

            let index = inst.index as usize;
            debug_assert!(
                 index < trace_buffer.len(),
                 "ROM trace index {} out of bounds for trace_buffer len {} (RomTrace::NUM_ROWS = {})",
                 index,
                 trace_buffer.len(),
                 RomTrace::<F>::NUM_ROWS
            );
            trace_buffer[index] = F::from_u64(multiplicity);
        }

        Ok(AirInstance::new(TraceInfo::new(
            RomTrace::<F>::AIRGROUP_ID,
            RomTrace::<F>::AIR_ID,
            1,
            RomTrace::<F>::NUM_ROWS,
            trace_buffer,
            false,
            false,
        )))
    }

    pub fn compute_witness_from_asm<F: PrimeField64>(
        rom: &ZiskRom,
        asm_romh: &AsmRHData,
        mut trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        tracing::debug!("··· Creating Rom instance [{} rows]", RomTrace::<F>::NUM_ROWS);

        // Check that the provided histogram has at most as many entries as the ROM trace
        assert!(
            asm_romh.inst_count.len() <= RomTrace::<F>::NUM_ROWS,
            "The provided assembly histogram has {} entries, which exceeds the maximum supported by the Zisk PIL ROM trace ({} entries).  Please review zisk.pil and increase the ROM trace size accordingly.",
            asm_romh.inst_count.len(),
            RomTrace::<F>::NUM_ROWS
        );
        assert!(
            asm_romh.inst_count.len() <= trace_buffer.len(),
            "The provided assembly histogram has {} entries, but the trace buffer has only {} entries.",
            asm_romh.inst_count.len(),
            trace_buffer.len()
        );

        for (i, multiplicity) in asm_romh.inst_count.iter().enumerate() {
            if *multiplicity == 0 {
                continue;
            }
            trace_buffer[i] = F::from_u64(*multiplicity);
        }

        // Search for end instruction index
        let index = rom.get_instruction(ROM_EXIT).index as usize;
        assert!(
            index < trace_buffer.len(),
            "ROM trace index {} out of bounds for trace_buffer len {} (RomTrace::NUM_ROWS = {})",
            index,
            trace_buffer.len(),
            RomTrace::<F>::NUM_ROWS
        );
        assert!(
            F::is_one(&trace_buffer[index]),
            "The exit instruction should have been executed once in the assembly execution"
        );

        // Increment it as if it was executed the number of times needed to reach the end of the
        // main trace instance, i.e. we repeat the last instruction until the end of the instance
        let main_trace_len = MainTrace::<()>::NUM_ROWS as u64;
        trace_buffer[index] = F::from_u64(1 + main_trace_len - asm_romh.steps % main_trace_len);

        Ok(AirInstance::new(TraceInfo::new(
            RomTrace::<F>::AIRGROUP_ID,
            RomTrace::<F>::AIR_ID,
            1,
            RomTrace::<F>::NUM_ROWS,
            trace_buffer,
            false,
            false,
        )))
    }

    /// Computes the ROM trace based on the ROM instructions.
    ///
    /// # Arguments
    /// * `rom` - Reference to the Zisk ROM.
    /// * `rom_custom_trace` - Reference to the custom ROM trace.
    fn compute_trace_rom<F: PrimeField64>(rom: &ZiskRom, rom_custom_trace: &mut RomRomTrace<F>) {
        // For every instruction in the rom, fill its corresponding ROM trace
        for (_pc, zib) in rom.insts.iter() {
            // Get the ZisK instruction
            let inst = &zib.i;

            // Get the ZisK instruction index
            let index = inst.index as usize;
            debug_assert!(
                index < RomRomTrace::<F>::NUM_ROWS,
                "ROM instruction index {} out of bounds for ROM trace with {} rows",
                index,
                RomRomTrace::<F>::NUM_ROWS
            );

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
        let rh_data = self.rh_data.lock().unwrap().take();
        let (bios_inst_count, prog_inst_count) = if rh_data.is_some() {
            // ASM mode for this run: RomInstance routes to compute_witness_from_asm,
            // never reading these vecs. Pass empty Arcs to avoid the lazy allocation.
            (Arc::new(Vec::new()), Arc::new(Vec::new()))
        } else {
            // Emulator mode: allocate on first need; subsequent emulator-mode runs reuse
            // (RomInstance::reset zeroes them between executions).
            self.ensure_inst_counts()
        };
        Box::new(RomInstance::new(
            self.zisk_rom.lock().unwrap().as_ref().unwrap().clone(),
            ictx,
            bios_inst_count,
            prog_inst_count,
            rh_data,
        ))
    }
}
