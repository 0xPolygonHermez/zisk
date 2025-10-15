use std::path::PathBuf;

use fields::PrimeField64;
use libloading::{Library, Symbol};
use proofman_common::VerboseMode;
use zisk_common::{ZiskLib, ZiskLibInitFn};

use anyhow::Result;

use crate::get_witness_computation_lib;

#[derive(Default)]
pub struct ZiskLibLoader;

impl ZiskLibLoader {
    fn load_library<F: PrimeField64>(
        witness_lib: PathBuf,
        elf: PathBuf,
        world_rank: i32,
        local_rank: i32,
        verbose: VerboseMode,
        shared_tables: bool,
        asm_mt_filename: Option<PathBuf>,
        asm_rh_filename: Option<PathBuf>,
        base_port: Option<u16>,
        unlock_mapped_memory: Option<bool>,
    ) -> Result<(Library, Box<dyn ZiskLib<F>>)> {
        let lib_path = get_witness_computation_lib(Some(&witness_lib));
        let library = unsafe { Library::new(lib_path) }?;

        let witness_lib_constructor: Symbol<ZiskLibInitFn<F>> =
            unsafe { library.get(b"init_library")? };

        let witness_lib = witness_lib_constructor(
            verbose,
            elf,
            asm_mt_filename,
            asm_rh_filename,
            Some(world_rank),
            Some(local_rank),
            base_port,
            unlock_mapped_memory.unwrap_or(false),
            shared_tables,
        )
        .expect("Failed to initialize witness library");

        Ok((library, witness_lib))
    }

    pub fn load_emu<F: PrimeField64>(
        witness_lib: PathBuf,
        elf: PathBuf,
        world_rank: i32,
        local_rank: i32,
        verbose: VerboseMode,
        shared_tables: bool,
    ) -> Result<(Library, Box<dyn ZiskLib<F>>)> {
        Self::load_library(
            witness_lib,
            elf,
            world_rank,
            local_rank,
            verbose,
            shared_tables,
            None,
            None,
            None,
            None,
        )
    }

    pub fn load_asm<F: PrimeField64>(
        witness_lib: PathBuf,
        elf: PathBuf,
        world_rank: i32,
        local_rank: i32,
        verbose: VerboseMode,
        shared_tables: bool,
        asm_mt_filename: PathBuf,
        asm_rh_filename: PathBuf,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<(Library, Box<dyn ZiskLib<F>>)> {
        Self::load_library(
            witness_lib,
            elf,
            world_rank,
            local_rank,
            verbose,
            shared_tables,
            Some(asm_mt_filename),
            Some(asm_rh_filename),
            base_port,
            Some(unlock_mapped_memory),
        )
    }
}
