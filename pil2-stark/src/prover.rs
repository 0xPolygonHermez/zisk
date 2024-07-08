use std::error::Error;
use std::path::PathBuf;

use log::{info, trace};
use pilout::pilout_proxy::PilOutProxy;
use common::{AirGroupInstanceMap, ExecutionCtx, ProofCtx};
use wcmanager::{WitnessManagerAPI, WitnessModule};

use crate::Proof;
use wcmanager::load_plugin;

pub struct Pil2StarkProver<'a, F> {
    _phantom: std::marker::PhantomData<&'a F>,
}

#[allow(unused_variables)]
impl<'a, F: Default + Clone> Pil2StarkProver<'a, F> {
    const MY_NAME: &'static str = "StrkPrvr";

    pub fn prove(
        lib_path: PathBuf,
        pilout_path: PathBuf,
        inputs_path: Option<PathBuf>,
    ) -> Result<Proof<F>, Box<dyn Error>> {
        // Check input parameters
        if !lib_path.exists() {
            return Err(format!("Path does not exist: {:?}", lib_path).into());
        }

        if !pilout_path.exists() {
            return Err(format!("Path does not exist: {:?}", pilout_path).into());
        }
        if !pilout_path.is_file() {
            return Err(format!("Path is not a file: {:?}", pilout_path).into());
        }

        if let Some(ref inputs_path) = inputs_path {
            if !inputs_path.exists() {
                return Err(format!("Path does not exist: {:?}", inputs_path).into());
            }
            if !inputs_path.is_file() {
                return Err(format!("Path is not a file: {:?}", inputs_path).into());
            }
        }

        trace!("{}: ··· Loading plugin: {:?}", Self::MY_NAME, lib_path);
        let wc_plugin: Box<dyn WitnessManagerAPI<'a, F>> = load_plugin(lib_path).expect("Failed to load plugin");

        let wcm: Box<dyn WitnessModule<'a, F> + 'a> = wc_plugin.build_wcmanager();

        // Proving key
        let pilout = PilOutProxy::new(&pilout_path.display().to_string(), false).unwrap();

        // Check hashes from proving key and plugin match
        if wc_plugin.get_pilout_hash() != b"fibonacci-vadcop-hash" {
            return Err("Hashes do not match".into());
        }

        info!("{}: ··· Creating proof context", Self::MY_NAME);
        let mut proof_ctx = Self::create_proof_context(&pilout);

        info!("{}: ··· Creating execution context", Self::MY_NAME);
        let execution_ctx = ExecutionCtx::builder().with_air_instances_map().with_all_instances().build();

        let public_inputs: Vec<u8> = vec![17u8, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];

        info!("{}: ··· Starting proof", Self::MY_NAME);
        wcm._start_proof(&public_inputs, &mut proof_ctx, &execution_ctx);

        info!("{}: ··· Calculating Air instances map", Self::MY_NAME);
        wcm._calculate_air_instances_map(&proof_ctx);

        for stage in 1..=pilout.num_stages() {
            if stage == 1 {
                Self::create_buffers(stage, &proof_ctx.air_instances_map)
            }

            info!("{}: ··· Calculating Witness for stage {}", Self::MY_NAME, stage);
            wcm._calculate_witness(stage, &public_inputs, &proof_ctx, &execution_ctx);

            Self::commit_stage(stage, &mut proof_ctx);

            if stage < pilout.num_stages() {
                Self::update_challenges(stage, &mut proof_ctx);
            }
        }

        info!("{}: ··· Ending proof", Self::MY_NAME);
        // end_calculate_witness
        wcm._end_proof(&proof_ctx);

        Self::opening_stages(&proof_ctx);

        let proof = Self::finalize_proof(&proof_ctx);

        Ok(proof)
    }

    fn create_proof_context(pilout: &PilOutProxy) -> ProofCtx<F> {
        ProofCtx::new(pilout)
    }

    fn create_buffers(stage: u32, air_instances_map: &AirGroupInstanceMap) {
        info!("{}: ··· Creating buffers for stage", Self::MY_NAME);
    }

    fn update_challenges(stage: u32, proof_ctx: &mut ProofCtx<F>) {
        info!("{}: ··· Updating challenges for stage {}", Self::MY_NAME, stage);
    }

    fn commit_stage(stage: u32, proof_ctx: &mut ProofCtx<F>) {
        info!("{}: ··· Committing stage", Self::MY_NAME);
    }

    fn opening_stages(proof_ctx: &ProofCtx<F>) {
        info!("{}: ··· Opening stages", Self::MY_NAME);
    }

    fn finalize_proof(proof_ctx: &ProofCtx<F>) -> Proof<F> {
        info!("{}: ··· Finalizing proof", Self::MY_NAME);
        Proof::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // TODO The library must be generated during the test
        let path = "/Users/xpinsach/dev/pil2-proofman/target/debug/libzisk_wc.dylib";

        // let _proof = Pil2StarkProver::prove::<u8>(PathBuf::from(path), None);
    }
}
