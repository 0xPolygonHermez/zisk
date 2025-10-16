use std::path::{Path, PathBuf};

use crate::{
    get_asm_paths, get_proving_key, get_witness_computation_lib,
    prover::{Asm, AsmProver, Emu, EmuProver, ZiskProver},
};
use colored::Colorize;
use fields::{ExtensionField, GoldilocksQuinticExtension, PrimeField64};
use proofman_common::ParamsGPU;

use anyhow::Result;

// Typestate markers
pub struct EmuB;
pub struct AsmB;

pub struct VerifyConstraints;
pub struct Prove;

/// Unified builder for both EMU and ASM provers with typestate pattern
///
/// This builder uses typestate pattern to ensure type-safe configuration:
/// - Backend state: `EmulatorBackend` or `AsmBackend`  
/// - Operation state: `VerifyConstraints` or `Prove`
///
/// # Example
/// ```rust
/// use zisk_sdk::ProverClientBuilder;
///
/// // EMU builder for constraint verification
/// let prover = ProverClientBuilder::new()
///     .emu()
///     .verify_constraints()
///     .elf_path(Some(elf_path))
///     .build()?;
///
/// // ASM builder for proving
/// let prover = ProverClientBuilder::new()
///     .asm()
///     .prove()
///     .elf_path(Some(elf_path))
///     .save_proofs(true)
///     .output_dir(output_path)
///     .unlock_mapped_memory(true)
///     .build()?;
/// ```
#[derive(Default)]
pub struct ProverClientBuilder<Backend = (), Operation = ()> {
    // Common fields for both EMU and ASM
    aggregation: bool,
    final_snark: bool,
    witness_lib: Option<PathBuf>,
    proving_key: Option<PathBuf>,
    elf: Option<PathBuf>,
    verbose: u8,
    shared_tables: bool,
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
        Self::default()
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
    pub fn verify_constraints(self) -> ProverClientBuilder<Backend, VerifyConstraints> {
        self.into()
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

    /// Enables final SNARK generation.
    #[must_use]
    pub fn final_snark(mut self, enable: bool) -> Self {
        self.final_snark = enable;
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
impl ProverClientBuilder<EmuB, VerifyConstraints> {
    /// Builds an [`EmuProver`] configured for constraint verification.
    ///
    /// # Example
    /// ```rust
    /// use zisk_sdk::ProverClientBuilder;
    ///
    /// let prover = ProverClientBuilder::new()
    ///     .emu()
    ///     .verify_constraints()
    ///     .elf_path(elf_path)
    ///     .build()?;
    /// ```
    pub fn build(self) -> Result<ZiskProver<Emu>> {
        self.build_emu(true)
    }
}

impl ProverClientBuilder<EmuB, Prove> {
    /// Builds an [`EmuProver`] configured for proof generation.
    ///
    /// # Example
    /// ```rust
    /// use zisk_sdk::ProverClientBuilder;
    ///     
    /// let prover = ProverClientBuilder::new()
    ///    .emu()
    ///    .prove()
    ///    .elf_path(elf_path)
    ///    .build()?;
    /// ```
    pub fn build(self) -> Result<ZiskProver<Emu>> {
        self.build_emu(false)
    }
}

impl<X> ProverClientBuilder<EmuB, X> {
    fn build_emu(self, verify_constraints: bool) -> Result<ZiskProver<Emu>> {
        let witness_lib = get_witness_computation_lib(self.witness_lib.as_ref());
        let proving_key = get_proving_key(self.proving_key.as_ref());
        let elf = self.elf.ok_or_else(|| anyhow::anyhow!("ELF path is required"))?;

        let output_dir = if !verify_constraints {
            Some(self.output_dir.unwrap_or_else(|| "tmp".into()))
        } else {
            None
        };

        // TODO!: Validate paths exist?

        if self.print_command_info {
            Self::print_emu_command_info(
                verify_constraints,
                &witness_lib,
                &proving_key,
                &elf,
                output_dir.as_ref(),
            );
        }

        let emu = EmuProver::new(
            verify_constraints,
            self.aggregation,
            self.final_snark,
            witness_lib,
            proving_key,
            elf,
            self.verbose,
            self.shared_tables,
            self.gpu_params.filter(|_| !verify_constraints).unwrap_or_default(),
            self.verify_proofs,
            self.minimal_memory,
            self.save_proofs,
            output_dir.clone(),
        )?;

        Ok(ZiskProver::<Emu>::new(emu))
    }

    fn print_emu_command_info(
        verify_constraints: bool,
        witness_lib: &Path,
        proving_key: &Path,
        elf: &Path,
        output_dir: Option<&PathBuf>,
    ) {
        if verify_constraints {
            println!("{: >12} VerifyConstraints", "Command".bright_green().bold());
        } else {
            println!("{: >12} Prove", "Command".bright_green().bold());
        }

        println!("{: >12} {}", "Witness Lib".bright_green().bold(), witness_lib.display());
        println!("{: >12} {}", "Elf".bright_green().bold(), elf.display());
        println!(
            "{: >12} {}",
            "Emulator".bright_green().bold(),
            "Running in emulator mode".bright_yellow()
        );
        println!("{: >12} {}", "Proving key".bright_green().bold(), proving_key.display());

        if let Some(output_dir) = output_dir {
            println!("{: >12} {}", "Output Dir".bright_green().bold(), output_dir.display());
        }

        println!();
    }
}

// Build methods for ASM
impl ProverClientBuilder<AsmB, VerifyConstraints> {
    /// Builds an [`AsmProver`] configured for constraint verification.
    ///
    /// # Example
    /// ```rust
    /// use zisk_sdk::ProverClientBuilder;
    ///
    /// let prover = ProverClientBuilder::new()
    ///     .asm()
    ///     .verify_constraints()
    ///     .elf_path(elf_path)
    ///     .build()?;
    /// ```
    pub fn build<F>(self) -> Result<ZiskProver<Asm>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        self.build_asm(true)
    }
}

impl ProverClientBuilder<AsmB, Prove> {
    /// Builds an [`AsmProver`] configured for proof generation.
    ///
    /// # Example
    /// ```rust
    /// use zisk_sdk::ProverClientBuilder;
    ///
    /// let prover = ProverClientBuilder::new()
    ///     .asm()
    ///     .prove()
    ///     .elf_path(elf_path)
    ///     .build()?;
    /// ```
    pub fn build<F>(self) -> Result<ZiskProver<Asm>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        self.build_asm(false)
    }
}

impl<X> ProverClientBuilder<AsmB, X> {
    fn build_asm<F>(self, verify_constraints: bool) -> Result<ZiskProver<Asm>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        let witness_lib = get_witness_computation_lib(self.witness_lib.as_ref());
        let proving_key = get_proving_key(self.proving_key.as_ref());
        let elf = self.elf.ok_or_else(|| anyhow::anyhow!("ELF path is required"))?;

