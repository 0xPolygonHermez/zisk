//! This module provides the ZisK zkVM Prover interface.

use crate::common::{get_home_dir, Field, OutputPath, ZiskLibInitFn};
use crate::proof_log;
use crate::prove::{ProveConfig, ProveContext};

use anyhow::Result;
use colored::Colorize;
use executor::ZiskExecutionResult;
use libloading::{Library, Symbol};
use log::info;
use p3_goldilocks::Goldilocks;
use proofman::ProofMan;
use proofman_common::{initialize_logger, ModeName, ProofOptions};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use std::{collections::HashMap, fs, path::PathBuf};
use witness::WitnessLibrary;

/// Main ZisK zkVM Prover interface.
#[derive(Clone, Default)]
pub struct Prover {}

impl Prover {
    /// Creates a new ZisK zkVM Prover instance
    pub fn new() -> Self {
        Prover::default()
    }

    fn default_cache_path() -> PathBuf {
        PathBuf::from(format!("{}/{}", get_home_dir(), DEFAULT_CACHE_PATH))
    }

    fn get_asm_files(context: &ProveContext) -> Result<Option<PathBuf>, ProverError> {
        let mut asm: Option<PathBuf> = None;

        // If emulator is not enabled and no ASM file is provided
        if !context.config.emulator && context.config.asm.is_none() {
            let stem = context.elf.file_stem().unwrap().to_str().unwrap();

            let hash = get_elf_data_hash(&context.elf)
                .map_err(|e| ProverError::ElfHashError(e.to_string()))?;

            let asm_filename = format!("{stem}-{hash}.bin");
            asm = Some(Self::default_cache_path().join(&asm_filename));
        }

        // If the ASM file is set, check if it exists
        if let Some(path) = &asm {
            if !path.exists() {
                return Err(ProverError::AsmLoadError(path.to_string_lossy().into()));
            }
        }

        Ok(asm)
    }

    fn get_elf_bin_path(context: &ProveContext) -> Result<PathBuf, ProverError> {
        let blowup_factor = get_rom_blowup_factor(context.config.proving_key.as_ref());

        let elf_bin_path =
            get_elf_bin_file_path(&context.elf, &Self::default_cache_path(), blowup_factor)
                .map_err(|e| ProverError::ElfLoadError(e.to_string()))?;

        if !elf_bin_path.exists() {
            let _ = gen_elf_hash(&context.elf, &elf_bin_path, blowup_factor, false)
                .map_err(|e| ProverError::ElfHashError(e.to_string()))?;
        }

        Ok(elf_bin_path)
    }

