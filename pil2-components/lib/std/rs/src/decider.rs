use std::sync::Arc;

use proofman_common::{ProofCtx, SetupCtx};

pub trait Decider<F> {
    fn decide(&self, sctx: Arc<SetupCtx<F>>, pctx: Arc<ProofCtx<F>>);
}
