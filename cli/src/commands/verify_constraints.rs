use crate::ZISK_VERSION_MESSAGE;

use anyhow::Result;
use std::path::PathBuf;
use zisk::{common::Field, prove::ProveConfig, prover::Prover};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskVerifyConstraints {
    /// Path to the ELF file for constraints verification.
    #[clap(short = 'e', long, required = true)]
    pub elf: PathBuf,

    /// Path to the input file for constraints verification.
    #[clap(short = 'i', long)]
    pub input: Option<PathBuf>,

    /// Path to the assembly file for the emulator [default: installation path].
    /// Cannot be used together with `--emulator`.
    #[clap(short = 's', long)]
    pub asm: Option<PathBuf>,

    /// Use the prebuilt emulator instead of assembly.
    /// Cannot be used together with `--asm`.
    #[clap(short = 'l', long, action = clap::ArgAction::SetTrue)]
    pub emulator: bool,

    /// Path to the witness computation library [default: installation path].
    #[clap(short = 'w', long)]
    pub witness_lib: Option<PathBuf>,

    /// Path to the proving key setup directory [default: installation path].
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Path to the SHA256f script file [default: installation path].
    #[clap(short = 's', long)]
    pub sha256f_script: Option<PathBuf>,

    /// Finite field to use for constraints verification.
    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

    /// Increase verbosity [possible values: -v, -vv, etc...].
    #[clap(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Enable debug mode.
    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,
}

impl ZiskVerifyConstraints {
    pub fn run(&mut self) -> Result<()> {
        // Configure prove command (only_verify_constraints)
        let prove_config = ProveConfig::new()
            .only_verify_constraints(true)
            .witness_lib(self.witness_lib.clone())
            .asm(self.asm.clone())
            .emulator(self.emulator)
            .proving_key(self.proving_key.clone())
            .field(self.field.clone())
            .verbose(self.verbose)
            .debug(self.debug.clone())
            .sha256f_script(self.sha256f_script.clone());

        // Verify constraints
        let result = Prover::new().prove(
            self.elf.clone(),
            self.input.clone(),
            Some(prove_config.clone()),
        )?;

        // Print the result
        result.print();

        Ok(())
    }
}
