//! Library API for ZiskProve
//!
//! This module provides a clean API for setting up and running a proving session.

use std::path::PathBuf;

use crate::commands::Field;

/// Builder for `ProveConfig`.
#[derive(Debug, Default)]
pub struct ProveConfigBuilder {
    witness_lib: Option<PathBuf>,
    elf: Option<PathBuf>,
    asm: Option<PathBuf>,
    emulator: bool,
    input: Option<PathBuf>,
    proving_key: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    field: Field,
    aggregation: bool,
    final_snark: bool,
    verify_proofs: bool,
    verbose: u8,
    debug: Option<Option<String>>,
    keccak_script: Option<PathBuf>,
}

impl ProveConfigBuilder {
    /// Creates a new builder instance with defaults.
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn witness_lib(mut self, path: impl Into<PathBuf>) -> Self {
        self.witness_lib = Some(path.into());
        self
    }

    pub fn elf(mut self, path: impl Into<PathBuf>) -> Self {
        self.elf = Some(path.into());
        self
    }

    pub fn asm(mut self, path: impl Into<PathBuf>) -> Self {
        self.asm = Some(path.into());
        self
    }

    pub fn emulator(mut self, enabled: bool) -> Self {
        self.emulator = enabled;
        self
    }

    pub fn input(mut self, path: impl Into<PathBuf>) -> Self {
        self.input = Some(path.into());
        self
    }

    pub fn proving_key(mut self, path: impl Into<PathBuf>) -> Self {
        self.proving_key = Some(path.into());
        self
    }

    pub fn output_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.output_dir = Some(path.into());
        self
    }

    pub fn field(mut self, field: Field) -> Self {
        self.field = field;
        self
    }

    pub fn aggregation(mut self, enabled: bool) -> Self {
        self.aggregation = enabled;
        self
    }

    pub fn final_snark(mut self, enabled: bool) -> Self {
        self.final_snark = enabled;
        self
    }

    pub fn verify_proofs(mut self, enabled: bool) -> Self {
        self.verify_proofs = enabled;
        self
    }

    pub fn verbose(mut self, level: u8) -> Self {
        self.verbose = level;
        self
    }

    pub fn debug(mut self, debug: Option<String>) -> Self {
        self.debug = Some(debug);
        self
    }

    pub fn keccak_script(mut self, path: impl Into<PathBuf>) -> Self {
        self.keccak_script = Some(path.into());
        self
    }

    /// Finalizes the builder and produces a `ProveConfig`.
    ///
    /// # Errors
    /// Returns an error if required fields are missing.
    pub fn build(self) -> Result<ProveConfig, ProverError> {
        Ok(ProveConfig {
            witness_lib: self.witness_lib,
            elf: self.elf.ok_or_else(|| ProverError::Other("Missing required field: elf".into()))?,
            asm: self.asm,
            emulator: self.emulator,
            input: self.input,
            proving_key: self.proving_key,
            output_dir: self.output_dir.ok_or_else(|| ProverError::Other("Missing required field: output_dir".into()))?,
            field: self.field,
            aggregation: self.aggregation,
            final_snark: self.final_snark,
            verify_proofs: self.verify_proofs,
            verbose: self.verbose,
            debug: self.debug,
            keccak_script: self.keccak_script,
        })
    }
}


/// Configuration for running a proof generation session.
#[derive(Debug, Clone)]
pub struct ProveConfig {
    /// Witness computation dynamic library path
    pub witness_lib: Option<PathBuf>,

    /// ELF file path
    pub elf: PathBuf,

    /// ASM file path (optional, mutually exclusive with emulator)
    pub asm: Option<PathBuf>,

    /// Use prebuilt emulator instead of ASM
    pub emulator: bool,

    /// Input path (optional)
    pub input: Option<PathBuf>,

    /// Proving key path (optional)
    pub proving_key: Option<PathBuf>,

    /// Output directory
    pub output_dir: PathBuf,

    /// Field type to use
    pub field: Field,

    /// Enable aggregation
    pub aggregation: bool,

    /// Enable final SNARK generation
    pub final_snark: bool,

    /// Enable proof verification
    pub verify_proofs: bool,

    /// Verbosity level (0 = silent, 1 = verbose, 2 = very verbose, etc.)
    pub verbose: u8,

    /// Enable debug mode with optional filter
    pub debug: Option<Option<String>>,

    /// Keccak script path (optional)
    pub keccak_script: Option<PathBuf>,
}

/// Result produced after running the prover.
#[derive(Debug)]
pub struct ProveOutput {
    /// Path where proofs have been written
    pub proofs_dir: PathBuf,

    /// Optionally, verification result
    pub verified: Option<bool>,
}

/// Main prover interface.
pub struct Prover {
    config: ProveConfig,
}

impl Prover {
    /// Creates a new Prover instance.
    pub fn new(config: ProveConfig) -> Self {
        Self { config }
    }

    /// Runs the proof generation process.
    ///
    /// # Errors
    /// Returns an error if any step of the proving process fails.
    pub fn run(&self) -> Result<ProveOutput, ProverError> {
        // Example pseudo-code:
        // 1. Load witness generation library if needed
        // 2. Prepare input data (input file or default inputs)
        // 3. Setup proving key
        // 4. Run emulator or ASM witness generation
        // 5. Generate proofs
        // 6. Optionally verify proofs
        // 7. Write outputs
        
        todo!("Implement prover logic based on configuration");
    }
}

/// Errors that can occur during proving.
#[derive(Debug, thiserror::Error)]
pub enum ProverError {
    #[error("Failed to load witness library: {0}")]
    WitnessLoadError(String),

    #[error("Failed to run emulator: {0}")]
    EmulatorError(String),

    #[error("Failed to generate witness: {0}")]
    WitnessGenerationError(String),

    #[error("Failed to generate proof: {0}")]
    ProofGenerationError(String),

    #[error("Proof verification failed")]
    VerificationFailed,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}
