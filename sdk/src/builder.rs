use std::path::{Path, PathBuf};

use crate::{
    get_asm_paths, get_proving_key, get_witness_computation_lib,
    prover::{Asm, AsmProver, Emu, EmuProver, ZiskProver},
};
use colored::Colorize;
use fields::{ExtensionField, GoldilocksQuinticExtension, PrimeField64};
use proofman_common::ParamsGPU;
use zisk_distributed_common::LoggingConfig;

use anyhow::Result;

// Typestate markers
pub struct EmuB;
pub struct AsmB;

pub struct WitnessGeneration;
pub struct Prove;

/// Unified builder for both EMU and ASM provers with typestate pattern
///
/// This builder uses typestate pattern to ensure type-safe configuration:
/// - Backend state: `EmulatorBackend` or `AsmBackend`  
/// - Operation state: `WitnessGeneration` or `Prove`
///
/// # Example
/// ```rust,no_run
/// use zisk_sdk::ProverClientBuilder;
///
/// let elf_path = std::path::PathBuf::from("path/to/program.elf");
/// let output_path = std::path::PathBuf::from("path/to/output");
///
/// let prover_emu = ProverClientBuilder::new()
///     .emu()
///     .verify_constraints()
///     .elf_path(elf_path.clone())
///     .build();
///
/// let prover_asm = ProverClientBuilder::new()
///     .asm()
///     .prove()
///     .elf_path(elf_path)
///     .save_proofs(true)
///     .output_dir(output_path)
///     .unlock_mapped_memory(true)
///     .build();
/// ```
#[derive(Default)]
pub struct ProverClientBuilder<Backend = (), Operation = ()> {
    // Common fields for both EMU and ASM
    aggregation: bool,
    rma: bool,
    compressed: bool,
    witness_lib: Option<PathBuf>,
    proving_key: Option<PathBuf>,
    proving_key_snark: Option<PathBuf>,
    elf: Option<PathBuf>,
    verify_constraints: bool,
    witness: bool,
    verbose: u8,
    shared_tables: bool,
    logging_config: Option<LoggingConfig>,
    print_command_info: bool,

    // ASM-specific fields (only available when Backend = AsmBackend)
    asm_path: Option<PathBuf>,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,

    // Prove-specific fields (only available when Operation = Prove)
    save_proofs: bool,
    output_dir: Option<PathBuf>,
    verify_proofs: bool,
    minimal_memory: bool,
    gpu_params: Option<ParamsGPU>,

    // Phantom data to track state
    _backend: std::marker::PhantomData<Backend>,
    _operation: std::marker::PhantomData<Operation>,
}

impl ProverClientBuilder<(), ()> {
    #[must_use]
    pub fn new() -> Self {
        Self { aggregation: true, rma: true, ..Default::default() }
    }

    /// Configure for Emulator backend
    #[must_use]
    pub fn emu(self) -> ProverClientBuilder<EmuB, ()> {
        self.into()
    }

    /// Configure for ASM backend
    #[must_use]
    pub fn asm(self) -> ProverClientBuilder<AsmB, ()> {
        self.into()
    }
}

// Common methods available for any backend
impl<Backend> ProverClientBuilder<Backend, ()> {
    /// Configure for constraint verification operation
    #[must_use]
    pub fn witness(self) -> ProverClientBuilder<Backend, WitnessGeneration> {
        let mut builder: ProverClientBuilder<Backend, WitnessGeneration> = self.into();
        builder.verify_constraints = false;
        builder.witness = true;
        builder.aggregation = false;
        builder
    }

    /// Configure for constraint verification operation
    #[must_use]
    pub fn verify_constraints(self) -> ProverClientBuilder<Backend, WitnessGeneration> {
        let mut builder: ProverClientBuilder<Backend, WitnessGeneration> = self.into();
        builder.verify_constraints = true;
        builder.aggregation = false;
        builder
    }

    /// Configure for proof generation operation
    #[must_use]
    pub fn prove(self) -> ProverClientBuilder<Backend, Prove> {
        self.into()
    }
}

// Common configuration methods for any backend and operation
impl<Backend, Operation> ProverClientBuilder<Backend, Operation> {
    /// Enables aggregation.
    #[must_use]
    pub fn aggregation(mut self, enable: bool) -> Self {
        self.aggregation = enable;
        self
    }

