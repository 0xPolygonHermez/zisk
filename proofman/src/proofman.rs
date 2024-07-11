use libloading::{Library, Symbol};
use log::info;
use p3_field::AbstractField;
use stark::StarkProver;
use std::path::PathBuf;

use wchelpers::WCLibrary;

use common::{ExecutionCtx, ProofCtx};

pub struct ProofMan<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: AbstractField + 'static> ProofMan<F> {
    const MY_NAME: &'static str = "ProofMan ";

    pub fn generate_proof(
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
        let mut wc_lib: Box<dyn WCLibrary> = init_library(wc_lib).expect("Failed to load plugin");

        let pilout = wc_lib.get_pilout();

        let mut pctx = ProofCtx::create_ctx(pilout, public_inputs);
        let mut ectx = ExecutionCtx::builder().with_air_instances_map().with_all_instances().build();

        wc_lib.start_proof(&mut pctx, &mut ectx);

        wc_lib.calculate_plan(&mut ectx);

        wc_lib.initialize_air_instances(&mut pctx, &ectx);

        // Initialize prover and buffers to fit the proof
        Self::initialize_prover(&proving_key, &mut pctx);

        for stage in 1..=pctx.pilout.num_stages() {
            wc_lib.calculate_witness(stage, &mut pctx, &ectx);

            Self::commit_stage(stage, &mut pctx);
            if stage <= pctx.pilout.num_stages() {
                Self::calculate_challenges(stage, &pctx);
            }
        }

        wc_lib.end_proof();

        Self::opening_stages(&pctx);

        let proof = Self::finalize_proof(&pctx);

        Ok(proof)
    }

    fn initialize_prover(proving_key: &PathBuf, pctx: &mut ProofCtx) {
        println!("{}: Initializing prover and creating buffers", Self::MY_NAME);

        for air_instance in pctx.air_instances.iter_mut() {
            println!("{}: Initializing prover for air instance {:?}", Self::MY_NAME, air_instance);
            let folder = match air_instance.air_group_id {
                0 => "build/FibonacciSquare/airs/FibonacciSquare_0/air",
                1 => "build/Module/airs/Module_0/air",
                2 => "build/U8Air/airs/U8Air_0/air",
                _ => panic!("{}: Invalid air group id", Self::MY_NAME),
            };
            let prover = Box::new(StarkProver::<F>::new2(proving_key.join(folder)));
            let buffer_size = prover.get_total_bytes();
            info!("{}: Preallocating a buffer of {} bytes", Self::MY_NAME, buffer_size);
            air_instance.buffer = vec![0u8; buffer_size];

            pctx.provers.push(prover);
        }
    }

    pub fn commit_stage(stage: u32, _pctx: &ProofCtx) {
        println!("{}: Committing stage {}", Self::MY_NAME, stage);
    }

    fn calculate_challenges(stage: u32, _proof_ctx: &ProofCtx) {
        // This is a mock implementation
        println!("{}: Calculating challenges for stage {}", Self::MY_NAME, stage);
    }

    pub fn opening_stages(_pctx: &ProofCtx) {
        println!("{}: Opening stages", Self::MY_NAME);
    }

    fn finalize_proof(_proof_ctx: &ProofCtx) -> Vec<F> {
        // This is a mock implementation
        vec![]
    }
}

fn init_library(path: PathBuf) -> Result<Box<dyn WCLibrary>, libloading::Error> {
    let library = unsafe { Library::new(path)? };

    let library: Symbol<fn() -> Box<dyn WCLibrary>> = unsafe { library.get(b"init_library")? };

    Ok(library())
}
