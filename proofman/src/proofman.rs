use crate::{prover::Prover, executor::Executor};
use pilout::pilout::PilOut;
use pilout::load_pilout;

use crate::proof_ctx::{ProofCtx, AirInstance};

use std::path::Path;

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

pub struct ProofMan<'a> {
    pilout: PilOut,
    wc: Vec<&'a dyn Executor>,
    prover: &'a dyn Prover,
    proof_ctx: ProofCtx,
    options: &'a ProofManOpt,
}

impl<'a> ProofMan<'a> {
    pub fn new(pilout: &Path, wc: Vec<&'a dyn Executor>, prover: &'a dyn Prover, options: &'a ProofManOpt) -> Self {
        let pilout = load_pilout(pilout);

        let mut proof_ctx = ProofCtx::new();

        for (subproof_index, subproof) in pilout.subproofs.iter().enumerate() {            
            for (air_index, _air) in subproof.airs.iter().enumerate() {
                proof_ctx.add_air_instance(AirInstance::new(subproof_index, air_index));
            }
        }

        Self {
            pilout,
            wc,
            prover,
            proof_ctx,
            options
        }
    }

    pub fn prove(&mut self, /* public_inputs */) {
        // Setup logging
        env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .filter_level(log::LevelFilter::Debug)
        .init();

        print_proofman_prompt();
        print_proofman_2("Loaded", std::env::current_exe().unwrap().display().to_string().as_str());
        print_proofman_2("Main PID", std::process::id().to_string().as_str());
        println!("");
        print_proofman_2("PROVE COMMAND", "");
        print_proofman_2("Proofman", "TODO");
        print_proofman_2("Pilout", "TODO");
        print_proofman_2("Executors", "TODO");
        print_proofman_2("Prover", "TODO");
    }

    pub fn verify() {
        unimplemented!();
    }
}

fn print_proofman_prompt() {
    let reset = "\x1b[37;0m";
    let purple = "\x1b[35m";
    let bold = "\x1b[1m";
    println!("{}{}Proof Manager {} by Polygon Labs{}", bold, purple, env!("CARGO_PKG_VERSION"), reset);
}

fn print_proofman_2(field1: &str, field2: &str) {
    let reset = "\x1b[37;0m";
    let green = "\x1b[32;1m";
    let padded_field1 = format!("{: >12}", field1);
    println!("{}{}{} {}", green, padded_field1, reset, field2);
}

