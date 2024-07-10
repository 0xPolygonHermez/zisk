use log::debug;
use std::rc::Rc;

use common::{ExecutionCtx, ProofCtx};
use proofman::{trace, WCManager};
use wchelpers::WCComponent;

use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;

use crate::FibonacciVadcopInputs;

pub struct Fibonacci;

trace!(FibonacciTrace { a: Goldilocks, b: Goldilocks });

impl Fibonacci {
    pub fn new<F: AbstractField>(wcm: &mut WCManager<F>) -> Rc<Self> {
        let fibonacci = Rc::new(Fibonacci);
        wcm.register_component(Rc::clone(&fibonacci) as Rc<dyn WCComponent<F>>);

        fibonacci
    }
}

impl<F: AbstractField> WCComponent<F> for Fibonacci {
    fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx) {
        if stage != 1 {
            return;
        }

        debug!("Fibonacci: Calculating witness");
        let air_group_id =
            pctx.pilout.get_air_group_idx("FibonacciSquare").unwrap_or_else(|| panic!("Air group not found"));
        let air_id =
            pctx.pilout.get_air_idx(air_group_id, "FibonacciSquare").unwrap_or_else(|| panic!("Air not found"));
        let air = &pctx.pilout.subproofs[air_group_id].airs[air_id];

        let num_rows: usize = 1 << air.num_rows.unwrap();

        let mut trace = Box::new(FibonacciTrace::new(num_rows));

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

        pctx.air_groups[air_group_id].airs[air_id].add_trace(trace);
    }
}
