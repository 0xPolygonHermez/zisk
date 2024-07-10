use common::ProofCtx;

pub struct StarkProver;

impl StarkProver {
    pub const MY_NAME: &'static str = "StrkPrvr ";

    pub fn initialize_prover<F>(&self, _pctx: &ProofCtx<F>) {
        println!("{}: Initializing prover and creating buffers", Self::MY_NAME);
    }

    pub fn commit_stage<F>(&self, stage: u32, _pctx: &ProofCtx<F>) {
        println!("{}: Committing stage {}", Self::MY_NAME, stage);
    }

    pub fn opening_stages<F>(&self, _pctx: &ProofCtx<F>) {
        println!("{}: Opening stages", Self::MY_NAME);
    }
}