    /// Set RMA.
    #[must_use]
    pub fn rma(mut self, use_rma: bool) -> Self {
        self.rma = use_rma;
        self
    }

    /// Enables final vadcop is compressed proof.
    #[must_use]
    pub fn compressed(mut self, enable: bool) -> Self {
        self.compressed = enable;
        self
    }

    #[must_use]
    pub fn witness_lib_path(mut self, witness_lib: PathBuf) -> Self {
        self.witness_lib = Some(witness_lib);
        self
    }

    #[must_use]
    pub fn witness_lib_path_opt(mut self, witness_lib: Option<PathBuf>) -> Self {
        self.witness_lib = witness_lib;
        self
    }

    #[must_use]
    pub fn proving_key_path(mut self, proving_key: PathBuf) -> Self {
        self.proving_key = Some(proving_key);
        self
    }

    #[must_use]
    pub fn proving_key_path_opt(mut self, proving_key: Option<PathBuf>) -> Self {
        self.proving_key = proving_key;
        self
    }

    #[must_use]
    pub fn proving_key_snark_path(mut self, proving_key_snark: PathBuf) -> Self {
        self.proving_key_snark = Some(proving_key_snark);
        self
    }

    #[must_use]
    pub fn proving_key_snark_path_opt(mut self, proving_key_snark: Option<PathBuf>) -> Self {
        self.proving_key_snark = proving_key_snark;
        self
    }

    #[must_use]
    pub fn elf_path(mut self, elf_path: PathBuf) -> Self {
        self.elf = Some(elf_path);
        self
    }

    #[must_use]
    pub fn verbose(mut self, verbose: u8) -> Self {
        self.verbose = verbose;
        self
    }

    #[must_use]
    pub fn shared_tables(mut self, shared: bool) -> Self {
        self.shared_tables = shared;
        self
    }

    #[must_use]
    pub fn logging_config(mut self, logging_config: LoggingConfig) -> Self {
        self.logging_config = Some(logging_config);
        self
    }

    #[must_use]
    pub fn print_command_info(mut self) -> Self {
        self.print_command_info = true;
        self
    }
}

// ASM-specific methods
impl<Operation> ProverClientBuilder<AsmB, Operation> {
    #[must_use]
    pub fn asm_path(mut self, asm_path: PathBuf) -> Self {
        self.asm_path = Some(asm_path);
        self
    }

    #[must_use]
    pub fn asm_path_opt(mut self, asm_path: Option<PathBuf>) -> Self {
        self.asm_path = asm_path;
        self
    }

    #[must_use]
    pub fn base_port(mut self, base_port: u16) -> Self {
        self.base_port = Some(base_port);
        self
    }

    #[must_use]
    pub fn base_port_opt(mut self, base_port: Option<u16>) -> Self {
        self.base_port = base_port;
        self
    }

    #[must_use]
    pub fn unlock_mapped_memory(mut self, unlock: bool) -> Self {
        self.unlock_mapped_memory = unlock;
        self
    }
}

// Prove-specific methods (available for both backends when operation is Prove)
impl<Backend> ProverClientBuilder<Backend, Prove> {
    #[must_use]
    pub fn save_proofs(mut self, save: bool) -> Self {
        self.save_proofs = save;
        self
    }

    #[must_use]
    pub fn output_dir(mut self, output_dir: PathBuf) -> Self {
        self.output_dir = Some(output_dir);
        self
    }

    #[must_use]
    pub fn verify_proofs(mut self, verify: bool) -> Self {
        self.verify_proofs = verify;
        self
    }

    #[must_use]
    pub fn minimal_memory(mut self, minimal: bool) -> Self {
        self.minimal_memory = minimal;
        self
    }

    #[must_use]
    pub fn gpu(mut self, gpu_params: ParamsGPU) -> Self {
        self.gpu_params = Some(gpu_params);
        self
    }
}

