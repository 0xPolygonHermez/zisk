use proofman_common::{ProofCtx, SetupCtx};

pub trait Decider<F> {
    fn decide(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx,
    );
}
