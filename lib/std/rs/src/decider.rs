pub trait Decider {
    fn decide<F>(
        &self,
        stage: u32,
        pilout: &PilOutProxy,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    );
}