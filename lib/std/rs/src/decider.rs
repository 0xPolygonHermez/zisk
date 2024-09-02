use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;

pub trait Decider<F> {
    fn decide(
        &mut self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx,
    );
}