// Build methods for Emulator
impl ProverClientBuilder<EmuB, WitnessGeneration> {
    /// Builds an [`EmuProver`] configured for constraint verification.
    ///
    /// # Example
    /// ```rust,no_run
    /// use zisk_sdk::ProverClientBuilder;
    ///
    /// let elf_path = std::path::PathBuf::from("path/to/program.elf");
    ///
    /// let prover = ProverClientBuilder::new()
    ///     .emu()
    ///     .verify_constraints()
    ///     .elf_path(elf_path)
    ///     .build();
    /// ```
    pub fn build(self) -> Result<ZiskProver<Emu>> {
        self.build_emu()
    }
}

impl ProverClientBuilder<EmuB, Prove> {
    /// Builds an [`EmuProver`] configured for proof generation.
    ///
    /// # Example
    /// ```rust,no_run
    /// use zisk_sdk::ProverClientBuilder;
    ///
    /// let elf_path = std::path::PathBuf::from("path/to/program.elf");
    ///     
    /// let prover = ProverClientBuilder::new()
    ///    .emu()
    ///    .prove()
    ///    .elf_path(elf_path)
    ///    .build();
    /// ```
    pub fn build(self) -> Result<ZiskProver<Emu>> {
        self.build_emu()
    }
}

impl<X> ProverClientBuilder<EmuB, X> {
    fn build_emu(self) -> Result<ZiskProver<Emu>> {
        let witness_lib = get_witness_computation_lib(self.witness_lib.as_ref());
        let proving_key = get_proving_key(self.proving_key.as_ref());
        let proving_key_snark = None;
        let elf = self.elf.ok_or_else(|| anyhow::anyhow!("ELF path is required"))?;

        let output_dir = if !self.verify_constraints {
            Some(self.output_dir.unwrap_or_else(|| "tmp".into()))
        } else {
            None
        };

        if self.print_command_info {
            Self::print_emu_command_info(
                self.witness,
                self.verify_constraints,
                &witness_lib,
                &proving_key,
                &proving_key_snark,
                &elf,
                output_dir.as_ref(),
            );
        }

        let emu = EmuProver::new(
            self.verify_constraints,
            self.aggregation,
            self.rma,
            self.compressed,
            witness_lib,
            proving_key,
            proving_key_snark,
            elf,
            self.verbose,
            self.shared_tables,
            self.gpu_params.filter(|_| !self.verify_constraints).unwrap_or_default(),
            self.verify_proofs,
            self.minimal_memory,
            self.save_proofs,
            output_dir.clone(),
            self.logging_config,
        )?;

        Ok(ZiskProver::<Emu>::new(emu))
    }

    fn print_emu_command_info(
        witness: bool,
        verify_constraints: bool,
        witness_lib: &Path,
        proving_key: &Path,
        proving_key_snark: &Option<PathBuf>,
        elf: &Path,
        output_dir: Option<&PathBuf>,
    ) {
        if witness {
            println!("{: >12} StatsConstraints", "Command".bright_green().bold());
        } else if verify_constraints {
            println!("{: >12} VerifyConstraints", "Command".bright_green().bold());
        } else {
            println!("{: >12} Prove", "Command".bright_green().bold());
        }

        println!("{: >12} {}", "Witness Lib".bright_green().bold(), witness_lib.display());
        println!("{: >12} {}", "ELF".bright_green().bold(), elf.display());
        println!(
            "{: >12} {}",
            "Emulator".bright_green().bold(),
            "Running in emulator mode".bright_yellow()
        );
        println!("{: >12} {}", "Proving Key".bright_green().bold(), proving_key.display());

        if let Some(proving_key_snark) = proving_key_snark {
            println!(
                "{: >12} {}",
                "Proving key SNARK".bright_green().bold(),
                proving_key_snark.display()
            );
        }

        if let Some(output_dir) = output_dir {
            println!("{: >12} {}", "Output Dir".bright_green().bold(), output_dir.display());
        }

        println!();
    }
}

// Build methods for ASM
impl ProverClientBuilder<AsmB, WitnessGeneration> {
    /// Builds an [`AsmProver`] configured for constraint verification.
    ///
    /// # Example
    /// ```rust,no_run
    /// use zisk_sdk::ProverClientBuilder;
    ///
    /// let elf_path = std::path::PathBuf::from("path/to/program.elf");
    ///
    /// let prover = ProverClientBuilder::new()
    ///     .asm()
    ///     .verify_constraints()
    ///     .elf_path(elf_path)
    ///     .build();
    /// ```
    pub fn build<F>(self) -> Result<ZiskProver<Asm>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        self.build_asm()
    }
}

