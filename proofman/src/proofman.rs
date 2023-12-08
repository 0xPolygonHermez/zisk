use crate::prover::Prover;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "proofman", about = "Proofman CLI")]
pub enum ProofManSettings {
    /// Prove
    #[structopt(name = "prove")]
    Prove {
        /// De/Activate debug mode
        #[structopt(short, long)]
        debug: bool,

        // TODO: Public inputs as Option

        /// Airout file
        #[structopt(short, long, parse(from_os_str))]
        airout: PathBuf, 
        
        /// Prover settings file
        #[structopt(short, long, parse(from_os_str))]
        prover_settings: PathBuf,

        /// Output file
        #[structopt(short, long, parse(from_os_str))]
        output: PathBuf,
    },
    Verify {

    }
}

pub struct ProofMan {
    //settings: ProofManSettings,
}

impl ProofMan {
    pub fn new(prover: &dyn Prover /*, wc: Vec<WitnessCalculators>, */) -> Self {
        Self {
        }
    }
}
