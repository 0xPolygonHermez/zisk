use log::debug;
use std::rc::Rc;

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::{trace, WCManager};
use wchelpers::{WCComponent, WCExecutor};

use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;

use crate::{/*FibonacciSquareTrace0,*/ FibonacciVadcopInputs, Module};

trace!(FibonacciSquareTrace0 { a: Goldilocks, b: Goldilocks });

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

    // 0:a, 1:b, 2:module
    fn calculate_verify(&self, verify: bool, values: Vec<u64>) -> Vec<u64> {
        let (a, b, module) = (values[0], values[1], values[2]);
        let tmp = b;
        let b = self.module.calculate_verify(verify, vec![a.pow(2) + b.pow(2), module])[0];
        let a = tmp;

        vec![a, b]
    }

    fn calculate_fibonacci<F>(&self, air_group_id: usize, air_id: usize, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        let pi: FibonacciVadcopInputs = pctx.public_inputs.as_slice().into();
        let (mut a, mut b, module) = pi.inner();

        let num_rows = 1 << pctx.pilout.get_air(air_group_id, air_id).num_rows();

        let mut trace = if ectx.is_discovery_execution {
            None
        } else {
            let air_instance_ctx = &mut pctx.find_air_instances(air_group_id, air_id)[0];
            let mut trace = Box::new(FibonacciSquareTrace0::from_buffer(&air_instance_ctx.buffer, num_rows, 0));

            trace.a[0] = Goldilocks::from_canonical_u64(a);
            trace.b[0] = Goldilocks::from_canonical_u64(b);
            Some(trace)
        };

        for i in 1..num_rows {
            let result = self.calculate_verify(ectx.is_discovery_execution, vec![a, b, module]);
            (a, b) = (result[0], result[1]);

            if let Some(trace) = &mut trace {
                trace.b[i] = Goldilocks::from_canonical_u64(b);
                trace.a[i] = Goldilocks::from_canonical_u64(a);
            }
        }
    }
}

impl<F> WCComponent<F> for FibonacciSquare {
    fn calculate_witness(&self, stage: u32, air_instance: &AirInstance, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        if stage != 1 {
            return;
        }

        debug!("Fibonacci: Calculating witness");
        Self::calculate_fibonacci(&self, air_instance.air_group_id, air_instance.air_id, pctx, ectx);
    }

    fn calculate_plan(&self, ectx: &mut ExecutionCtx) {
        ectx.instances.push(AirInstance::new(Self::AIR_GROUP_ID, Self::AIR_ID, None));
    }
}

impl<F> WCExecutor<F> for FibonacciSquare {
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        Self::calculate_fibonacci(&self, Self::AIR_GROUP_ID, Self::AIR_ID, pctx, ectx);
    }
}
