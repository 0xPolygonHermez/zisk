use std::path::{Path, PathBuf};

use crate::{
    asm::prover::{Asm, AsmProver},
    get_asm_paths, get_proving_key, get_witness_computation_lib, ZiskProver,
};
use colored::Colorize;
use fields::{ExtensionField, GoldilocksQuinticExtension, PrimeField64};

use anyhow::Result;

#[derive(Default)]
pub struct AsmProverBuilder {
    verify_constraints: bool,
    aggregation: bool,
    final_snark: bool,
    witness_lib: Option<PathBuf>,
    proving_key: Option<PathBuf>,
    elf: Option<PathBuf>,
    verbose: u8,
    shared_tables: bool,

    asm_path: Option<PathBuf>,
    base_port: Option<u16>,
    unlock_mapped_memory: bool,

    print_command_info: bool,
}

impl AsmProverBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables constraint verification.
    #[must_use]
    pub fn verify_constraints(mut self) -> Self {
        self.verify_constraints = true;
        self
    }

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
    pub fn witness_lib_path(mut self, witness_lib: Option<PathBuf>) -> Self {
        self.witness_lib = witness_lib;
        self
    }

    #[must_use]
    pub fn proving_key_path(mut self, proving_key: Option<PathBuf>) -> Self {
        self.proving_key = proving_key;
        self
    }

    #[must_use]
    pub fn elf_path(mut self, elf_path: Option<PathBuf>) -> Self {
        self.elf = elf_path;
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
    pub fn asm_path(mut self, asm_path: Option<PathBuf>) -> Self {
        self.asm_path = asm_path;
        self
    }

    #[must_use]
    pub fn base_port(mut self, base_port: Option<u16>) -> Self {
        self.base_port = base_port;
        self
    }

    #[must_use]
    pub fn unlock_mapped_memory(mut self, unlock: bool) -> Self {
        self.unlock_mapped_memory = unlock;
        self
    }

    #[must_use]
    pub fn print_command_info(mut self) -> Self {
        self.print_command_info = true;
        self
    }

    /// Builds a [`AsmProver`].
    ///
    /// # Example
    /// ```rust
    /// use zisk_sdk::ProverClient;
    ///
    /// let prover = ProverClient::builder().asm().build();
    /// ```
    pub fn build<F>(self) -> Result<ZiskProver<Asm>>
    where
        F: PrimeField64,
        GoldilocksQuinticExtension: ExtensionField<F>,
    {
        let witness_lib = get_witness_computation_lib(self.witness_lib.as_ref());
        let proving_key = get_proving_key(self.proving_key.as_ref());
        let elf = self.elf.ok_or_else(|| anyhow::anyhow!("elf_path is required"))?;

        let (asm_mt_filename, asm_rh_filename) = get_asm_paths(&elf)?;

        // TODO! Check that paths exist

        if self.print_command_info {
            Self::_print_command_info(&witness_lib, &proving_key, &elf);
        }

        let asm = AsmProver::new(
            self.verify_constraints,
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
        )?;

        Ok(ZiskProver::<Asm>::new(asm))
    }

    fn _print_command_info(witness_lib: &Path, proving_key: &Path, elf: &Path) {
        // Print Verify Constraints command info
        println!("{} VerifyConstraints", format!("{: >12}", "Command").bright_green().bold());
        println!("{: >12} {}", "Witness Lib".bright_green().bold(), witness_lib.display());
        println!("{: >12} {}", "Elf".bright_green().bold(), elf.display());
        println!("{: >12} {}", "Proving key".bright_green().bold(), proving_key.display());

        println!();
    }
}
