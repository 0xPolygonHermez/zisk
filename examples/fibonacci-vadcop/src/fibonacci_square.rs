use log::debug;
use std::rc::Rc;

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::{WCComponent, WCExecutor};

use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;

use crate::{FibonacciSquareTrace0, FibonacciVadcopInputs};

pub struct FibonacciSquare;

impl FibonacciSquare {
    pub fn new(wcm: &mut WCManager) -> Rc<Self> {
        let fibonacci = Rc::new(FibonacciSquare);
        wcm.register_component(Rc::clone(&fibonacci) as Rc<dyn WCComponent>);
        wcm.register_executor(Rc::clone(&fibonacci) as Rc<dyn WCExecutor>);
        fibonacci
    }
}

impl WCComponent for FibonacciSquare {
    fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx, _ectx: &ExecutionCtx) {
        if stage != 1 {
            return;
        }

        debug!("Fibonacci: Calculating witness");
        const AIR_GROUP_ID: usize = 0;
        const AIR_ID: usize = 0;
        const INSTANCE_ID: usize = 0;

        let air = pctx.pilout.find_air(AIR_GROUP_ID, AIR_ID);
        let air_instance_ctx = &mut pctx.air_instances[INSTANCE_ID];
        println!("air.air_id(): {}", air.air_id());
        let num_rows: usize = 1 << air.num_rows();

        let mut trace = Box::new(FibonacciSquareTrace0::from_buffer(&air_instance_ctx.buffer, num_rows));

        let pi: FibonacciVadcopInputs = pctx.public_inputs.as_slice().into();
        let mut a = pi.a as u64;
        let mut b = pi.b as u64;

        let module = pi.module as u64;

        trace.a[0] = Goldilocks::from_canonical_u64(a);
        trace.b[0] = Goldilocks::from_canonical_u64(b);

        for i in 1..num_rows {
            let tmp = b;
            b = (a.pow(2) + b.pow(2)) % module;
            a = tmp;

            trace.b[i] = Goldilocks::from_canonical_u64(b);
            trace.a[i] = Goldilocks::from_canonical_u64(a);
        }
    }
}

impl WCExecutor for FibonacciSquare {
    fn execute(&self, _pctx: &mut ProofCtx, ectx: &mut ExecutionCtx) {
        ectx.instances.extend(vec![
            AirInstance::new(0, 0, 1 << 10),
            AirInstance::new(1, 1, 1 << 10),
            AirInstance::new(2, 2, 1 << 8),
        ]);
    }
}
