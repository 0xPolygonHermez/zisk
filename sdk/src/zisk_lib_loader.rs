use std::path::PathBuf;

use fields::PrimeField64;
use proofman_common::VerboseMode;
use zisk_witness::{init_zisk_lib, WitnessLib};

use anyhow::Result;

#[derive(Default)]
pub struct ZiskLibLoader;

impl ZiskLibLoader {
    #[allow(clippy::too_many_arguments)]
    fn load_library<F: PrimeField64>(
        elf: PathBuf,
        verbose: VerboseMode,
        shared_tables: bool,
        asm_mt_filename: Option<PathBuf>,
        asm_rh_filename: Option<PathBuf>,
        base_port: Option<u16>,
        unlock_mapped_memory: Option<bool>,
        with_hints: bool,
    ) -> Result<WitnessLib<F>> {
        let witness_lib = init_zisk_lib(
            verbose,
            elf,
            asm_mt_filename,
            asm_rh_filename,
            base_port,
            unlock_mapped_memory.unwrap_or(false),
            shared_tables,
            with_hints,
        );

        Ok(witness_lib)
    }

    pub fn load_emu<F: PrimeField64>(
        elf: PathBuf,
        verbose: VerboseMode,
        shared_tables: bool,
    ) -> Result<WitnessLib<F>> {
        Self::load_library(elf, verbose, shared_tables, None, None, None, None, false)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn load_asm<F: PrimeField64>(
        elf: PathBuf,
        verbose: VerboseMode,
        shared_tables: bool,
        asm_mt_filename: PathBuf,
        asm_rh_filename: PathBuf,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        with_hints: bool,
    ) -> Result<WitnessLib<F>> {
        Self::load_library(
            elf,
            verbose,
            shared_tables,
            Some(asm_mt_filename),
            Some(asm_rh_filename),
            base_port,
            Some(unlock_mapped_memory),
            with_hints,
        )
    }
}
