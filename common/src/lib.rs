mod bus;
mod component;
mod emu_minimal_trace;
mod executor_stats;
mod instance_context;
pub mod io;
mod mpi_context;
mod planner_helpers;
mod proof;
mod proof_log;
mod regular_counters;
mod regular_planner;
mod types;
mod utils;
mod zisk_lib_init;

pub use bus::*;
pub use component::*;
pub use emu_minimal_trace::*;
pub use executor_stats::*;
pub use instance_context::*;
pub use mpi_context::*;
pub use planner_helpers::*;
pub use proof::*;
pub use proof_log::*;
pub use regular_counters::*;
pub use regular_planner::*;
pub use types::*;
pub use utils::*;
pub use zisk_lib_init::*;

pub struct ElfInfo {
    elf_hash: &'static str,
    pilout_hash: &'static str,
    rom_setup_num_rows: u64,
    rom_setup_blowup_factor: usize,
}

// Now I'd like to compose this name as a file name
impl ElfInfo {
    pub fn custom_commits_filename(&self) -> String {
        format!(
            "{}_{}_{}_{}.bin",
            self.elf_hash, self.pilout_hash, self.rom_setup_num_rows, self.rom_setup_blowup_factor
        )
    }

    pub fn asm_mo_filename(&self) -> String {
        format!("{}-mo.bin", self.elf_hash)
    }

    pub fn asm_mt_filename(&self) -> String {
        format!("{}-mt.bin", self.elf_hash)
    }

    pub fn asm_rh_filename(&self) -> String {
        format!("{}-rh.bin", self.elf_hash)
    }
}