        let output_dir = if !verify_constraints {
            Some(self.output_dir.unwrap_or_else(|| "tmp".into()))
        } else {
            None
        };

        let (asm_mt_filename, asm_rh_filename) = get_asm_paths(&elf)?;

        if self.print_command_info {
            Self::print_asm_command_info(
                verify_constraints,
                &witness_lib,
                &proving_key,
                &elf,
                output_dir.as_ref(),
            );
        }

        let asm = AsmProver::new(
            verify_constraints,
            self.aggregation,
            self.final_snark,
            witness_lib,
            proving_key,
            elf,
            self.verbose,
            self.shared_tables,
            asm_mt_filename,
            asm_rh_filename,
            self.base_port,
            self.unlock_mapped_memory,
            self.gpu_params.filter(|_| !verify_constraints).unwrap_or_default(),
            self.verify_proofs,
            self.minimal_memory,
            self.save_proofs,
            output_dir.clone(),
        )?;

        Ok(ZiskProver::<Asm>::new(asm))
    }

    fn print_asm_command_info(
        verify_constraints: bool,
        witness_lib: &Path,
        proving_key: &Path,
        elf: &Path,
        output_dir: Option<&PathBuf>,
    ) {
        if verify_constraints {
            println!("{: >12} VerifyConstraints", "Command".bright_green().bold());
        } else {
            println!("{: >12} Prove", "Command".bright_green().bold());
        }

        println!("{: >12} {}", "Witness Lib".bright_green().bold(), witness_lib.display());
        println!("{: >12} {}", "Elf".bright_green().bold(), elf.display());
        println!("{: >12} {}", "Proving key".bright_green().bold(), proving_key.display());

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
            final_snark: builder.final_snark,
            witness_lib: builder.witness_lib,
            proving_key: builder.proving_key,
            elf: builder.elf,
            verbose: builder.verbose,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,

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
            final_snark: builder.final_snark,
            witness_lib: builder.witness_lib,
            proving_key: builder.proving_key,
            elf: builder.elf,
            verbose: builder.verbose,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,

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
    for ProverClientBuilder<Backend, VerifyConstraints>
{
    fn from(builder: ProverClientBuilder<Backend, ()>) -> Self {
        Self {
            // Preserve common fields
            aggregation: builder.aggregation,
            final_snark: builder.final_snark,
            witness_lib: builder.witness_lib,
            proving_key: builder.proving_key,
            elf: builder.elf,
            verbose: builder.verbose,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,

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
            final_snark: builder.final_snark,
            witness_lib: builder.witness_lib,
            proving_key: builder.proving_key,
            elf: builder.elf,
            verbose: builder.verbose,
            shared_tables: builder.shared_tables,
            print_command_info: builder.print_command_info,

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
