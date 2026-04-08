use std::path::{Path, PathBuf};

use crate::{get_proving_key, get_proving_key_snark, Asm, AsmProver, Emu, EmuProver, ZiskProver};
use colored::Colorize;
use fields::{ExtensionField, GoldilocksQuinticExtension, PrimeField64};
use proofman_common::ProofmanOptions;
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
/// let output_path = std::path::PathBuf::from("path/to/output");
///
/// let prover_emu = ProverClientBuilder::new()
///     .emu()
///     .verify_constraints()
///     .build();
///
/// let prover_asm = ProverClientBuilder::new()
///     .asm()
///     .prove()
///     .unlock_mapped_memory(true)
///     .build();
/// ```
#[derive(Default)]
pub struct ProverClientBuilder<Backend = (), Operation = ()> {
    // Common fields for both EMU and ASM
    snark_wrapper: bool,
    proving_key: Option<PathBuf>,
    proving_key_snark: Option<PathBuf>,
    shared_tables: bool,
    logging_config: Option<LoggingConfig>,
    print_command_info: bool,

    // ASM-specific fields (only available when Backend = AsmBackend)
    asm_path: Option<PathBuf>,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,
    asm_out_file: bool,
    no_auto_setup: bool,
    is_distributed: bool,

    options: ProofmanOptions,

    // Phantom data to track state
    _backend: std::marker::PhantomData<Backend>,
    _operation: std::marker::PhantomData<Operation>,
}

impl ProverClientBuilder<(), ()> {
    #[must_use]
    pub fn new() -> Self {
        Self { snark_wrapper: false, ..Default::default() }
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

    pub fn build(self) -> Result<ZiskProver<Emu>> {
        let builder: ProverClientBuilder<EmuB, Prove> = self.emu().into();
        builder.build_emu()
    }
}

// Common methods available for any backend
impl<Backend> ProverClientBuilder<Backend, ()> {
    /// Configure for constraint verification operation
    #[must_use]
    pub fn witness(self) -> ProverClientBuilder<Backend, WitnessGeneration> {
        let mut builder: ProverClientBuilder<Backend, WitnessGeneration> = self.into();
        builder.options.verify_constraints();
        builder
    }

    /// Configure for constraint verification operation
    #[must_use]
    pub fn verify_constraints(self) -> ProverClientBuilder<Backend, WitnessGeneration> {
        let mut builder: ProverClientBuilder<Backend, WitnessGeneration> = self.into();
        builder.options.verify_constraints();
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
        if !enable {
            self.options.no_aggregation();
        }
        self
    }

