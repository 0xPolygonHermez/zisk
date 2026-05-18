//! The `RomSM` module implements the ROM State Machine,
//! directly managing the ROM execution process, generating traces, and computing custom traces.
//!
//! Key components of this module include:
//! - The `RomSM` struct, which represents the ROM State Machine and encapsulates ROM-related
//!   operations.
//! - Methods for proving instances and computing traces from the ROM data.
//! - `ComponentBuilder` trait implementations for creating counters, planners, and input
//!   collectors.rm

use std::sync::{atomic::AtomicU64, Arc, Mutex, OnceLock};

use crate::{RomInstance, RomPlanner};
use asm_runner::AsmRunnerRH;
use fields::PrimeField64;
use zisk_common::{create_atomic_vec, ComponentBuilder, Instance, InstanceCtx, Planner};
use zisk_core::{zisk_ops::ZiskOp, Riscv2zisk, ZiskRom, SRC_IMM};
use zisk_pil::{RomRomTrace, RomRomTraceRow, RomTrace};

/// The `RomSM` struct represents the ROM State Machine
pub struct RomSM {
    /// Zisk Rom, set once via [`set_rom`](Self::set_rom) before the first `build_instance` call.
    zisk_rom: OnceLock<Arc<ZiskRom>>,

    /// Shared program instruction counter for monitoring ROM operations.
    inst_count: Arc<Vec<AtomicU64>>,

    /// ASM-runner ROM histogram, set via [`set_rh_data`](Self::set_rh_data) when running in ASM mode.
    rh_data: Mutex<Option<AsmRunnerRH>>,
}

impl RomSM {
    /// Creates a new instance of the `RomSM` state machine.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `RomSM`.
    pub fn new<F: PrimeField64>() -> Arc<Self> {
        Arc::new(Self {
            zisk_rom: OnceLock::new(),
            inst_count: Arc::new(create_atomic_vec(RomTrace::<F>::NUM_ROWS)),
            rh_data: Mutex::new(None),
        })
    }

    /// Provides the ASM-runner ROM histogram. Must be called before `build_instance`
    /// when running in ASM mode; in Rust mode it is not called at all.
    pub fn set_rh_data(&self, handler: AsmRunnerRH) {
        *self.rh_data.lock().expect("RomSM rh_data mutex poisoned") = Some(handler);
    }

    /// Provides the parsed Zisk ROM. Must be called exactly once before `build_instance`.
    ///
    /// # Panics
    /// Panics if called more than once.
    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) {
        self.zisk_rom
            .set(zisk_rom)
            .map_err(|_| ())
            .expect("RomSM::set_rom called more than once");
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

            // Fill the rom trace row fields
            rom_custom_trace[index].line = F::from_u64(inst.paddr); // TODO: unify names: pc, paddr, line
            rom_custom_trace[index].a_offset_imm0 = signed_to_field(inst.a_offset_imm0 as i64);
            rom_custom_trace[index].a_imm1 =
                F::from_u64(if inst.a_src == SRC_IMM { inst.a_use_sp_imm1 } else { 0 });
            rom_custom_trace[index].b_offset_imm0 = signed_to_field(inst.b_offset_imm0 as i64);
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
            rom_custom_trace[index].store_offset = signed_to_field(inst.store_offset);
            rom_custom_trace[index].jmp_offset1 = signed_to_field(inst.jmp_offset1);
            rom_custom_trace[index].jmp_offset2 = signed_to_field(inst.jmp_offset2);
            rom_custom_trace[index].flags = F::from_u64(inst.get_flags());
        }

        // Padd with zeroes
        let num_rows: usize = RomRomTrace::<F>::NUM_ROWS;
        for i in rom.insts.len()..num_rows {
            rom_custom_trace[i] = RomRomTraceRow::default();
        }
    }

    /// Computes a custom trace ROM from the given ELF bytes.
    ///
    /// # Arguments
    /// * `elf` - The ELF bytes.
    /// * `rom_custom_trace` - Reference to the custom ROM trace to populate.
    pub fn compute_custom_trace_rom<F: PrimeField64>(
        elf: &[u8],
        rom_custom_trace: &mut RomRomTrace<F>,
    ) {
        tracing::info!("Computing custom trace ROM");

        // Load and parse the ELF file, and transpile it into a ZisK ROM using Riscv2zisk

        // Create an instance of the RISCV -> ZisK program converter
        let riscv2zisk = Riscv2zisk::new(elf);

        // Convert program to rom
        let rom = riscv2zisk
            .run()
            .expect("RomSM::compute_custom_trace_rom failed converting elf to rom");

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

/// Converts a signed integer to a field element, mapping negatives through `F::neg`.
#[inline]
fn signed_to_field<F: PrimeField64>(v: i64) -> F {
    if v >= 0 {
        F::from_u64(v as u64)
    } else {
        F::neg(F::from_u64((-v) as u64))
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
        let zisk_rom = self
            .zisk_rom
            .get()
            .expect("RomSM::build_instance called before set_rom")
            .clone();
        if let Some(rh_data) = self.rh_data.lock().unwrap().take() {
            Box::new(RomInstance::new_asm(zisk_rom, ictx, rh_data))
        } else {
            Box::new(RomInstance::new_rust(zisk_rom, ictx, self.inst_count.clone()))
        }
    }
}
