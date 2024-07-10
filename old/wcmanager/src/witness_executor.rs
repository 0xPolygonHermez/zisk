extern crate common;
use common::{ExecutionCtx, ProofCtx};

pub trait WitnessExecutor<'a, F> {
    fn start_execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx);

    fn execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx);

    fn end_execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx);
}
