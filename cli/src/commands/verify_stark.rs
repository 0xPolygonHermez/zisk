use crate::ZISK_VERSION_MESSAGE;

use anyhow::{Ok, Result};
use std::path::PathBuf;
use zisk::{verify::VerifyConfig, Prover};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
pub struct ZiskVerify {
    /// Path to the proof file to verify.
    #[clap(short = 'p', long, required = true)]
    pub proof: PathBuf,

    /// Path to the public inputs file.
    #[clap(short = 'u', long)]
    pub public_inputs: Option<PathBuf>,

    /// Path to the STARK info file [default: installation path].
    #[clap(short = 's', long)]
    pub stark_info: Option<PathBuf>,

    /// Path to the verifier binary [default: installation path].
    #[clap(short = 'e', long)]
    pub verifier_bin: Option<PathBuf>,

    /// Path to the verification key file [default: installation path].
    #[clap(short = 'k', long)]
    pub verkey: Option<PathBuf>,

    /// Increase verbosity [possible values: -v, -vv, etc...].
    #[clap(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskVerify {
    pub fn run(&self) -> Result<()> {
        // Configure verify command
        let verify_config = VerifyConfig::new()
            .stark_info(self.stark_info.clone())
            .verifier_bin(self.verifier_bin.clone())
            .verification_key(self.verkey.clone());

        // Verify the proof
        let result = Prover::new().verify(
            self.proof.clone(),
            self.public_inputs.clone(),
            Some(verify_config.clone()),
        )?;

        // Print the result
        result.print();

        Ok(())
    }
}