    /// Configure whether to use SNARK wrapper
    #[must_use]
    pub fn with_snark(mut self, snark: bool) -> Self {
        self.snark_wrapper = snark;
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
    pub fn verbose(mut self, verbose: u8) -> Self {
        self.options.verbose_mode(verbose.into());
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
    pub fn no_auto_setup(mut self, no_auto_setup: bool) -> Self {
        self.no_auto_setup = no_auto_setup;
        self
    }

    #[must_use]
    pub fn is_distributed(mut self, is_distributed: bool) -> Self {
        self.is_distributed = is_distributed;
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

    #[must_use]
    pub fn asm_out_file(mut self, asm_out_file: bool) -> Self {
        self.asm_out_file = asm_out_file;
        self
    }
}

// Prove-specific methods (available for any operation state - will use defaults if not in Prove mode)
impl<Backend, Operation> ProverClientBuilder<Backend, Operation> {
    #[must_use]
    pub fn options(mut self, options: ProofmanOptions) -> Self {
        self.options = options;
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
    ///
    /// let prover = ProverClientBuilder::new()
    ///     .emu()
    ///     .verify_constraints()
    ///     .build();
    /// ```
    pub fn build(self) -> Result<ZiskProver<Emu>> {
        self.build_emu()
    }
}

impl ProverClientBuilder<EmuB, ()> {
    pub fn build(self) -> Result<ZiskProver<Emu>> {
        let builder: ProverClientBuilder<EmuB, Prove> = self.into();
        builder.build_emu()
    }
}

impl ProverClientBuilder<EmuB, Prove> {
    /// Builds an [`EmuProver`] configured for proof generation.
    ///
    /// # Example
    /// ```rust,no_run
    /// use zisk_sdk::ProverClientBuilder;
    ///
    /// let prover = ProverClientBuilder::new()
    ///    .emu()
    ///    .prove()
    ///    .build();
    /// ```
    pub fn build(self) -> Result<ZiskProver<Emu>> {
        self.build_emu()
    }
}

impl<X> ProverClientBuilder<EmuB, X> {
    fn build_emu(self) -> Result<ZiskProver<Emu>> {
        let proving_key = get_proving_key(self.proving_key.as_ref());
        let proving_key_snark = get_proving_key_snark(self.proving_key_snark.as_ref());

        if self.print_command_info {
            Self::print_emu_command_info(&proving_key, &proving_key_snark);
        }

        let emu = EmuProver::new(
            self.snark_wrapper,
            false,
            proving_key,
            proving_key_snark,
            self.shared_tables,
            self.options,
            self.logging_config,
        )?;

        Ok(ZiskProver::<Emu>::new(emu))
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
impl ProverClientBuilder<AsmB, WitnessGeneration> {
    /// Builds an [`AsmProver`] configured for constraint verification.
    ///
    /// # Example
    /// ```rust,no_run
    /// use zisk_sdk::ProverClientBuilder;
    ///
    /// let prover = ProverClientBuilder::new()
    ///     .asm()
    ///     .verify_constraints()
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

impl ProverClientBuilder<AsmB, ()> {
    pub fn build<F>(self) -> Result<ZiskProver<Asm>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        let builder: ProverClientBuilder<AsmB, Prove> = self.into();
        builder.build_asm()
    }
}

impl ProverClientBuilder<AsmB, Prove> {
    /// Builds an [`AsmProver`] configured for proof generation.
    ///
    /// # Example
    /// ```rust,no_run
    /// use zisk_sdk::ProverClientBuilder;
    ///
    /// let prover = ProverClientBuilder::new()
    ///     .asm()
    ///     .prove()
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
        let proving_key = get_proving_key(self.proving_key.as_ref());
        let proving_key_snark = get_proving_key_snark(self.proving_key_snark.as_ref());

        if self.print_command_info {
            Self::print_asm_command_info(&proving_key, &proving_key_snark);
        }

        let asm = AsmProver::new(
            self.snark_wrapper,
            false,
            proving_key,
            proving_key_snark,
            self.shared_tables,
            self.base_port,
            self.unlock_mapped_memory,
            self.asm_out_file,
            self.no_auto_setup,
            self.options,
            self.is_distributed,
            self.logging_config,
        )?;

        Ok(ZiskProver::<Asm>::new(asm))
    }

    fn print_asm_command_info(proving_key: &Path, proving_key_snark: &Path) {
        println!("{: >12} {}", "Proving Key".bright_green().bold(), proving_key.display());

        println!("{: >12} {}", "SNARK Key".bright_green().bold(), proving_key_snark.display());

        println!();
    }
}

// Safe state transitions using From traits
impl From<ProverClientBuilder<(), ()>> for ProverClientBuilder<EmuB, ()> {
    fn from(builder: ProverClientBuilder<(), ()>) -> Self {
        Self {
            // Preserve common fields
            snark_wrapper: builder.snark_wrapper,
            proving_key: builder.proving_key,
            proving_key_snark: builder.proving_key_snark,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,
            logging_config: builder.logging_config,
            options: builder.options,

            // Reset ASM-specific fields for EMU backend
            asm_path: None,
            base_port: None,
            unlock_mapped_memory: false,
            asm_out_file: false,

            no_auto_setup: false,
            is_distributed: false,

            _backend: std::marker::PhantomData,
            _operation: std::marker::PhantomData,
        }
    }
}

impl From<ProverClientBuilder<(), ()>> for ProverClientBuilder<AsmB, ()> {
    fn from(builder: ProverClientBuilder<(), ()>) -> Self {
        Self {
            // Preserve common fields
            snark_wrapper: builder.snark_wrapper,
            proving_key: builder.proving_key,
            proving_key_snark: builder.proving_key_snark,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,
            logging_config: builder.logging_config,
            options: builder.options,

            // Preserve ASM-specific fields (user may have set defaults)
            asm_path: builder.asm_path,
            base_port: builder.base_port,
            unlock_mapped_memory: builder.unlock_mapped_memory,
            asm_out_file: builder.asm_out_file,
            no_auto_setup: builder.no_auto_setup,
            is_distributed: builder.is_distributed,

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
            snark_wrapper: builder.snark_wrapper,
            proving_key: builder.proving_key,
            proving_key_snark: builder.proving_key_snark,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,
            logging_config: builder.logging_config,
            options: builder.options,

            // Preserve backend-specific fields (ASM or EMU)
            asm_path: builder.asm_path,
            base_port: builder.base_port,
            unlock_mapped_memory: builder.unlock_mapped_memory,
            asm_out_file: builder.asm_out_file,
            no_auto_setup: builder.no_auto_setup,
            is_distributed: builder.is_distributed,

            _backend: std::marker::PhantomData,
            _operation: std::marker::PhantomData,
        }
    }
}

impl<Backend> From<ProverClientBuilder<Backend, ()>> for ProverClientBuilder<Backend, Prove> {
    fn from(builder: ProverClientBuilder<Backend, ()>) -> Self {
        Self {
            // Preserve common fields
            snark_wrapper: builder.snark_wrapper,
            proving_key: builder.proving_key,
            proving_key_snark: builder.proving_key_snark,
            options: builder.options,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,
            logging_config: builder.logging_config,

            // Preserve backend-specific fields (ASM or EMU)
            asm_path: builder.asm_path,
            base_port: builder.base_port,
            unlock_mapped_memory: builder.unlock_mapped_memory,
            asm_out_file: builder.asm_out_file,
            no_auto_setup: builder.no_auto_setup,
            is_distributed: builder.is_distributed,

            _backend: std::marker::PhantomData,
            _operation: std::marker::PhantomData,
        }
    }
}
