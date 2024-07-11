use libloading::{Library, Symbol};
use std::path::PathBuf;

use crate::StarkProver;
use wchelpers::WCLibrary;

use common::{ExecutionCtx, ProofCtx};

pub struct ProofMan;

impl ProofMan {
    const MY_NAME: &'static str = "ProofMan ";

    pub fn generate_proof<F>(
        wc_lib: PathBuf,
        proving_key: PathBuf,
        public_inputs: Vec<u8>,
    ) -> Result<Vec<F>, Box<dyn std::error::Error>> {
        // Check wc_lib path exists
        if !wc_lib.exists() {
            return Err(format!("Witness computation dynamic library not found at path: {:?}", wc_lib).into());
        }

        // Check proving_key path exists
        if !proving_key.exists() {
            return Err(format!("Proving key not found at path: {:?}", proving_key).into());
        }
        // Check provingKey is a folder
        if !proving_key.is_dir() {
            return Err(format!("Proving key path is not a folder: {:?}", proving_key).into());
        }

        // Load the wtiness computation dynamic library
        let mut wc_lib: Box<dyn WCLibrary<F>> = init_library(wc_lib).expect("Failed to load plugin");

        let stark_prover = StarkProver;

        let pilout = wc_lib.get_pilout();

        let mut pctx = ProofCtx::<F>::create_ctx(pilout, public_inputs);
        let mut ectx = ExecutionCtx::builder().with_air_instances_map().with_all_instances().build();

        wc_lib.start_proof(&mut pctx, &mut ectx); //Exeecució ràpida -> Instàncies + Inputs per les màquines de L2

        wc_lib.calculate_plan(&pctx); // Calcular les instàncies que necessites

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
