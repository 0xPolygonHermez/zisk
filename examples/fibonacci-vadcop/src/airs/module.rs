use log::debug;
use std::rc::Rc;

use common::{ExecutionCtx, ProofCtx};
use proofman::{trace, WCManager};
use wchelpers::WCComponent;

use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;
use crate::FibonacciVadcopInputs;

trace!(ModuleTrace { x: Goldilocks, q: Goldilocks, x_mod: Goldilocks });

pub struct Module;

impl Module {
    pub fn new<F: AbstractField>(wcm: &mut WCManager<F>) -> Rc<Self> {
        let module = Rc::new(Module);
        wcm.register_component(Rc::clone(&module) as Rc<dyn WCComponent<F>>);

        module
    }
}

impl<F: AbstractField> WCComponent<F> for Module {
    fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) {
        if stage != 1 {
            return;
        }

        debug!("Module   : Calculating witness");
        let air_group_id = pctx.pilout.get_air_group_idx("Module").unwrap_or_else(|| panic!("Air group not found"));
        let air_id = pctx.pilout.get_air_idx(air_group_id, "Module").unwrap_or_else(|| panic!("Air not found"));
        let air = &pctx.pilout.subproofs[air_group_id].airs[air_id];

        let num_rows: usize = 1 << air.num_rows.unwrap();

        let mut trace = Box::new(ModuleTrace::new(num_rows));

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

        pctx.air_groups[air_group_id].airs[air_id].add_trace(trace);
    }
}