impl ProverClientBuilder<AsmB, Prove> {
    /// Builds an [`AsmProver`] configured for proof generation.
    ///
    /// # Example
    /// ```rust,no_run
    /// use zisk_sdk::ProverClientBuilder;
    ///
    /// let elf_path = std::path::PathBuf::from("path/to/program.elf");
    ///
    /// let prover = ProverClientBuilder::new()
    ///     .asm()
    ///     .prove()
    ///     .elf_path(elf_path)
    ///     .build();
    /// ```
    pub fn build<F>(self) -> Result<ZiskProver<Asm>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        self.build_asm()
    }
}

impl<X> ProverClientBuilder<AsmB, X> {
    fn build_asm<F>(self) -> Result<ZiskProver<Asm>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        let witness_lib = get_witness_computation_lib(self.witness_lib.as_ref());
        let proving_key = get_proving_key(self.proving_key.as_ref());
        let proving_key_snark = None;
        let elf = self.elf.ok_or_else(|| anyhow::anyhow!("ELF path is required"))?;

        let output_dir = if !self.verify_constraints {
            Some(self.output_dir.unwrap_or_else(|| "tmp".into()))
        } else {
            None
        };

        let (asm_mt_filename, asm_rh_filename) = get_asm_paths(&elf)?;

        if self.print_command_info {
            Self::print_asm_command_info(
                self.witness,
                self.verify_constraints,
                &witness_lib,
                &proving_key,
                &proving_key_snark,
                &elf,
                output_dir.as_ref(),
            );
        }

        let asm = AsmProver::new(
            self.verify_constraints,
            self.aggregation,
            self.rma,
            self.compressed,
            witness_lib,
            proving_key,
            proving_key_snark,
            elf,
            self.verbose,
            self.shared_tables,
            asm_mt_filename,
            asm_rh_filename,
            self.base_port,
            self.unlock_mapped_memory,
            self.gpu_params.filter(|_| !self.verify_constraints).unwrap_or_default(),
            self.verify_proofs,
            self.minimal_memory,
            self.save_proofs,
            output_dir.clone(),
            self.logging_config,
        )?;

        Ok(ZiskProver::<Asm>::new(asm))
    }

    fn print_asm_command_info(
        witness: bool,
        verify_constraints: bool,
        witness_lib: &Path,
        proving_key: &Path,
        proving_key_snark: &Option<PathBuf>,
        elf: &Path,
        output_dir: Option<&PathBuf>,
    ) {
        if witness {
            println!("{: >12} StatsConstraints", "Command".bright_green().bold());
        } else if verify_constraints {
            println!("{: >12} VerifyConstraints", "Command".bright_green().bold());
        } else {
            println!("{: >12} Prove", "Command".bright_green().bold());
        }

        println!("{: >12} {}", "Witness Lib".bright_green().bold(), witness_lib.display());
        println!("{: >12} {}", "ELF".bright_green().bold(), elf.display());
        println!("{: >12} {}", "Proving Key".bright_green().bold(), proving_key.display());

        if let Some(proving_key_snark) = proving_key_snark {
            println!(
                "{: >12} {}",
                "Proving key SNARK".bright_green().bold(),
                proving_key_snark.display()
            );
        }

        if let Some(output_dir) = output_dir {
            println!("{: >12} {}", "Output Dir".bright_green().bold(), output_dir.display());
        }

        println!();
    }
}

// Safe state transitions using From traits
impl From<ProverClientBuilder<(), ()>> for ProverClientBuilder<EmuB, ()> {
    fn from(builder: ProverClientBuilder<(), ()>) -> Self {
        Self {
            // Preserve common fields
            aggregation: builder.aggregation,
            witness: builder.witness,
            rma: builder.rma,
            compressed: builder.compressed,
            witness_lib: builder.witness_lib,
            proving_key: builder.proving_key,
            proving_key_snark: builder.proving_key_snark,
            verify_constraints: builder.verify_constraints,
            elf: builder.elf,
            verbose: builder.verbose,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,
            logging_config: builder.logging_config,

            // Reset ASM-specific fields for EMU backend
            asm_path: None,
            base_port: None,
            unlock_mapped_memory: false,

            // Reset prove-specific fields (will be set when choosing operation)
            save_proofs: false,
            output_dir: None,
            verify_proofs: false,
            minimal_memory: false,
            gpu_params: None,

            _backend: std::marker::PhantomData,
            _operation: std::marker::PhantomData,
        }
    }
}

