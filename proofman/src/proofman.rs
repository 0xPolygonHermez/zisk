use crate::{prover::Prover, executor::Executor};
use pilout::pilout::PilOut;
use pilout::load_pilout;

use crate::proof_ctx::{ProofCtx, AirInstance};

use std::path::PathBuf;
use structopt::StructOpt;
use std::path::Path;

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
    _pilout: PilOut,
    //settings: ProofManSettings,
}

impl ProofMan {
    pub fn new(pilout: &Path, _wc: Vec<&dyn Executor>, _prover: &dyn Prover, /* options */) -> Self {
        let pilout = load_pilout(pilout);

        let mut proof_ctx = ProofCtx::new();

        for (subproof_index, subproof) in pilout.subproofs.iter().enumerate() {            
            for (air_index, _air) in subproof.airs.iter().enumerate() {
                proof_ctx.add_air_instance(AirInstance::new(subproof_index, air_index));
            }
        }

        Self {
            _pilout: pilout,
        }
    }

    pub fn prove(&mut self, /* public_inputs */) {
        println!("Proving...");
    }

    pub fn verify() {
        unimplemented!();
    }
}
