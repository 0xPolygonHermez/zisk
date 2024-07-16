use libloading::{Library, Symbol};
use log::{debug, info, trace};
use p3_field::AbstractField;
use stark::{GlobalInfo, StarkProver};
use std::path::PathBuf;

use wchelpers::WCLibrary;

use common::{ExecutionCtx, ProofCtx};

pub struct ProofMan<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: AbstractField + 'static> ProofMan<F> {
    const MY_NAME: &'static str = "ProofMan";

    pub fn generate_proof(
        wc_lib_path: PathBuf,
        proving_key_path: PathBuf,
        public_inputs: Vec<u8>,
    ) -> Result<Vec<F>, Box<dyn std::error::Error>> {
        // Check wc_lib path exists
        if !wc_lib_path.exists() {
            return Err(format!("Witness computation dynamic library not found at path: {:?}", wc_lib_path).into());
        }

        // Check proving_key path exists
        if !proving_key_path.exists() {
            return Err(format!("Proving key not found at path: {:?}", proving_key_path).into());
        }
        // Check provingKey is a folder
        if !proving_key_path.is_dir() {
            return Err(format!("Proving key path is not a folder: {:?}", proving_key_path).into());
        }

        // Load the witness computation dynamic library
        let library = unsafe { Library::new(wc_lib_path.clone())? };
        let wc_lib: Symbol<fn() -> Box<dyn WCLibrary<F>>> = unsafe { library.get(b"init_library")? };
        let mut wc_lib = wc_lib();

        let pilout = wc_lib.get_pilout();
        // TODO! Check hash

        let mut pctx = ProofCtx::create_ctx(pilout, public_inputs);
        let mut ectx = ExecutionCtx::builder().is_discovery_execution().build();

        Self::init_proof(&mut wc_lib, &mut pctx, &mut ectx);

        // Initialize prover and buffers to fit the proof
        Self::initialize_provers(&proving_key_path, &mut pctx);

        ectx.discovering = false;

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

    fn init_proof(wc_lib: &mut Box<dyn WCLibrary<F>>, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        wc_lib.start_proof(pctx, ectx);

        wc_lib.calculate_plan(ectx);

        trace!("{}: Plan: ", Self::MY_NAME);
        for air_instance in ectx.instances.iter() {
            let air = pctx.pilout.get_air(air_instance.air_group_id, air_instance.air_id);
            let name = if air.name().is_some() { air.name().unwrap() } else { "Unnamed" };
            trace!("{}:     + Air[{}][{}] {}", Self::MY_NAME, air_instance.air_group_id, air_instance.air_id, name);
        }

        wc_lib.initialize_air_instances(pctx, &*ectx);
    }

    fn initialize_provers(proving_key_path: &PathBuf, pctx: &mut ProofCtx<F>) {
        info!("{}: Initializing prover and creating buffers", Self::MY_NAME);

        let global_info = GlobalInfo::from_file(&proving_key_path.join("pilout.globalInfo.json"));

        pctx.provers = Vec::new();

        for air_instance in pctx.air_instances.iter_mut() {
            debug!(
                "{}: Initializing prover for air instance ({}, {})",
                Self::MY_NAME,
                air_instance.air_group_id,
                air_instance.air_id
            );

            let prover = Box::new(StarkProver::new(
                &proving_key_path,
                &global_info,
                air_instance.air_group_id,
                air_instance.air_id,
            ));

            let buffer_size = prover.get_total_bytes();
            trace!("{}: ··· Preallocating a buffer of {} bytes", Self::MY_NAME, buffer_size);
            air_instance.buffer = vec![0u8; buffer_size];

            pctx.provers.push(prover);
        }
    }

    pub fn commit_stage(stage: u32, _pctx: &ProofCtx<F>) {
        info!("{}: Committing stage {}", Self::MY_NAME, stage);
    }

    fn calculate_challenges(stage: u32, _proof_ctx: &ProofCtx<F>) {
        // This is a mock implementation
        info!("{}: Calculating challenges for stage {}", Self::MY_NAME, stage);
    }

    pub fn opening_stages(_pctx: &ProofCtx<F>) {
        info!("{}: Opening stages", Self::MY_NAME);
    }

    fn finalize_proof(_proof_ctx: &ProofCtx<F>) -> Vec<F> {
        // This is a mock implementation
        vec![]
    }
}
