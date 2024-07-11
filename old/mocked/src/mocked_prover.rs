#![allow(non_snake_case)]

use log::info;
use proofman::{AirInstanceCtx, ProofCtx};
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
    fn build(&mut self, _air_instance_ctx: &AirInstanceCtx<T>) {
        info!("{}: <-> Mocked prover - BUILD", Self::MY_NAME);
    }

    fn num_stages(&self) -> u32 {
        info!("{}: <-> Mocked prover - NUM STAGES", Self::MY_NAME);

        1
    }

    fn commit_stage(&mut self, stage_id: u32, _proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        info!("{}: <-> Mocked prover - STAGE {}", Self::MY_NAME, stage_id);

        ProverStatus::OpeningStage
    }

    fn opening_stage(&mut self, opening_id: u32, _proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        info!("{}: <-> Mocked prover - OPENING {}", Self::MY_NAME, opening_id);

        ProverStatus::StagesCompleted
    }

    fn get_commit_stage_root_challenge_256(&self, _stage_id: u32) -> Option<[u64; 4]> {
        Some([1u64; 4])
    }

    fn get_opening_stage_root_challenge_256(&self, _opening_id: u32) -> Option<[u64; 4]> {
        Some([1u64; 4])
    }

    fn add_root_challenge_256_to_transcript(&mut self, _root_challenge: [u64; 4]) {}

    fn get_subproof_values(&self) -> Vec<T> {
        vec![]
    }
}
