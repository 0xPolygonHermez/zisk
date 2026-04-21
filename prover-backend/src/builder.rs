use std::path::Path;

use crate::{
    get_proving_key, get_proving_key_snark, Asm, AsmProver, BackendProverOpts, Emu, EmuProver,
    ZiskProver,
};
use colored::Colorize;
use fields::{ExtensionField, GoldilocksQuinticExtension, PrimeField64};
use zisk_cluster_common::LoggingConfig;

use anyhow::Result;

// Typestate markers
pub struct EmuB;
pub struct AsmB;

/// Operation mode for the prover builder
#[derive(Debug, Clone, Copy, Default)]
enum OperationMode {
    #[default]
    Prove,
    Witness,
}

/// Unified builder for both EMU and ASM provers with typestate pattern
///
/// This builder uses typestate pattern to ensure type-safe configuration:
/// - Backend state: `EmulatorBackend` or `AsmBackend`
///
/// # Example
/// ```rust,no_run
/// use zisk_prover_backend::ProverClientBuilder;
///
/// let output_path = std::path::PathBuf::from("path/to/output");
///
/// let prover_emu = ProverClientBuilder::new()
///     .emu()
///     .verify_constraints()
///     .build();
///
/// let prover_asm = ProverClientBuilder::new()
///     .asm()
///     .build();
/// ```
#[derive(Default)]
pub struct ProverClientBuilder<Backend = ()> {
    // Configuration
    prover_options: BackendProverOpts,
    logging_config: Option<LoggingConfig>,
    operation_mode: OperationMode,

    // Phantom data to track state
    _backend: std::marker::PhantomData<Backend>,
}

impl ProverClientBuilder<()> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            prover_options: BackendProverOpts::default(),
            logging_config: None,
            operation_mode: OperationMode::default(),
            _backend: std::marker::PhantomData,
        }
    }

    /// Configure for Emulator backend
    #[must_use]
    pub fn emu(self) -> ProverClientBuilder<EmuB> {
        self.into()
    }

    /// Configure for ASM backend
    #[must_use]
    pub fn asm(self) -> ProverClientBuilder<AsmB> {
        self.into()
    }

    pub fn build(self) -> Result<ZiskProver<Emu>> {
        let builder: ProverClientBuilder<EmuB> = self.emu();
        builder.build_emu()
    }
}

// Common configuration methods for any backend
impl<Backend> ProverClientBuilder<Backend> {
    /// Configure for witness generation operation
    #[must_use]
    pub fn witness(mut self) -> Self {
        self.operation_mode = OperationMode::Witness;
        self
    }

    /// Configure for constraint verification operation
    #[must_use]
    pub fn verify_constraints(mut self) -> Self {
        self.operation_mode = OperationMode::Witness;
        self
    }

    /// Configure for proof generation operation (default)
    #[must_use]
    pub fn prove(mut self) -> Self {
        self.operation_mode = OperationMode::Prove;
        self
    }
    #[must_use]
    pub fn with_prover_options(mut self, opts: BackendProverOpts) -> Self {
        self.prover_options = opts;
        self
    }

    #[must_use]
    pub fn logging_config(mut self, logging_config: LoggingConfig) -> Self {
        self.logging_config = Some(logging_config);
        self
    }
}