    fn cleanup_output_dir(output_dir: &OutputPath) {
        let proofs_dir = output_dir.as_ref().join("proofs");
        if proofs_dir.exists() {
            // In distributed mode two different processes may enter here at the same time and try to remove the same directory
            if let Err(e) = fs::remove_dir_all(&proofs_dir) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    panic!("Failed to remove the proofs directory: {:?}", e);
                }
            }
        }

        if let Err(e) = fs::create_dir_all(&proofs_dir) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                // prevent collision in distributed mode
                panic!("Failed to create the proofs directory: {:?}", e);
            }
        }
    }

    fn create_cache_dir() {
        let default_cache_path = Self::default_cache_path();

        if default_cache_path.exists() {
            if let Err(e) = fs::create_dir_all(default_cache_path.clone()) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    // prevent collision in distributed mode
                    panic!("Failed to create the cache directory: {:?}", e);
                }
            }
        }
    }

    fn load_witness_lib(
        context: &ProveContext,
    ) -> Result<(Library, Box<dyn WitnessLibrary<Goldilocks>>), ProverError> {
        match context.config.field {
            Field::Goldilocks => {
                let witness_lib_pathbuf: PathBuf = context.config.witness_lib.clone().into();
                let library = unsafe {
                    Library::new(witness_lib_pathbuf)
                        .map_err(|e| ProverError::WitnessLoadError(e.to_string()))?
                };
                let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> = unsafe {
                    library
                        .get(b"init_library")
                        .map_err(|e| ProverError::WitnessLoadError(e.to_string()))?
                };

                let witness_lib = witness_lib_constructor(
                    context.config.verbose.into(),
                    context.elf.clone(),
                    context.asm_path.clone(),
                    context.input.clone(),
                    context.config.keccak_script.clone().into(),
                )
                .expect("Failed to initialize witness library");

                Ok((library, witness_lib))
            }
        }
    }

    /// Runs the proof generation process.
    pub fn prove(
        &self,
        elf: PathBuf,
        input: Option<PathBuf>,
        config: Option<ProveConfig>,
    ) -> Result<()> {
        // Define the context for the proving process
        let mut context = ProveContext {
            elf,
            input,
            config: config.unwrap_or_else(ProveConfig::new),
            ..Default::default()
        };
        context.asm_path = Self::get_asm_files(&context)?;
        context.elf_bin_path = Self::get_elf_bin_path(&context)?;

        // Initialize the logger
        initialize_logger(context.config.verbose.into());

        let start = std::time::Instant::now();

        // Clean up the output directory and create the cache directory (if needed)
        Self::cleanup_output_dir(&context.config.output_dir);
        Self::create_cache_dir();

        // Print the context information
        context.print();

        // Define the custom commits map
        let mut custom_commits_map: HashMap<String, PathBuf> = HashMap::new();
        custom_commits_map.insert("rom".to_string(), context.elf_bin_path.clone());

        // Define variables to store withness library and proof Id
        // We need this witness_lib variable to keep a "live" reference to the witness library
        let _witness_lib: Library;
        let mut witness_lib_constructor: Box<dyn WitnessLibrary<Goldilocks>>;
        let mut proof_id: Option<String> = None;

        match context.config.field {
            Field::Goldilocks => {
                (_witness_lib, witness_lib_constructor) = Self::load_witness_lib(&context).unwrap();

                // Generate the proof (or verify-constraints if in debug mode)
                if context.config.debug_info.std_mode.name == ModeName::Debug {
                    ProofMan::<Goldilocks>::verify_proof_constraints_from_lib(
                        &mut *witness_lib_constructor,
                        context.config.proving_key.clone().into(),
                        context.config.output_dir.clone().into(),
                        custom_commits_map,
                        ProofOptions::new(
                            false,
                            context.config.verbose.into(),
                            context.config.aggregation,
                            context.config.final_snark,
                            context.config.verify_proofs,
                            context.config.debug_info.clone(),
                        ),
                    )
                    .map_err(|e| ProverError::ProofGenerationError(e.to_string()))?;
                } else {
                    proof_id = ProofMan::<Goldilocks>::generate_proof_from_lib(
                        &mut *witness_lib_constructor,
                        context.config.proving_key.clone().into(),
                        context.config.output_dir.clone().into(),
                        custom_commits_map,
                        ProofOptions::new(
                            false,
                            context.config.verbose.into(),
                            context.config.aggregation,
                            context.config.final_snark,
                            context.config.verify_proofs,
                            context.config.debug_info.clone(),
                        ),
                    )
                    .map_err(|e| ProverError::ProofGenerationError(e.to_string()))?;
                }
            }
        }

        let elapsed = start.elapsed().as_secs_f64();

        // Get results from the witness library
        let result: ZiskExecutionResult = *witness_lib_constructor
            .get_execution_result()
            .ok_or_else(|| anyhow::anyhow!("No execution result found"))?
            .downcast::<ZiskExecutionResult>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast execution result"))?;

        // Print the results
        println!();
        info!("{}", "    Zisk: --- PROVE SUMMARY ------------------------".bright_green().bold());
        if let Some(proof_id) = &proof_id {
            info!("                Proof ID: {}", proof_id);
        }
        info!("              ► Statistics");
        info!("                time: {} seconds, steps: {}", elapsed, result.executed_steps);

        // Save the proof result
        if let Some(proof_id) = proof_id {
            let logs = proof_log::ProofLog::new(result.executed_steps, proof_id, elapsed);
            let log_path = context.config.output_dir.as_ref().join("result.json");
            proof_log::ProofLog::write_json_log(&log_path, &logs)
                .map_err(|e| ProverError::ProofLogError(e.to_string()))?;
        }

        Ok(())
    }
}

/// Errors that can occur during proving.
#[derive(Debug, thiserror::Error)]
pub enum ProverError {
    #[error("Missing required field: {0}")]
    RequiredFieldError(String),

    #[error("Failed to load ASM file: {0}")]
    AsmLoadError(String),

    #[error("Failed to load ASM ROM file: {0}")]
    AsmRomLoadError(String),

    #[error("Failed to load ELF file: {0}")]
    ElfLoadError(String),

    #[error("Failed to generate ELF hash: {0}")]
    ElfHashError(String),

    #[error("Failed to load witness library: {0}")]
    WitnessLoadError(String),

    #[error("Failed to run emulator: {0}")]
    EmulatorError(String),

    #[error("Failed to generate witness: {0}")]
    WitnessGenerationError(String),

    #[error("Failed to generate proof: {0}")]
    ProofGenerationError(String),

    #[error("Failed to store proof log: {0}")]
    ProofLogError(String),

    #[error("Proof verification failed")]
    VerificationFailed,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}
