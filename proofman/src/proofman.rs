use libloading::{Library, Symbol};
use std::path::PathBuf;

use crate::StarkProver;
use wchelpers::WCLibrary;

use common::{ExecutionCtx, ProofCtx};

pub struct ProofMan;

impl ProofMan {
    const MY_NAME: &'static str = "ProofMan ";

    pub fn generate_proof<F>(
        library_path: PathBuf,
        proving_key: PathBuf,
        public_inputs: Vec<u8>,
    ) -> Result<Vec<F>, Box<dyn std::error::Error>> {
        // Load the wtiness computation dynamic library
        let mut wc_lib: Box<dyn WCLibrary<F>> = init_library(library_path).expect("Failed to load plugin");

        let stark_prover = StarkProver;

        let mut pctx = ProofCtx::<F>::create_ctx(proving_key, public_inputs);
        let mut ectx = ExecutionCtx::builder().with_air_instances_map().with_all_instances().build();

        wc_lib.start_proof(&mut pctx, &mut ectx);

        wc_lib.calculate_plan(&pctx);

        // Initialize prover and buffers to fit the proof
        stark_prover.initialize_prover(&mut pctx);

        for stage in 1..=pctx.pilout.num_stages() {
            wc_lib.calculate_witness(stage, &mut pctx, &ectx);

            stark_prover.commit_stage(stage, &mut pctx);
            if stage <= pctx.pilout.num_stages() {
                Self::calculate_challenges(stage, &pctx);
            }
        }

        wc_lib.end_proof();

        stark_prover.opening_stages(&pctx);

        let proof = Self::finalize_proof(&pctx);

        Ok(proof)
    }

    fn calculate_challenges<F>(stage: u32, _proof_ctx: &ProofCtx<F>) {
        // This is a mock implementation
        println!("{}: Calculating challenges for stage {}", Self::MY_NAME, stage);
    }

    fn finalize_proof<F>(_proof_ctx: &ProofCtx<F>) -> Vec<F> {
        // This is a mock implementation
        vec![]
    }
}

fn init_library<F>(path: PathBuf) -> Result<Box<dyn WCLibrary<F>>, libloading::Error> {
    let library = unsafe { Library::new(path)? };

    let library: Symbol<fn() -> Box<dyn WCLibrary<F>>> = unsafe { library.get(b"init_library")? };

    Ok(library())
}
