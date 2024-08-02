use log::debug;
use std::{cell::RefCell, sync::Arc};

use common::{AirInstance, ExecutionCtx, ProofCtx, Prover};
use proofman::WCManager;
use wchelpers::{WCComponent, WCOpCalculator};

use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;
use crate::{FibonacciVadcopPublicInputs, ModuleTrace, MODULE_SUBPROOF_ID, MODULE_AIR_IDS};

pub struct Module {
    inputs: RefCell<Vec<(u64, u64)>>,
}

impl Module {
    pub fn new<F>(wcm: &mut WCManager<F>) -> Arc<Self> {
        let module = Arc::new(Module { inputs: RefCell::new(Vec::new()) });
        wcm.register_component(Arc::clone(&module) as Arc<dyn WCComponent<F>>, Some(MODULE_SUBPROOF_ID));

        module
    }
    pub fn new_no_register<F>(wcm: &mut WCManager<F>) -> Arc<Self> {
        let module = Arc::new(Module { inputs: RefCell::new(Vec::new()) });
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
    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        provers: &Vec<Box<dyn Prover<F>>>,
    ) {
        if stage != 1 {
            return;
        }

        debug!("Module  : Calculating witness");

        let pi: FibonacciVadcopPublicInputs = pctx.public_inputs.as_slice().into();
        let module = pi.module as u64;

        let (air_idx, air_instance_ctx) = &mut pctx.find_air_instances(MODULE_SUBPROOF_ID[0], MODULE_AIR_IDS[0])[0];

        let interval = air_instance.inputs_interval.unwrap();
        let inputs = &self.inputs.borrow()[interval.0..interval.1];
        let offset = (provers[*air_idx].get_map_offsets("cm1", false) * 8) as usize;
        let num_rows = pctx.pilout.get_air(MODULE_SUBPROOF_ID[0], MODULE_AIR_IDS[0]).num_rows();
        let mut trace = unsafe { Box::new(ModuleTrace::from_buffer(&air_instance_ctx.buffer, num_rows, offset)) };

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
            MODULE_SUBPROOF_ID[0],
            MODULE_AIR_IDS[0],
            Some((0, self.inputs.borrow().len())),
        ));
    }
}
