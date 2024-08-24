use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;

pub trait Decider<F> {
    fn decide(
        &self,
        stage: u32,
        air_instance: &AirInstanceCtx<F>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    );
}
