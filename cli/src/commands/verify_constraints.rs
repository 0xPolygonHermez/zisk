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
    /// Witness computation dynamic library path
    #[clap(short = 'w', long)]
    pub witness_lib: Option<PathBuf>,

    /// ROM file path.
    /// This is the path to the ROM file that the witness computation dynamic library will use
    /// to generate the witness.
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    /// ASM file path (mutually exclusive with `--emulator`)
    #[clap(short = 's', long)]
    pub asm: Option<PathBuf>,

    /// Use prebuilt emulator (mutually exclusive with `--asm`)
    #[clap(short = 'l', long, action = clap::ArgAction::SetTrue)]
    pub emulator: bool,

    /// Input path
    #[clap(short = 'i', long)]
    pub input: Option<PathBuf>,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,

    // PRECOMPILES OPTIONS
    /// Sha256f script path
    pub sha256f_script: Option<PathBuf>,
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
