use log::debug;
use std::rc::Rc;

use common::{AirInstance, ExecutionCtx, ProofCtx, Prover};
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
        provers: &Vec<Box<dyn Prover<F>>>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let pi: FibonacciVadcopPublicInputs = pctx.public_inputs.as_slice().into();
        let (module, mut a, mut b, out) = pi.inner();
        let num_rows = 1 << pctx.pilout.get_air(air_group_id, air_id).num_rows();

        let mut trace = if ectx.discovering {
            None
        } else {
            let (air_idx, air_instance_ctx): &mut (usize, &common::AirInstanceCtx) =
                &mut pctx.find_air_instances(air_group_id, air_id)[0];
            let offset = (provers[*air_idx].get_map_offsets("cm1", false) * 8) as usize;
            let mut trace = Box::new(FibonacciSquareTrace0::from_buffer(&air_instance_ctx.buffer, num_rows, offset));

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
        pctx.public_inputs[24..32].copy_from_slice(&b.to_le_bytes());
        Ok(b)
    }
}

impl<F> WCComponent<F> for FibonacciSquare {
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        provers: &Vec<Box<dyn Prover<F>>>,
    ) {
        if stage != 1 {
            return;
        }

        debug!("Fiboncci: Calculating witness");
        Self::calculate_fibonacci(&self, air_instance.air_group_id, air_instance.air_id, pctx, ectx, provers).unwrap();
    }

    fn calculate_plan(&self, ectx: &mut ExecutionCtx) {
        ectx.instances.push(AirInstance::new(FIBONACCI_AIR_GROUP_ID, FIBONACCI_0_AIR_ID, None));
    }
}

impl<F> WCExecutor<F> for FibonacciSquare {
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        let _provers: Vec<Box<dyn Prover<F>>> = Vec::new();
        Self::calculate_fibonacci(&self, FIBONACCI_AIR_GROUP_ID, FIBONACCI_0_AIR_ID, pctx, ectx, &_provers).unwrap();
    }
}
