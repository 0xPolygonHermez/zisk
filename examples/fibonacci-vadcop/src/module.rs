use log::debug;
use std::rc::Rc;

use common::{ExecutionCtx, ProofCtx};
use proofman::{trace, WCManager};
use wchelpers::WCComponent;

use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;
use crate::{FibonacciVadcopInputs, ModuleTrace0};

trace!(ModuleTrace { x: Goldilocks, q: Goldilocks, x_mod: Goldilocks });

pub struct Module;

impl Module {
    pub fn new(wcm: &mut WCManager) -> Rc<Self> {
        let module = Rc::new(Module);
        wcm.register_component(Rc::clone(&module) as Rc<dyn WCComponent>);

        module
    }
}

impl WCComponent for Module {
    fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx, _ectx: &ExecutionCtx) {
        if stage != 1 {
            return;
        }

        debug!("Module   : Calculating witness");
        const AIR_GROUP_ID: usize = 1;
        const AIR_ID: usize = 0;
        const INSTANCE_ID: usize = 1;

        let air = pctx.pilout.find_air(AIR_GROUP_ID, AIR_ID);
        let air_instance_ctx = &mut pctx.air_instances[INSTANCE_ID];

        let num_rows: usize = 1 << air.num_rows();

        let mut trace = Box::new(ModuleTrace0::from_buffer(&air_instance_ctx.buffer, num_rows));

        let pi: FibonacciVadcopInputs = pctx.public_inputs.as_slice().into();
        let mut a = pi.a as u64;
        let mut b = pi.b as u64;
        let module = pi.module as u64;

        for i in 0..num_rows {
            let x = a * a + b * b;
            let q = x / module;
            let x_mod = x % module;

            trace.x[i] = Goldilocks::from_canonical_u64(x);
            trace.q[i] = Goldilocks::from_canonical_u64(q);
            trace.x_mod[i] = Goldilocks::from_canonical_u64(x_mod);

            a = b;
            b = x_mod;
        }
    }
}
