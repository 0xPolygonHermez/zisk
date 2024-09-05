use std::sync::Arc;

use proofman_common::{ExecutionCtx, ProofCtx};
use proofman::{WitnessManager, WitnessComponent};

use p3_field::Field;
use proofman_setup::SetupCtx;

use crate::{FibonacciSquare0Trace, FibonacciSquarePublics, Module, FIBONACCI_SQUARE_SUBPROOF_ID, FIBONACCI_SQUARE_AIR_IDS};

pub struct FibonacciSquare<F> {
    module: Arc<Module<F>>,
}

impl<F: Field + Copy> FibonacciSquare<F> {
    const MY_NAME: &'static str = "FibonacciSquare";

    pub fn new(wcm: &mut WitnessManager<F>, module: Arc<Module<F>>) -> Arc<Self> {
        let fibonacci = Arc::new(Self { module });

        wcm.register_component(fibonacci.clone(), Some(FIBONACCI_SQUARE_SUBPROOF_ID));

        fibonacci
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // TODO: We should create the instance here and fill the trace in calculate witness!!!
        if let Err(e) =
            Self::calculate_trace(self, FIBONACCI_SQUARE_SUBPROOF_ID[0], FIBONACCI_SQUARE_AIR_IDS[0], pctx, ectx)
        {
            panic!("Failed to calculate fibonacci: {:?}", e);
        }
        self.module.execute(pctx, ectx);
    }

    fn calculate_trace(
        &self,
        air_group_id: usize,
        air_id: usize,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        log::info!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        let public_inputs: FibonacciSquarePublics = pctx.public_inputs.as_slice().into();

        let (module, mut a, mut b, _out) = public_inputs.inner();

        let (buffer_size, offsets) =
            ectx.buffer_allocator.as_ref().get_buffer_info("FibonacciSquare".into(), FIBONACCI_SQUARE_AIR_IDS[0])?;

        let mut buffer = vec![F::zero(); buffer_size as usize];

        let num_rows = pctx.pilout.get_air(air_group_id, air_id).num_rows();
        let mut trace = FibonacciSquare0Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize)?;

        trace[0].a = F::from_canonical_u64(a);
        trace[0].b = F::from_canonical_u64(b);

        for i in 1..num_rows {
            let tmp = b;
            let result = self.module.calculate_module(a.pow(2) + b.pow(2), module);
            (a, b) = (tmp, result);

            trace[i].a = F::from_canonical_u64(a);
            trace[i].b = F::from_canonical_u64(b);
        }
        pctx.public_inputs[24..32].copy_from_slice(&b.to_le_bytes());

        // Not needed, for debugging!
        // let mut result = F::zero();
        // for (i, _) in buffer.iter().enumerate() {
        //     result += buffer[i] * F::from_canonical_u64(i as u64);
        // }
        // log::info!("Result Fibonacci buffer: {:?}", result);

        pctx.add_air_instance_ctx(FIBONACCI_SQUARE_SUBPROOF_ID[0], FIBONACCI_SQUARE_AIR_IDS[0], Some(buffer));

        Ok(b)
    }
}

impl<F: Field + Copy> WitnessComponent<F> for FibonacciSquare<F> {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance_id: Option<usize>,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
        return;
    }
}
