use proofman_common::ProofCtx;
use proofman_setup::SetupCtx;

pub trait Decider<F> {
    fn decide(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx,
    );
}
