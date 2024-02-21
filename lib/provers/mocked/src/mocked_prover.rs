#![allow(non_snake_case)]

use log::info;
use proofman::proof_ctx::ProofCtx;
use proofman::provers_manager::Prover;

pub struct MockedProver<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T> MockedProver<T> {
    const MY_NAME: &'static str = "mockdPrv";

    pub fn new() -> Self {
        Self { phantom: std::marker::PhantomData }
    }
}
impl<T> Prover<T> for MockedProver<T> {
    fn build(&mut self) {
        info!("{}: --> Mocked prover - BUILD", Self::MY_NAME);
    }

    fn compute_stage(&mut self, stage_id: u32, _proof_ctx: &mut ProofCtx<T>) {
        info!("{}: --> Mocked prover - STAGE {}", Self::MY_NAME, stage_id);
        info!("{}: <-- Mocked prover - STAGE {}", Self::MY_NAME, stage_id);
    }
}
