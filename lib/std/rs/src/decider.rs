use pilout::pilout_proxy::PilOutProxy;
use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};

pub trait Decider {
    fn decide<F>(
        &self,
        stage: u32,
        pilout: &PilOutProxy,
        air_instance: &AirInstanceCtx<F>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    );
}
