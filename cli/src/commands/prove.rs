use crate::ZISK_VERSION_MESSAGE;
use anyhow::Result;
use std::path::PathBuf;
use zisk::{common::Field, prove::ProveConfig, prover::Prover};
// Structure representing the 'prove' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskProve {
    /// Witness computation dynamic library path
    #[clap(short = 'w', long)]
    pub witness_lib: Option<PathBuf>,

    /// ELF file path
    /// This is the path to the ROM file that the witness computation dynamic library will use
    /// to generate the witness.
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    /// ASM file path
    /// Optional, mutually exclusive with `--emulator`
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

    /// Output dir path
    #[clap(short = 'o', long, default_value = "tmp")]
    pub output_dir: PathBuf,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

    #[clap(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    #[clap(short = 'f', long, default_value_t = false)]
    pub final_snark: bool,

    #[clap(short = 'y', long, default_value_t = false)]
    pub verify_proofs: bool,

    /// Verbosity (-v, -vv)
    #[arg(short ='v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,

    // PRECOMPILES OPTIONS
    /// Sha256f script path
    pub sha256f_script: Option<PathBuf>,
}

impl ZiskProve {
    pub fn run(&self) -> Result<()> {
        let prove_config = ProveConfig::new()
            .witness_lib(self.witness_lib.clone())
            .asm(self.asm.clone())
            .emulator(self.emulator)
            .proving_key(self.proving_key.clone())
            .output_dir(Some(self.output_dir.clone()))
            .field(self.field.clone())
            .aggregation(self.aggregation)
            .final_snark(self.final_snark)
            .verify_proofs(self.verify_proofs)
            .verbose(self.verbose)
            .debug(self.debug.clone())
            .sha256f_script(self.sha256f_script.clone());

        Prover::new().prove(self.elf.clone(), self.input.clone(), Some(prove_config))
    }
}
