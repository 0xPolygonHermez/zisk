use crate::executor::Executor;
use crate::proof_ctx::ProofCtx;
use log::debug;

use super::BufferManager;

// WITNESS CALCULATOR MANAGER TRAIT
// ================================================================================================
pub trait ExecutorsManager<'a, T: 'a> {
    const MY_NAME: &'static str;

    fn new<I>(wc: I) -> Self
    where
        I: IntoIterator<Item = &'a dyn Executor<T>>;
    fn witness_computation(
        &self,
        stage_id: u32,
        proof_ctx: &mut ProofCtx<T>,
        buffers_manager: Option<&Box<dyn BufferManager<T>>>,
    );
}

// WITNESS CALCULATOR MANAGER (SEQUENTIAL)
// ================================================================================================
pub struct ExecutorsManagerSequential<'a, T> {
    wc: Vec<&'a dyn Executor<T>>,
    phantom_data: std::marker::PhantomData<T>,
}

impl<'a, T> ExecutorsManager<'a, T> for ExecutorsManagerSequential<'a, T> {
    const MY_NAME: &'static str = "exctrMan";

    fn new<I>(wc: I) -> Self
    where
        I: IntoIterator<Item = &'a dyn Executor<T>>,
    {
        // fn new(wc: &'a [&dyn Executor<T>]) -> Self {
        debug!("{}: Initializing", Self::MY_NAME);

        let wc: Vec<&dyn Executor<T>> = wc.into_iter().collect();

        ExecutorsManagerSequential { wc, phantom_data: std::marker::PhantomData }
    }

    fn witness_computation(
        &self,
        stage_id: u32,
        proof_ctx: &mut ProofCtx<T>,
        buffers_manager: Option<&Box<dyn BufferManager<T>>>,
    ) {
        debug!("{}: --> Computing witness stage {}", Self::MY_NAME, stage_id);

        for wc in self.wc.iter() {
            wc.witness_computation(stage_id, proof_ctx, buffers_manager);
        }

        debug!("{}: <-- Computing witness stage {}", Self::MY_NAME, stage_id);
    }
}
