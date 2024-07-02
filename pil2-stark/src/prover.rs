use std::error::Error;
use std::path::PathBuf;

use log::trace;
use pilout::pilout_proxy::PilOutProxy;
use crate::{AirInstanceMap, ExecutionCtx, ProofCtx, WitnessManager};

use crate::{load_plugin, Proof};

pub struct Pil2StarkProver<F> {
    _phantom: std::marker::PhantomData<F>,
}

#[allow(unused_variables)]
impl<F: Default + Clone> Pil2StarkProver<F> {
    const MY_NAME: &'static str = "StrkPrvr";

    pub fn prove(lib_path: PathBuf, inputs: Option<PathBuf>) -> Result<Proof<F>, Box<dyn Error>> {
        let mut loaded_inputs: Option<Vec<F>> = None;

        // Check input parameters
        if let Some(path) = inputs {
            if path.exists() {
                if path.is_file() {
                    loaded_inputs = Some(vec![F::default(); 32]);
                } else {
                    return Err(format!("Path exists but is not a file: {:?}", path).into());
                }
            } else {
                return Err(format!("Path does not exist: {:?}", path).into());
            }
        }

        trace!("{}: ··· Loading plugin: {:?}", Self::MY_NAME, lib_path);
        let mut wc_plugin: Box<dyn WitnessManager<F>> = load_plugin(lib_path).expect("Failed to load plugin");
        wc_plugin.initialize();

        let mut proof_ctx = Self::create_proof_context(loaded_inputs, wc_plugin.get_pilout());
        trace!("{}: ··· Creating execution context", Self::MY_NAME);
        let execution_ctx = ExecutionCtx::builder().with_air_instances_map().with_all_instances().build();

        wc_plugin.start_proof(&proof_ctx, &execution_ctx);

        let air_instances_map = wc_plugin.get_air_instances_map(&proof_ctx);

        for stage in 1..=wc_plugin.get_pilout().num_stages() {
            if stage == 1 {
                Self::create_buffers(stage, &air_instances_map)
            }

            wc_plugin.calculate_witness(stage, wc_plugin.get_pilout(), &proof_ctx);

            Self::commit_stage(stage, &mut proof_ctx);

            if stage < wc_plugin.get_pilout().num_stages() {
                Self::update_challenges(stage, &mut proof_ctx);
            }
        }
        wc_plugin.end_proof(&proof_ctx);

        Self::opening_stages(&proof_ctx);

        let proof = Self::finalize_proof(&proof_ctx);

        Ok(proof)
    }

    fn create_proof_context(inputs: Option<Vec<F>>, pilout: &PilOutProxy) -> ProofCtx<F> {
        ProofCtx::new(inputs, pilout)
    }

    fn create_buffers(stage: u32, air_instances_map: &AirInstanceMap) {
        trace!("Creating buffers for stage {}", stage);
    }

    fn update_challenges(stage: u32, proof_ctx: &mut ProofCtx<F>) {
        trace!("Updating challenges for stage {}", stage);
    }

    fn commit_stage(stage: u32, proof_ctx: &mut ProofCtx<F>) {
        trace!("Committing stage {}", stage);
    }

    fn opening_stages(proof_ctx: &ProofCtx<F>) {
        trace!("Opening stages");
    }

    fn finalize_proof(proof_ctx: &ProofCtx<F>) -> Proof<F> {
        trace!("Finalizing proof");
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

        let _proof = Pil2StarkProver::prove::<u8>(PathBuf::from(path), None);
    }
}