// Build methods for Emulator
impl ProverClientBuilder<EmuB> {
    /// Builds an [`EmuProver`] configured for the selected operation mode.
    ///
    /// # Example
    /// ```rust,no_run
    /// use zisk_prover_backend::ProverClientBuilder;
    ///
    /// // Constraint verification
    /// let prover = ProverClientBuilder::new()
    ///     .emu()
    ///     .verify_constraints()
    ///     .build();
    ///
    /// // Proof generation (default)
    /// let prover = ProverClientBuilder::new()
    ///     .emu()
    ///     .build();
    /// ```
    pub fn build(self) -> Result<ZiskProver<Emu>> {
        self.build_emu()
    }
    fn build_emu(self) -> Result<ZiskProver<Emu>> {
        let proving_key = get_proving_key(self.prover_options.proving_key.as_ref());
        let proving_key_snark =
            get_proving_key_snark(self.prover_options.proving_key_snark.as_ref());

        Self::print_emu_command_info(&proving_key, &proving_key_snark);

        let mut options = self.prover_options.build_proofman_options();

        if matches!(self.operation_mode, OperationMode::Witness) {
            options.verify_constraints = true;
            options.aggregation = false;
        }

        let emu = EmuProver::new(
            self.prover_options.plonk,
            self.prover_options.preload_plonk,
            proving_key,
            proving_key_snark,
            true,
            options,
            self.logging_config,
        )?;

        Ok(ZiskProver::<Emu>::new(emu, self.prover_options))
    }

    fn print_emu_command_info(proving_key: &Path, proving_key_snark: &Path) {
        println!(
            "{: >12} {}",
            "Emulator".bright_green().bold(),
            "Running in emulator mode".bright_yellow()
        );
        println!("{: >12} {}", "Proving Key".bright_green().bold(), proving_key.display());

        println!("{: >12} {}", "SNARK Key".bright_green().bold(), proving_key_snark.display());

        println!();
    }
}

// Build methods for ASM
impl ProverClientBuilder<AsmB> {
    /// Builds an [`AsmProver`] configured for the selected operation mode.
    ///
    /// # Example
    /// ```rust,no_run
    /// use zisk_prover_backend::ProverClientBuilder;
    ///
    /// // Constraint verification
    /// let prover = ProverClientBuilder::new()
    ///     .asm()
    ///     .verify_constraints()
    ///     .build();
    ///
    /// // Proof generation (default)
    /// let prover = ProverClientBuilder::new()
    ///     .asm()
    ///     .build();
    /// ```
    pub fn build<F>(self) -> Result<ZiskProver<Asm>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        self.build_asm()
    }
    fn build_asm<F>(self) -> Result<ZiskProver<Asm>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        let proving_key = get_proving_key(self.prover_options.proving_key.as_ref());
        let proving_key_snark =
            get_proving_key_snark(self.prover_options.proving_key_snark.as_ref());

        Self::print_asm_command_info(&proving_key, &proving_key_snark);

        let mut options = self.prover_options.build_proofman_options();

        if matches!(self.operation_mode, OperationMode::Witness) {
            options.verify_constraints = true;
            options.aggregation = false;
        }

        let asm = AsmProver::new(
            self.prover_options.plonk,
            self.prover_options.preload_plonk,
            proving_key,
            proving_key_snark,
            true,
            self.prover_options.asm_options.unlock_mapped_memory,
            self.prover_options.asm_options.asm_out_file,
            self.prover_options.asm_options.no_auto_setup,
            options,
            self.prover_options.asm_options.is_distributed,
            self.prover_options.asm_options.stdio,
            self.logging_config,
        )?;

        Ok(ZiskProver::<Asm>::new(asm, self.prover_options))
    }

    fn print_asm_command_info(proving_key: &Path, proving_key_snark: &Path) {
        println!("{: >12} {}", "Proving Key".bright_green().bold(), proving_key.display());

        println!("{: >12} {}", "SNARK Key".bright_green().bold(), proving_key_snark.display());

        println!();
    }
}

// Safe state transitions using From trait
impl From<ProverClientBuilder<()>> for ProverClientBuilder<EmuB> {
    fn from(builder: ProverClientBuilder<()>) -> Self {
        Self {
            prover_options: builder.prover_options,
            logging_config: builder.logging_config,
            operation_mode: builder.operation_mode,
            _backend: std::marker::PhantomData,
        }
    }
}

impl From<ProverClientBuilder<()>> for ProverClientBuilder<AsmB> {
    fn from(builder: ProverClientBuilder<()>) -> Self {
        Self {
            prover_options: builder.prover_options,
            logging_config: builder.logging_config,
            operation_mode: builder.operation_mode,
            _backend: std::marker::PhantomData,
        }
    }
}
