use log::debug;
use std::rc::Rc;

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::{WCComponent, WCExecutor, WCOpCalculator};

use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;

use crate::{FibonacciSquareTrace0, FibonacciVadcopPublicInputs, Module, FIBONACCI_0_AIR_ID, FIBONACCI_AIR_GROUP_ID};

pub struct FibonacciSquare {
    module: Rc<Module>,
}

impl FibonacciSquare {
    pub fn new<F>(wcm: &mut WCManager<F>, module: &Rc<Module>) -> Rc<Self> {
        let fibonacci = Rc::new(Self { module: Rc::clone(&module) });
        wcm.register_component(Rc::clone(&fibonacci) as Rc<dyn WCComponent<F>>);
        wcm.register_executor(Rc::clone(&fibonacci) as Rc<dyn WCExecutor<F>>);
        fibonacci
    }

    fn calculate_fibonacci<F>(
        &self,
        air_group_id: usize,
        air_id: usize,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let pi: FibonacciVadcopPublicInputs = pctx.public_inputs.as_slice().into();
        let (mut a, mut b, module) = pi.inner();

        let num_rows = 1 << pctx.pilout.get_air(air_group_id, air_id).num_rows();

        let mut trace = if ectx.discovering {
            None
        } else {
            let air_instance_ctx = &mut pctx.find_air_instances(air_group_id, air_id)[0];
            let mut trace = Box::new(FibonacciSquareTrace0::from_buffer(&air_instance_ctx.buffer, num_rows, 0));

            trace.a[0] = Goldilocks::from_canonical_u64(a);
            trace.b[0] = Goldilocks::from_canonical_u64(b);
            Some(trace)
        };

        for i in 1..num_rows {
            let tmp = b;
            let result = self.module.calculate_verify(ectx.discovering, vec![a.pow(2) + b.pow(2), module])?;
            (a, b) = (tmp, result[0]);

            if let Some(trace) = &mut trace {
                trace.b[i] = Goldilocks::from_canonical_u64(b);
                trace.a[i] = Goldilocks::from_canonical_u64(a);
            }
        }

        Ok(b)
    }
}

impl<F> WCComponent<F> for FibonacciSquare {
    fn calculate_witness(&self, stage: u32, air_instance: &AirInstance, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        if stage != 1 {
            return;
        }

        debug!("Fiboncci: Calculating witness");
        Self::calculate_fibonacci(&self, air_instance.air_group_id, air_instance.air_id, pctx, ectx).unwrap();
    }

    fn calculate_plan(&self, ectx: &mut ExecutionCtx) {
        ectx.instances.push(AirInstance::new(FIBONACCI_AIR_GROUP_ID, FIBONACCI_0_AIR_ID, None));
    }
}

impl<F> WCExecutor<F> for FibonacciSquare {
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        Self::calculate_fibonacci(&self, FIBONACCI_AIR_GROUP_ID, FIBONACCI_0_AIR_ID, pctx, ectx).unwrap();
    }
}
