use log::debug;
use std::{cell::RefCell, rc::Rc};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::{trace, WCManager};
use wchelpers::WCComponent;

use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;
use crate::FibonacciVadcopInputs;

trace!(ModuleTrace0 { x: Goldilocks, q: Goldilocks, x_mod: Goldilocks });

pub struct Module {
    inputs: RefCell<Vec<(u64, u64)>>,
}

impl Module {
    const AIR_GROUP_ID: usize = 1;
    const AIR_ID: usize = 0;

    pub fn new<F>(wcm: &mut WCManager<F>) -> Rc<Self> {
        let module = Rc::new(Module { inputs: RefCell::new(Vec::new()) });
        wcm.register_component(Rc::clone(&module) as Rc<dyn WCComponent<F>>);

        module
    }

    // 0:x, 1:module
    pub fn calculate_verify(&self, verify: bool, values: Vec<u64>) -> Vec<u64> {
        let (x, module) = (values[0], values[1]);

        let x_mod = x % module;

        if verify {
            self.inputs.borrow_mut().push((x, x_mod));
        }

        vec![x_mod]
    }
}

impl<F> WCComponent<F> for Module {
    fn calculate_witness(&self, stage: u32, air_instance: &AirInstance, pctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) {
        if stage != 1 {
            return;
        }

        debug!("Module   : Calculating witness");

        let pi: FibonacciVadcopInputs = pctx.public_inputs.as_slice().into();
        let module = pi.module as u64;

        let air_instance_ctx = &mut pctx.find_air_instances(Self::AIR_GROUP_ID, Self::AIR_ID)[0];

        let interval = air_instance.inputs_interval.unwrap();
        let inputs = &self.inputs.borrow()[interval.0..interval.1];

        let num_rows = 1 << pctx.pilout.get_air(Self::AIR_GROUP_ID, Self::AIR_ID).num_rows();
        let mut trace = Box::new(ModuleTrace0::from_buffer(&air_instance_ctx.buffer, num_rows, 0));

        for (i, input) in inputs.iter().enumerate() {
            let x = input.0;
            let q = x / module;
            let x_mod = input.1;

            trace.x[i] = Goldilocks::from_canonical_u64(x);
            trace.q[i] = Goldilocks::from_canonical_u64(q);
            trace.x_mod[i] = Goldilocks::from_canonical_u64(x_mod);
        }

        for i in inputs.len()..num_rows {
            trace.x[i] = Goldilocks::zero();
            trace.q[i] = Goldilocks::zero();
            trace.x_mod[i] = Goldilocks::zero();
        }
    }

    fn calculate_plan(&self, ectx: &mut ExecutionCtx) {
        ectx.instances.push(AirInstance::new(Self::AIR_GROUP_ID, Self::AIR_ID, Some((0, self.inputs.borrow().len()))));
    }
}
