use std::path::PathBuf;

use fields::PrimeField64;
use libloading::{Library, Symbol};
use proofman_common::VerboseMode;
use zisk_common::{ZiskLib, ZiskLibInitFn};

use anyhow::Result;

use crate::get_witness_computation_lib;

#[derive(Default)]
pub struct DyLibBuilder {
    witness_lib: Option<PathBuf>,
    verbose: u8,
    elf_path: Option<PathBuf>,
    world_rank: Option<i32>,
    local_rank: Option<i32>,
    shared_tables: bool,
    // Optional parameters for assembly
    asm_path: Option<PathBuf>,
    asm_rom_path: Option<PathBuf>,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,
}

impl DyLibBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_witness_lib(mut self, path: Option<PathBuf>) -> Self {
        self.witness_lib = path;
        self
    }

    pub fn with_verbose(mut self, level: u8) -> Self {
        self.verbose = level;
        self
    }

    pub fn with_elf_path(mut self, path: Option<PathBuf>) -> Self {
        self.elf_path = path;
        self
    }

    pub fn with_world_rank(mut self, rank: i32) -> Self {
        self.world_rank = Some(rank);
        self
    }

    pub fn with_local_rank(mut self, rank: i32) -> Self {
        self.local_rank = Some(rank);
        self
    }

    pub fn with_shared_tables(mut self, shared: bool) -> Self {
        self.shared_tables = shared;
        self
    }

    pub fn with_asm_path(mut self, path: Option<PathBuf>) -> Self {
        self.asm_path = path;
        self
    }

    pub fn with_asm_rom_path(mut self, path: Option<PathBuf>) -> Self {
        self.asm_rom_path = path;
        self
    }

    pub fn with_base_port(mut self, port: Option<u16>) -> Self {
        self.base_port = port;
        self
    }

    pub fn with_unlock_mapped_memory(mut self, unlock: bool) -> Self {
        self.unlock_mapped_memory = unlock;
        self
    }

    pub fn build<F: PrimeField64>(&self) -> Result<(Library, Box<dyn ZiskLib<F>>)> {
        if self.asm_path.is_some() {
            // Build assembly library
            self.new_asm()
        } else {
            // Build emulator library
            self.new_emu()
        }
    }

    fn new_emu<F: PrimeField64>(&self) -> Result<(Library, Box<dyn ZiskLib<F>>)> {
        // Check that all required parameters are set
        let verbose: VerboseMode = self.verbose.into();
        let elf_path =
            self.elf_path.clone().ok_or_else(|| anyhow::anyhow!("elf_path is required"))?;
        let world_rank =
            self.world_rank.ok_or_else(|| anyhow::anyhow!("world_rank is required"))?;
        let local_rank =
            self.local_rank.ok_or_else(|| anyhow::anyhow!("local_rank is required"))?;
        let shared_tables = self.shared_tables;

        let library =
            unsafe { Library::new(get_witness_computation_lib(self.witness_lib.as_ref()))? };

        let witness_lib_constructor: Symbol<ZiskLibInitFn<F>> =
            unsafe { library.get(b"init_library")? };

        let witness_lib = witness_lib_constructor(
            verbose,
            elf_path,
            None,
            None,
            Some(world_rank),
            Some(local_rank),
            None,
            false,
            shared_tables,
        )
        .expect("Failed to initialize witness library");

        Ok((library, witness_lib))
    }

    fn new_asm<F: PrimeField64>(&self) -> Result<(Library, Box<dyn ZiskLib<F>>)> {
        // Check that all required parameters are set
        let verbose: VerboseMode = self.verbose.into();
        let elf_path =
            self.elf_path.clone().ok_or_else(|| anyhow::anyhow!("elf_path is required"))?;
        let asm_path = self
            .asm_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("asm_path is required for assembly"))?;
        let asm_rom_path = self
            .asm_rom_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("asm_rom_path is required for assembly"))?;
        let world_rank =
            self.world_rank.ok_or_else(|| anyhow::anyhow!("world_rank is required"))?;
        let local_rank =
            self.local_rank.ok_or_else(|| anyhow::anyhow!("local_rank is required"))?;
        let unlock_mapped_memory = self.unlock_mapped_memory;
        let shared_tables = self.shared_tables;

        let library =
            unsafe { Library::new(get_witness_computation_lib(self.witness_lib.as_ref()))? };

        let witness_lib_constructor: Symbol<ZiskLibInitFn<F>> =
            unsafe { library.get(b"init_library")? };

        let witness_lib = witness_lib_constructor(
            verbose,
            elf_path,
            Some(asm_path),
            Some(asm_rom_path),
            Some(world_rank),
            Some(local_rank),
            self.base_port,
            unlock_mapped_memory,
            shared_tables,
        )
        .expect("Failed to initialize witness library");

        Ok((library, witness_lib))
    }
}
