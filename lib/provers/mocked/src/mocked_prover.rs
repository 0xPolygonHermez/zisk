#![allow(non_snake_case)]

use log::info;
use proofman::proof_ctx::ProofCtx;
use proofman::provers_manager::Prover;

pub struct MockedProver<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T> MockedProver<T> {
    const MY_NAME: &'static str = "mck prvr";

    pub fn new() -> Self {
        Self { phantom: std::marker::PhantomData }
    }
}
impl<T> Prover<T> for MockedProver<T> {
    fn compute_stage(&self, stage_id: u32, _proof_ctx: &mut ProofCtx<T>) {
        info!("{}: --> Mocked prover - STAGE {}", Self::MY_NAME, stage_id);
        info!("{}: <-- Mocked prover - STAGE {}", Self::MY_NAME, stage_id);
    }
}
