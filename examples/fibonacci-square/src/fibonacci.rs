use std::sync::Arc;

use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman::{WitnessManager, WitnessComponent};

use p3_field::AbstractField;
use proofman_setup::SetupCtx;

use crate::{
    FibonacciSquare0Trace, FibonacciVadcopPublicInputs, Module, FIBONACCI_SQUARE_SUBPROOF_ID, FIBONACCI_SQUARE_AIR_IDS,
};

pub struct FibonacciSquare {
    module: Arc<Module>,
}

impl FibonacciSquare {
    pub fn new<F: AbstractField + Copy>(wcm: &mut WitnessManager<F>, module: Arc<Module>) -> Arc<Self> {
        let fibonacci = Arc::new(Self { module });

        wcm.register_component(fibonacci.clone(), Some(FIBONACCI_SQUARE_SUBPROOF_ID));

        fibonacci
    }

    // Calculate the Fibonacci sequence during the execution phase and store the trace in the buffer
    pub fn execute<F: AbstractField + Copy>(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx, _sctx: &SetupCtx) {
        if let Err(e) =
            Self::calculate_fibonacci(self, FIBONACCI_SQUARE_SUBPROOF_ID[0], FIBONACCI_SQUARE_AIR_IDS[0], pctx, ectx)
        {
            panic!("Failed to calculate fibonacci: {:?}", e);
        }
        self.module.close_module(pctx, ectx);
    }

    fn calculate_fibonacci<F: AbstractField + Copy>(
        &self,
        air_group_id: usize,
        air_id: usize,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let public_inputs: FibonacciVadcopPublicInputs = pctx.public_inputs.as_slice().into();

        let (module, mut a, mut b, _out) = public_inputs.inner();

        let (buffer_size, offsets) =
            ectx.buffer_allocator.as_ref().get_buffer_info("FibonacciSquare".into(), FIBONACCI_SQUARE_AIR_IDS[0])?;

        let mut buffer = vec![F::default(); buffer_size as usize];

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

        let mut result = F::zero();
        for (i, _) in buffer.iter().enumerate() {
            result += buffer[i] * F::from_canonical_u64(i as u64);
        }
        println!("Result Fibonacci buffer: {:?}", result);

        let mut air_instances = pctx.air_instances.write().unwrap();
        air_instances.push(AirInstanceCtx {
            air_group_id: FIBONACCI_SQUARE_SUBPROOF_ID[0],
            air_id: FIBONACCI_SQUARE_AIR_IDS[0],
            buffer: Some(buffer),
        });

        Ok(b)
    }
}

impl<F: AbstractField + Copy> WitnessComponent<F> for FibonacciSquare {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance_id: usize,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
        // Nothing to calculate, the witness is already stored in the buffer
        return;
    }
}
