use pilout::pilout_proxy::PilOutProxy;
use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;

pub trait Decider {
    fn decide<F>(
        &self,
        stage: u32,
        air_instance: &AirInstanceCtx<F>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    );
}
