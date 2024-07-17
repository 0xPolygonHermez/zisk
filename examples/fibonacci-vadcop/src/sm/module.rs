use log::debug;
use std::{cell::RefCell, rc::Rc};

use common::{AirInstance, ExecutionCtx, ProofCtx};
use proofman::WCManager;
use wchelpers::{WCComponent, WCOpCalculator};

use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;
use crate::{FibonacciVadcopPublicInputs, ModuleTrace0, MODULE_0_AIR_ID, MODULE_AIR_GROUP_ID};

pub struct Module {
    inputs: RefCell<Vec<(u64, u64)>>,
}

impl Module {
    pub fn new<F>(wcm: &mut WCManager<F>) -> Rc<Self> {
        let module = Rc::new(Module { inputs: RefCell::new(Vec::new()) });
        wcm.register_component(Rc::clone(&module) as Rc<dyn WCComponent<F>>, None);

        module
    }
}

impl WCOpCalculator for Module {
    // 0:x, 1:module
    fn calculate_verify(&self, verify: bool, values: Vec<u64>) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
        let (x, module) = (values[0], values[1]);

        let x_mod = x % module;

        if verify {
            self.inputs.borrow_mut().push((x.into(), x_mod.into()));
        }

        Ok(vec![x_mod])
    }
}

impl<F> WCComponent<F> for Module {
    fn calculate_witness(&self, stage: u32, air_instance: &AirInstance, pctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) {
        if stage != 1 {
            return;
        }

        debug!("Module  : Calculating witness");

        let pi: FibonacciVadcopPublicInputs = pctx.public_inputs.as_slice().into();
        let module = pi.module as u64;

        let air_instance_ctx = &mut pctx.find_air_instances(MODULE_AIR_GROUP_ID, MODULE_0_AIR_ID)[0];

        let interval = air_instance.inputs_interval.unwrap();
        let inputs = &self.inputs.borrow()[interval.0..interval.1];

        let num_rows = 1 << pctx.pilout.get_air(MODULE_AIR_GROUP_ID, MODULE_0_AIR_ID).num_rows();
        let mut trace = Box::new(ModuleTrace0::from_buffer(&air_instance_ctx.buffer, num_rows, 0));

        for (i, input) in inputs.iter().enumerate() {
            let x = input.0;
            let q = x / module;
            let x_mod = input.1;

            trace.x[i] = Goldilocks::from_canonical_u64(x as u64);
            trace.q[i] = Goldilocks::from_canonical_u64(q as u64);
            trace.x_mod[i] = Goldilocks::from_canonical_u64(x_mod as u64);
        }

        for i in inputs.len()..num_rows {
            trace.x[i] = Goldilocks::zero();
            trace.q[i] = Goldilocks::zero();
            trace.x_mod[i] = Goldilocks::zero();
        }
    }

    fn suggest_plan(&self, ectx: &mut ExecutionCtx) {
        ectx.instances.push(AirInstance::new(
            MODULE_AIR_GROUP_ID,
            MODULE_0_AIR_ID,
            Some((0, self.inputs.borrow().len())),
        ));
    }
}
