//! This module provides the ZisK zkVM Prover interface.

use crate::common::{get_home_dir, Field, OutputPath, ZiskLibInitFn};
use crate::prove::{ProveConfig, ProveContext, ProveResult};
use crate::{VerifyConfig, VerifyContext, VerifyResult};

use anyhow::Result;
use executor::ZiskExecutionResult;
use libloading::{Library, Symbol};
use p3_field::PrimeCharacteristicRing;
use p3_goldilocks::Goldilocks;
use proofman::verify_proof_from_file;
use proofman::ProofMan;
use proofman_common::{initialize_logger, ModeName, ProofOptions};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    path::PathBuf,
};
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

    fn get_asm_files(context: &mut ProveContext) -> Result<()> {
        // If it's macOS we need to use the emulator since ASM is not supported
        let emulator = if cfg!(target_os = "macos") { true } else { context.config.emulator };

        let mut asm_mt_path: Option<PathBuf> = context.config.asm.clone();
        let mut asm_rom_path: Option<PathBuf> = None;

        // If emulator is not enabled and no ASM file is provided
        if !emulator && asm_mt_path.is_none() {
            let stem = context.elf.file_stem().unwrap().to_str().unwrap();

            let hash = get_elf_data_hash(&context.elf)
                .map_err(|e| anyhow::anyhow!("Failed to generate ELF hash: {}", e))?;

            let asm_mt_filename = format!("{stem}-{hash}.bin");
            let asm_rom_filename = format!("{stem}-{hash}-rom.bin");
            asm_mt_path = Some(Self::default_cache_path().join(&asm_mt_filename));
            asm_rom_path = Some(Self::default_cache_path().join(&asm_rom_filename));
        }

        // If the asm_mt_path is set, check if the file exists
        if let Some(path) = &asm_mt_path {
            if !path.exists() {
                return Err(anyhow::anyhow!("Failed to load ASM file: {}", path.to_string_lossy()));
            }
        }

        // TODO: If the config.asm is set using cli parameter, the asm_rom_path will be always None
        // If the asm_rom_path is set, check if the file exists
        if let Some(path) = &asm_rom_path {
            if !path.exists() {
                return Err(anyhow::anyhow!("Failed to load ASM file: {}", path.to_string_lossy()));
            }
        }

        context.asm_mt_path = asm_mt_path;
        context.asm_rom_path = asm_rom_path;

        Ok(())
    }

    fn get_elf_bin_path(context: &mut ProveContext) -> Result<()> {
        let blowup_factor = get_rom_blowup_factor(context.config.proving_key.as_ref());

        let elf_bin_path =
            get_elf_bin_file_path(&context.elf, &Self::default_cache_path(), blowup_factor)
                .map_err(|e| anyhow::anyhow!("Failed to generate ELF hash: {}", e))?;

        if !elf_bin_path.exists() {
            let _ = gen_elf_hash(&context.elf, &elf_bin_path, blowup_factor, false)
                .map_err(|e| anyhow::anyhow!("Failed to generate ELF hash: {}", e))?;
        }

        context.elf_bin_path = elf_bin_path;

        Ok(())
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
    ) -> Result<(Library, Box<dyn WitnessLibrary<Goldilocks>>)> {
        match context.config.field {
            Field::Goldilocks => {
                let witness_lib_pathbuf: PathBuf = context.config.witness_lib.clone().into();
                let library = unsafe {
                    Library::new(witness_lib_pathbuf)
                        .map_err(|e| anyhow::anyhow!("Failed to load witness library: {}", e))?
                };
                let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> = unsafe {
                    library
                        .get(b"init_library")
                        .map_err(|e| anyhow::anyhow!("Failed to load witness library: {}", e))?
                };

                let witness_lib = witness_lib_constructor(
                    context.config.verbose.into(),
                    context.elf.clone(),
                    context.asm_mt_path.clone(),
                    context.asm_rom_path.clone(),
                    context.input.clone(),
                    context.config.sha256f_script.clone().into(),
                )
                .map_err(|e| anyhow::anyhow!("Failed to initialize witness library: {}", e))?;

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
    ) -> Result<ProveResult> {
        // Define the context for the proving process
        let mut context = ProveContext {
            elf,
            input,
            config: config.unwrap_or_else(ProveConfig::new),
            ..Default::default()
        };

        // Initialize the logger
        initialize_logger(context.config.verbose.into());

        // Get the paths for the ASM and ELF files
        Self::get_asm_files(&mut context)?;
        Self::get_elf_bin_path(&mut context)?;

        // TODO: Move this to the beginning of the function?
        let start = std::time::Instant::now();

        // Clean up the output directory only if we generate a proof
        if !context.config.only_verify_constraints {
            Self::cleanup_output_dir(&context.config.output_dir);
        }

        // Create the cache directory (if needed)
        Self::create_cache_dir();

        // Print the command context information
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

                // Generate the proof (or verify-constraints if in debug mode or only_verify_constraints)
                // TODO: Check if debug mode is still used
                if context.config.debug_info.std_mode.name == ModeName::Debug
                    || context.config.only_verify_constraints
                {
                    // Verify constraints only
                    ProofMan::<Goldilocks>::verify_proof_constraints_from_lib(
                        &mut *witness_lib_constructor,
                        context.config.proving_key.clone().into(),
                        context.config.output_dir.clone().into(),
                        custom_commits_map,
                        ProofOptions::new(
                            true,
                            context.config.verbose.into(),
                            false,
                            false,
                            false,
                            context.config.debug_info.clone(),
                        ),
                    )
                    .map_err(|e| anyhow::anyhow!("Failed to generate proof: {}", e))?;
                } else {
                    // Generate the proof
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
                    .map_err(|e| anyhow::anyhow!("Failed to generate proof: {}", e))?;
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

        Ok(ProveResult::new(proof_id, result.executed_steps, elapsed))
    }

    pub fn verify(
        &self,
        proof: PathBuf,
        public_inputs: Option<PathBuf>,
        config: Option<VerifyConfig>,
    ) -> Result<VerifyResult> {
        // Define the context for the verification process
        let context = VerifyContext {
            proof,
            public_inputs,
            config: config.unwrap_or_else(VerifyConfig::new),
        };

        // Initialize the logger
        initialize_logger(context.config.verbose.into());

        // Print the command context information
        context.print();

        // Get public inputs from the file
        let publics = if let Some(publics) = &context.public_inputs {
            let mut contents = String::new();
            let mut file = File::open(publics)?;

            let _ = file
                .read_to_string(&mut contents)
                .map_err(|e| anyhow::anyhow!("Failed to read public inputs file: {}", e));

            let verkey_json_string: Vec<String> = serde_json::from_str(&contents)?;

            let verkey_json: Vec<u64> = verkey_json_string
                .iter()
                .map(|s| s.parse::<u64>().expect("Failed to parse string as u64"))
                .collect();

            Some(verkey_json.into_iter().map(Goldilocks::from_u64).collect::<Vec<Goldilocks>>())
        } else {
            None
        };

        // Verify the proof
        // TODO: Modify the verify_proof_from_file function to pass params as PathBuf instead of String
        let valid = verify_proof_from_file::<Goldilocks>(
            context.proof.to_string_lossy().to_string(),
            context.config.stark_info.to_string_lossy().to_string(),
            context.config.verifier_bin.to_string_lossy().to_string(),
            context.config.verification_key.to_string_lossy().to_string(),
            publics,
            None,
            None,
        );

        Ok(VerifyResult { valid })
    }
}
