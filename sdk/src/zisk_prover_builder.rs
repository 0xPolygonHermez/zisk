use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::Result;
use asm_runner::{AsmRunnerOptions, AsmServices};
use colored::Colorize;
use fields::{ExtensionField, GoldilocksQuinticExtension, PrimeField64};
use proofman::ProofMan;
use proofman_common::{initialize_logger, json_to_debug_instances_map, DebugInfo, ParamsGPU};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use tracing::info;

use crate::{get_proving_key, get_witness_computation_lib, zisk_prover::ZiskProver, DyLibBuilder};

#[derive(Default)]
pub struct ZiskProverBuilder {
    witness_lib: Option<PathBuf>,
    proving_key: Option<PathBuf>,

    debug_info: Option<Option<String>>,
    verbose: u8,

    elf_path: Option<PathBuf>,
    shared_tables: bool,
    // Optional parameters for assembly
    emulator: bool,
    asm_path: Option<PathBuf>,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,

    print_command_info: bool,
}

impl ZiskProverBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_witness_lib_path(mut self, witness_lib: Option<PathBuf>) -> Self {
        self.witness_lib = witness_lib;
        self
    }

    pub fn with_proving_key_path(mut self, proving_key: Option<PathBuf>) -> Self {
        self.proving_key = proving_key;
        self
    }

    pub fn with_debug_info(mut self, debug_info: Option<Option<String>>) -> Self {
        self.debug_info = debug_info;
        self
    }

    pub fn with_verbose(mut self, verbose: u8) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn with_elf_path(mut self, elf_path: Option<PathBuf>) -> Self {
        self.elf_path = elf_path;
        self
    }

    pub fn with_shared_tables(mut self, shared_tables: bool) -> Self {
        self.shared_tables = shared_tables;
        self
    }

    pub fn with_emulator(mut self, emu: bool) -> Self {
        self.emulator = emu;
        self
    }

    pub fn with_asm_path(mut self, path: Option<PathBuf>) -> Self {
        self.asm_path = path;
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

    pub fn with_command_info(mut self) -> Self {
        self.print_command_info = true;
        self
    }

    pub fn build_emulator<F>(&mut self, elf_path: PathBuf) -> Result<ZiskProver<F>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        self.emulator = true;
        self.elf_path = Some(elf_path);
        self.build()
    }

    pub fn build_asm<F>(
        &mut self,
        elf_path: PathBuf,
        asm_path: PathBuf,
        base_port: u16,
        unlock_mapped_memory: bool,
    ) -> Result<ZiskProver<F>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        self.emulator = false;
        self.elf_path = Some(elf_path);
        self.asm_path = Some(asm_path);
        self.base_port = Some(base_port);
        self.unlock_mapped_memory = unlock_mapped_memory;
        self.build()
    }

    pub fn build<F>(&mut self) -> Result<ZiskProver<F>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        let proving_key = get_proving_key(self.proving_key.as_ref());

        let default_cache_path =
            std::env::var("HOME").ok().map(PathBuf::from).unwrap().join(DEFAULT_CACHE_PATH);

        if !default_cache_path.exists() {
            if let Err(e) = fs::create_dir_all(default_cache_path.clone()) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    // prevent collision in distributed mode
                    panic!("Failed to create the cache directory: {e:?}");
                }
            }
        }

        let mut asm_rom_path = None;
        let elf_path = self.elf_path.clone().expect("ELF path is required");

        if self.emulator {
            self.asm_path = None;
        } else if self.asm_path.is_none() {
            let stem = elf_path.file_stem().unwrap().to_str().unwrap();
            let hash = get_elf_data_hash(&elf_path)
                .map_err(|e| anyhow::anyhow!("Error computing ELF hash: {}", e))?;
            let new_filename = format!("{stem}-{hash}-mt.bin");
            let asm_rom_filename = format!("{stem}-{hash}-rh.bin");
            asm_rom_path = Some(default_cache_path.join(asm_rom_filename));
            self.asm_path = Some(default_cache_path.join(new_filename));
        }

        if let Some(asm_path) = &self.asm_path {
            if !asm_path.exists() {
                return Err(anyhow::anyhow!("ASM file not found at {:?}", asm_path.display()));
            }
        }

        if let Some(asm_rom) = &asm_rom_path {
            if !asm_rom.exists() {
                return Err(anyhow::anyhow!("ASM file not found at {:?}", asm_rom.display()));
            }
        }

        let blowup_factor = get_rom_blowup_factor(&proving_key);

        let rom_bin_path =
            get_elf_bin_file_path(&elf_path.to_path_buf(), &default_cache_path, blowup_factor)?;

        if !rom_bin_path.exists() {
            let _ = gen_elf_hash(&elf_path, rom_bin_path.as_path(), blowup_factor, false)
                .map_err(|e| anyhow::anyhow!("Error generating elf hash: {}", e));
        }

        if self.print_command_info {
            self.print_command_info();
        }

        let mut custom_commits_map: HashMap<String, PathBuf> = HashMap::new();
        custom_commits_map.insert("rom".to_string(), rom_bin_path);

        let proofman = ProofMan::<F>::new(
            proving_key.clone(),
            custom_commits_map,
            true,
            false,
            false,
            ParamsGPU::default(),
            self.verbose.into(),
        )
        .expect("Failed to initialize proofman");

        let mpi_ctx = proofman.get_mpi_ctx();
        let world_rank = mpi_ctx.rank;
        let local_rank = mpi_ctx.node_rank;

        initialize_logger(self.verbose.into(), Some(world_rank));

        let asm_services = if !self.emulator {
            info!(">>> [{}] Starting ASM microservices.", world_rank);

            let asm_services = AsmServices::new(world_rank, local_rank, self.base_port);

            let asm_runner_options = AsmRunnerOptions::new()
                .with_verbose(self.verbose > 0)
                .with_base_port(self.base_port)
                .with_world_rank(world_rank)
                .with_local_rank(local_rank)
                .with_unlock_mapped_memory(self.unlock_mapped_memory);

            asm_services.start_asm_services(self.asm_path.as_ref().unwrap(), asm_runner_options)?;

            Some(asm_services)
        } else {
            None
        };

        let (library, mut witness_lib) = if !self.emulator {
            // Build assembly library
            DyLibBuilder::new()
                .with_witness_lib(self.witness_lib.clone())
                .with_verbose(self.verbose)
                .with_elf_path(self.elf_path.clone())
                .with_asm_path(self.asm_path.clone())
                .with_asm_rom_path(asm_rom_path.clone())
                .with_world_rank(world_rank)
                .with_local_rank(local_rank)
                .with_base_port(self.base_port)
                .with_unlock_mapped_memory(self.unlock_mapped_memory)
                .with_shared_tables(self.shared_tables)
                .build()?
        } else {
            // Build emulator library
            DyLibBuilder::new()
                .with_witness_lib(self.witness_lib.clone())
                .with_verbose(self.verbose)
                .with_elf_path(self.elf_path.clone())
                .with_world_rank(world_rank)
                .with_local_rank(local_rank)
                .with_shared_tables(self.shared_tables)
                .build()?
        };

        proofman.register_witness(&mut *witness_lib, library);

        let debug_info = match &self.debug_info {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                json_to_debug_instances_map(proving_key, debug_value.clone())
            }
        };

        ZiskProver::new(witness_lib, world_rank, proofman, asm_services, debug_info)
    }

    fn print_command_info(&self) {
        // Print Verify Constraints command info
        println!("{} VerifyConstraints", format!("{: >12}", "Command").bright_green().bold());
        println!(
            "{: >12} {}",
            "Witness Lib".bright_green().bold(),
            get_witness_computation_lib(self.witness_lib.as_ref()).display()
        );

        println!(
            "{: >12} {}",
            "Elf".bright_green().bold(),
            self.elf_path.as_ref().unwrap().display()
        );

        if self.asm_path.is_some() {
            let asm_path = self.asm_path.as_ref().unwrap().display();
            println!("{: >12} {}", "ASM runner".bright_green().bold(), asm_path);
        } else {
            println!(
                "{: >12} {}",
                "Emulator".bright_green().bold(),
                "Running in emulator mode".bright_yellow()
            );
        }

        println!(
            "{: >12} {}",
            "Proving key".bright_green().bold(),
            get_proving_key(self.proving_key.as_ref()).display()
        );

        let std_mode = if self.debug_info.is_some() { "Debug mode" } else { "Standard mode" };
        println!("{: >12} {}", "STD".bright_green().bold(), std_mode);

        println!();
    }
}
