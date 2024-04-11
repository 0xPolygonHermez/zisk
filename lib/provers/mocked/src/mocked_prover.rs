#![allow(non_snake_case)]

use log::info;
use proofman::ProofCtx;
use proofman::proof_manager::ProverStatus;
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
        info!("{}: <-> Mocked prover - BUILD", Self::MY_NAME);
    }

    fn commit_stage(&mut self, stage_id: u32, _proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        info!("{}: <-> Mocked prover - STAGE {}", Self::MY_NAME, stage_id);

        ProverStatus::StagesPending
    }

    fn opening_stage(&mut self, opening_id: u32, _proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        info!("{}: <-> Mocked prover - OPENING {}", Self::MY_NAME, opening_id);

        ProverStatus::StagesCompleted
    }
}
