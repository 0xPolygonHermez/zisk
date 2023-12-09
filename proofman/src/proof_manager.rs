use crate::public_input::PublicInput;
use crate::{prover::Prover, executor::Executor};
use pilout::load_pilout;
use log::debug;

use math::FieldElement;
use crate::provers_manager::ProversManager;
use crate::witness_calculator_manager::WitnessCalculatorManager;

use crate::proof_ctx::ProofCtx;

#[derive(Debug)]
pub struct ProofManOpt {
    pub debug: bool,
}

impl Default for ProofManOpt {
    fn default() -> Self {
        Self {
            debug: false,
        }
    }
}

pub struct ProofManager<T> {
    options: ProofManOpt,
    proof_ctx: ProofCtx<T>,
    wc_manager: WitnessCalculatorManager<T>,
    provers_manager: ProversManager,
}

impl<T> ProofManager<T>
where T: FieldElement,
{
    const MY_NAME: &'static str = "proofman";

    pub fn new(pilout_path: &str, wc: Vec<Box<dyn Executor<T>>>, prover: Box<dyn Prover>, options: ProofManOpt) -> Self {
        env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .filter_level(log::LevelFilter::Debug)
        .init();

        let reset = "\x1b[37;0m";
        let bold = "\x1b[1m";
        let purple = "\x1b[35m";
        let green = "\x1b[32;1m";
        println!("{}{}Proof Manager {} by Polygon Labs{}", bold, purple, env!("CARGO_PKG_VERSION"), reset);
        println!("{}{}{} {}", green, format!("{: >13}", "Loaded:"), reset, std::env::current_exe().unwrap().display().to_string().as_str());
        println!("{}{}{} {}", green, format!("{: >13}", "Main PID:"), reset, std::process::id().to_string().as_str());
        println!("");
        println!("{}PROVE COMMAND{}", green, reset);
        // println!("{}{}{} {}", green, format!("{: >13}", "ProofMan:"), reset, "TODO");
        println!("{}{}{} {}", green, format!("{: >13}", "Pilout:"), reset, str::replace(pilout_path, "\\", "/"));
        // println!("{}{}{} {}", green, format!("{: >13}", "Executors:"), reset, "TODO");
        // println!("{}{}{} {}", green, format!("{: >13}", "Prover:"), reset, "TODO");
        println!("");
    
    
        debug!("{}> Initializing...", Self::MY_NAME);    

        let pilout = load_pilout(pilout_path);

        // TODO! Have we to take in account from here the FinitieField choosed in the pilout?

        let proof_ctx = ProofCtx::<T>::new(pilout);

        // Add WitnessCalculatorManager
        let wc_manager = WitnessCalculatorManager::new(wc);

        // Add ProverManager
        let provers_manager = ProversManager::new(prover);

        Self {
            options,
            proof_ctx,
            wc_manager,
            provers_manager,
        }
    }

    pub fn prove(&mut self, _public_inputs: Option<Box<dyn PublicInput>>) {
    }

    pub fn verify() {
        unimplemented!();
    }
}