use std::sync::Arc;

use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use proofman::{WitnessManager, WitnessComponent};

use p3_field::PrimeField;

use crate::{FibonacciSquare0Trace, FibonacciSquarePublics, Module, FIBONACCI_SQUARE_AIRGROUP_ID, FIBONACCI_SQUARE_AIR_IDS};

pub struct FibonacciSquare<F: PrimeField> {
    module: Arc<Module<F>>,
}

impl<F: PrimeField + Copy> FibonacciSquare<F> {
    const MY_NAME: &'static str = "FibonacciSquare";

    pub fn new(wcm: &mut WitnessManager<F>, module: Arc<Module<F>>) -> Arc<Self> {
        let fibonacci = Arc::new(Self { module });

        wcm.register_component(fibonacci.clone(), Some(FIBONACCI_SQUARE_AIRGROUP_ID), Some(FIBONACCI_SQUARE_AIR_IDS));

        fibonacci
    }

    pub fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // TODO: We should create the instance here and fill the trace in calculate witness!!!
        if let Err(e) =
            Self::calculate_trace(self, FIBONACCI_SQUARE_AIRGROUP_ID, FIBONACCI_SQUARE_AIR_IDS[0], pctx, ectx)
        {
            panic!("Failed to calculate fibonacci: {:?}", e);
        }
        self.module.execute(pctx, ectx);
    }

    fn calculate_trace(
        &self,
        airgroup_id: usize,
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

        let num_rows = pctx.pilout.get_air(airgroup_id, air_id).num_rows();
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

        // hint!

        pctx.public_inputs[24..32].copy_from_slice(&b.to_le_bytes());

        // Not needed, for debugging!
        // let mut result = F::zero();
        // for (i, _) in buffer.iter().enumerate() {
        //     result += buffer[i] * F::from_canonical_u64(i as u64);
        // }
        // log::info!("Result Fibonacci buffer: {:?}", result);

        let air_instance = AirInstance::new(FIBONACCI_SQUARE_AIRGROUP_ID, FIBONACCI_SQUARE_AIR_IDS[0], Some(0), buffer);
        pctx.air_instance_repo.add_air_instance(air_instance);

        Ok(b)
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for FibonacciSquare<F> {
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
