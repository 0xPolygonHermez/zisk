pub mod provers_manager;

pub trait Prover {
    fn compute_stage(&self, stage_id: u32);
}
