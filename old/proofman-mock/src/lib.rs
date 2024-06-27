mod wc_manager;

pub use wc_manager::WCManager;

// Type aliases for readability
pub type ProofContext<T> = Vec<T>;

pub struct SubproofPlan {
    pub instance_id: u32,
    pub subproof_id: u32,
    pub air_id: u32,
    pub meta: Box<dyn std::any::Any>,
}

pub trait AirPlanner {}

pub struct ProofmanPlanner {}

impl AirPlanner for ProofmanPlanner {}

pub trait AirWitnessComputation<T, I> {
    fn git_instance_map(&self, proof_id: &str, inputs: Option<I>) -> Vec<SubproofPlan>;

    fn start_proof(&self, proof_id: &str, instance: SubproofPlan);

    fn witness_calculate(
        &self,
        stage_id: u32,
        proof_ctx: ProofContext<T>,
        _instance: SubproofPlan,
        _buffer: Vec<u8>,
        inputs: Option<I>,
    );

    fn end_proof(&self, proof_id: &str);
}

pub trait ProofWitnessComputation<T, I> {
    fn start_proof(&self, proof_id: &str, instance: SubproofPlan);

    fn witness_calculate(&self, stage_id: u32, proof_ctx: ProofContext<T>, inputs: Option<I>);

    fn end_proof(&self, proof_id: &str);
}
