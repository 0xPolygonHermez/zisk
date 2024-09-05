use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

pub trait Decider<F> {
    fn decide(
        &self,
        sctx: &SetupCtx,
        pctx: &ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) -> Result<u64, Box<dyn std::error::Error>>;
}
