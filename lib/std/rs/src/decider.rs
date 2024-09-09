use proofman_common::{ProofCtx, SetupCtx};

pub trait Decider<F> {
    fn decide(&self, sctx: &SetupCtx, pctx: &ProofCtx<F>);
}