impl From<ProverClientBuilder<(), ()>> for ProverClientBuilder<AsmB, ()> {
    fn from(builder: ProverClientBuilder<(), ()>) -> Self {
        Self {
            // Preserve common fields
            aggregation: builder.aggregation,
            witness: builder.witness,
            rma: builder.rma,
            compressed: builder.compressed,
            witness_lib: builder.witness_lib,
            proving_key: builder.proving_key,
            proving_key_snark: builder.proving_key_snark,
            verify_constraints: builder.verify_constraints,
            elf: builder.elf,
            verbose: builder.verbose,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,
            logging_config: builder.logging_config,

            // Preserve ASM-specific fields (user may have set defaults)
            asm_path: builder.asm_path,
            base_port: builder.base_port,
            unlock_mapped_memory: builder.unlock_mapped_memory,

            // Reset prove-specific fields (will be set when choosing operation)
            save_proofs: false,
            output_dir: None,
            verify_proofs: false,
            minimal_memory: false,
            gpu_params: None,

            _backend: std::marker::PhantomData,
            _operation: std::marker::PhantomData,
        }
    }
}

impl<Backend> From<ProverClientBuilder<Backend, ()>>
    for ProverClientBuilder<Backend, WitnessGeneration>
{
    fn from(builder: ProverClientBuilder<Backend, ()>) -> Self {
        Self {
            // Preserve common fields
            aggregation: builder.aggregation,
            witness: builder.witness,
            rma: builder.rma,
            compressed: builder.compressed,
            witness_lib: builder.witness_lib,
            proving_key: builder.proving_key,
            proving_key_snark: builder.proving_key_snark,
            verify_constraints: builder.verify_constraints,
            elf: builder.elf,
            verbose: builder.verbose,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,
            logging_config: builder.logging_config,

            // Preserve backend-specific fields (ASM or EMU)
            asm_path: builder.asm_path,
            base_port: builder.base_port,
            unlock_mapped_memory: builder.unlock_mapped_memory,

            // Initialize prove-specific fields to defaults for verify_constraints mode
            save_proofs: false,    // Not relevant for constraint verification
            output_dir: None,      // Not needed for constraint verification
            verify_proofs: false,  // Not applicable for constraint verification
            minimal_memory: false, // Not relevant for constraint verification
            gpu_params: None,      // Not relevant for constraint verification

            _backend: std::marker::PhantomData,
            _operation: std::marker::PhantomData,
        }
    }
}

impl<Backend> From<ProverClientBuilder<Backend, ()>> for ProverClientBuilder<Backend, Prove> {
    fn from(builder: ProverClientBuilder<Backend, ()>) -> Self {
        Self {
            // Preserve common fields
            aggregation: builder.aggregation,
            witness: builder.witness,
            rma: builder.rma,
            compressed: builder.compressed,
            witness_lib: builder.witness_lib,
            proving_key: builder.proving_key,
            proving_key_snark: builder.proving_key_snark,
            verify_constraints: false,
            elf: builder.elf,
            verbose: builder.verbose,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,
            logging_config: builder.logging_config,

            // Preserve backend-specific fields (ASM or EMU)
            asm_path: builder.asm_path,
            base_port: builder.base_port,
            unlock_mapped_memory: builder.unlock_mapped_memory,

            // Initialize prove-specific fields to sensible defaults
            save_proofs: true,     // Default to saving proofs when proving
            output_dir: None,      // User should specify this
            verify_proofs: true,   // Default to verifying generated proofs
            minimal_memory: false, // Default to normal memory usage
            gpu_params: None,      // Default to CPU proving, user can set via .gpu()

            _backend: std::marker::PhantomData,
            _operation: std::marker::PhantomData,
        }
    }
}
