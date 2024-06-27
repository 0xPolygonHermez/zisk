use pilout::pilout_proxy::PilOutProxy;

use crate::{AirInstanceMap, ExecutionCtx, ProofCtx};

pub trait WitnessManager<F> {
    fn get_pilout(&self) -> &PilOutProxy;

    fn start_proof(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx);

    fn end_proof(&mut self, proof_ctx: &ProofCtx<F>);

    fn get_air_instances_map(&self, proof_ctx: &ProofCtx<F>) -> AirInstanceMap;

    fn calculate_witness(&self, stage: u32, pilout: &PilOutProxy, proof_ctx: &ProofCtx<F>);
}
