use std::path::PathBuf;

use pilout::pilout_proxy::PilOutProxy;
use proofman_common::{AirInstanceMap, ExecutionCtx, ProofCtx, WitnessManager};

use crate::{load_plugin, Proof};

pub struct Pil2StarkProver;

#[allow(unused_variables)]
impl Pil2StarkProver {
    pub fn prove<F: Default + Clone>(lib_path: PathBuf, inputs: Vec<u8>) -> Proof<F> {
        let mut wc_plugin: Box<dyn WitnessManager<F>> = load_plugin(lib_path).expect("Failed to load plugin");

        let mut proof_ctx = Self::create_proof_context(&inputs, wc_plugin.get_pilout());
        let execution_ctx = ExecutionCtx::builder().with_air_instances_map().with_all_instances().build();

        wc_plugin.start_proof(&proof_ctx, &execution_ctx);

        let air_instances_map = wc_plugin.get_air_instances_map(&proof_ctx);

        for stage in 1..=wc_plugin.get_pilout().num_stages() {
            if stage == 1 {
                Self::create_buffers::<F>(stage, &air_instances_map)
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

        proof
    }

    fn create_proof_context<F: Default + Clone>(inputs: &Vec<u8>, pilout: &PilOutProxy) -> ProofCtx<F> {
        ProofCtx::new(pilout)
    }

    fn create_buffers<F>(stage: u32, air_instances_map: &AirInstanceMap) {
        unimplemented!()
    }

    fn update_challenges<F>(stage: u32, proof_ctx: &mut ProofCtx<F>) {
        unimplemented!()
    }

    fn commit_stage<F>(stage: u32, proof_ctx: &mut ProofCtx<F>) {
        unimplemented!()
    }

    fn opening_stages<F>(proof_ctx: &ProofCtx<F>) {
        unimplemented!()
    }

    fn finalize_proof<F>(proof_ctx: &ProofCtx<F>) -> Proof<F> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // TODO! The library must be generated during the test
        let path = "/Users/xpinsach/dev/pil2-proofman/target/debug/libzisk_wc.dylib";

        let _proof = Pil2StarkProver::prove::<u8>(PathBuf::from(path), vec![0u8]);
    }
}
