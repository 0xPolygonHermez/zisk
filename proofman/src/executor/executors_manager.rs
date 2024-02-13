use crate::executor::Executor;
use crate::proof_ctx::ProofCtx;
use crate::proof_manager_config::ProofManConfig;
use log::debug;
use crate::proof_manager_config::{ExecutorsConfiguration, ProverConfiguration, MetaConfiguration};

// WITNESS CALCULATOR MANAGER TRAIT
// ================================================================================================
pub trait ExecutorsManager<T, E: ExecutorsConfiguration, P: ProverConfiguration, M: MetaConfiguration> {
    const MY_NAME: &'static str;

    fn new(wc: Vec<Box<dyn Executor<T, E, P, M>>>) -> Self;
    fn witness_computation(&self, config: &ProofManConfig<E, P, M>, stage_id: u32, proof_ctx: &mut ProofCtx<T>);
}

// WITNESS CALCULATOR MANAGER (SEQUENTIAL)
// ================================================================================================
pub struct ExecutorsManagerSequential<T, E: ExecutorsConfiguration, P: ProverConfiguration, M: MetaConfiguration> {
    wc: Vec<Box<dyn Executor<T, E, P, M>>>,
    phantom: std::marker::PhantomData<(E, P, M)>,
}

impl<T, E: ExecutorsConfiguration, P: ProverConfiguration, M: MetaConfiguration> ExecutorsManager<T, E, P, M>
    for ExecutorsManagerSequential<T, E, P, M>
{
    const MY_NAME: &'static str = "exectrsm";

    fn new(wc: Vec<Box<dyn Executor<T, E, P, M>>>) -> Self {
        debug!("{}: Initializing...", Self::MY_NAME);

        ExecutorsManagerSequential::<T, E, P, M> { wc, phantom: std::marker::PhantomData }
    }

    fn witness_computation(&self, config: &ProofManConfig<E, P, M>, stage_id: u32, proof_ctx: &mut ProofCtx<T>) {
        debug!("{}: --> Computing witness stage {}", Self::MY_NAME, stage_id);

        for wc in self.wc.iter() {
            wc.witness_computation(config, stage_id, proof_ctx);
        }

        debug!("{}: <-- Computing witness stage {}", Self::MY_NAME, stage_id);
    }
}
