use log::debug;
use std::rc::Rc;

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::{WCComponent, WCExecutor};

use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;

use crate::{FibonacciSquareTrace0, FibonacciVadcopInputs, Module};

pub struct FibonacciSquare {
    module: Rc<Module>,
}

impl FibonacciSquare {
    const AIR_GROUP_ID: usize = 0;
    const AIR_ID: usize = 0;

    pub fn new<F>(wcm: &mut WCManager<F>, module: &Rc<Module>) -> Rc<Self> {
        let fibonacci = Rc::new(Self { module: Rc::clone(&module) });
        wcm.register_component(Rc::clone(&fibonacci) as Rc<dyn WCComponent<F>>);
        wcm.register_executor(Rc::clone(&fibonacci) as Rc<dyn WCExecutor<F>>);
        fibonacci
    }

    fn calculate(&self, a: u64, b: u64, module: u64, generate_inputs: bool) -> (u64, u64) {
        let tmp = b;
        let b = self.module.calculate(a.pow(2) + b.pow(2), module, generate_inputs);
        let a = tmp;

        (a, b)
    }

    fn calculate_fibonacci<F>(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        let pi: FibonacciVadcopInputs = pctx.public_inputs.as_slice().into();
        let (mut a, mut b, module) = pi.inner();
        let num_rows = 1 << pctx.pilout.get_air(Self::AIR_GROUP_ID, Self::AIR_ID).num_rows();

        let mut trace = if ectx.is_discovery_execution {
            // Create a dummy trace for discovery execution
            Box::new(FibonacciSquareTrace0::<Goldilocks>::new(2))
        } else {
            let air_instance_ctx = &mut pctx.find_air_instances(Self::AIR_GROUP_ID, Self::AIR_ID)[0];
            let mut trace = Box::new(FibonacciSquareTrace0::from_buffer(&air_instance_ctx.buffer, num_rows));

            trace.a[0] = Goldilocks::from_canonical_u64(a);
            trace.b[0] = Goldilocks::from_canonical_u64(b);

            trace
        };

        for i in 1..num_rows {
            (a, b) = self.calculate(a, b, module, ectx.is_discovery_execution);

            if !ectx.is_discovery_execution {
                trace.b[i] = Goldilocks::from_canonical_u64(b);
                trace.a[i] = Goldilocks::from_canonical_u64(a);
            }
        }
    }
}

impl<F> WCComponent<F> for FibonacciSquare {
    fn calculate_witness(&self, stage: u32, _air_instance: &AirInstance, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        if stage != 1 {
            return;
        }

        debug!("Fibonacci: Calculating witness");
        Self::calculate_fibonacci(&self, pctx, ectx);
    }

    fn calculate_plan(&self, ectx: &mut ExecutionCtx) {
        ectx.instances.push(AirInstance::new(Self::AIR_GROUP_ID, Self::AIR_ID, 1 << 10, None));
    }
}

impl<F> WCExecutor<F> for FibonacciSquare {
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        Self::calculate_fibonacci(&self, pctx, ectx);
    }
}
