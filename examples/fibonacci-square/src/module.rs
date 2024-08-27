use log::debug;
use pil_std_lib::Std;
use proofman_setup::SetupCtx;
use std::{cell::RefCell, sync::Arc};

use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman::{WitnessManager, WitnessComponent};

use p3_field::AbstractField;
use crate::{FibonacciVadcopPublicInputs, Module0Trace, MODULE_SUBPROOF_ID, MODULE_AIR_IDS};

pub struct Module {
    inputs: RefCell<Vec<(u64, u64)>>,
    std_lib: Arc<Std<F>>,
}

impl Module {
    pub fn new<F: AbstractField + Copy>(wcm: &mut WitnessManager<F>, std_lib: Arc<Std<F>>) -> Arc<Self> {
        let module = Arc::new(Module { inputs: RefCell::new(Vec::new()), std_lib });

        wcm.register_component(module.clone(), Some(MODULE_SUBPROOF_ID));

        module
    }

    pub fn calculate_module(&self, x: u64, module: u64) -> u64 {
        let x_mod = x % module;

        self.inputs.borrow_mut().push((x, x_mod));

        x_mod
    }

    pub fn close_module<F: AbstractField + Clone + Copy + Default>(&self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        debug!("Module  : Calculating witness");

        let pi: FibonacciVadcopPublicInputs = pctx.public_inputs.as_slice().into();
        let module = pi.module;

        let (buffer_size, offsets) =
            ectx.buffer_allocator.as_ref().get_buffer_info("Module".to_owned(), MODULE_AIR_IDS[0]).unwrap();

        let mut buffer = vec![F::zero(); buffer_size as usize];

        let num_rows = pctx.pilout.get_air(MODULE_SUBPROOF_ID[0], MODULE_AIR_IDS[0]).num_rows();
        let mut trace = Module0Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize).unwrap();

        for (i, input) in self.inputs.borrow().iter().enumerate() {
            let x = input.0;
            let q = x / module;
            let x_mod = input.1;

            trace[i].x = F::from_canonical_u64(x);
            trace[i].q = F::from_canonical_u64(q);
            trace[i].x_mod = F::from_canonical_u64(x_mod);
        }

        let mut result = F::zero();
        for (i, _) in buffer.iter().enumerate() {
            result += buffer[i] * F::from_canonical_u64(i as u64);
        }
        println!("Result Module    buffer: {:?}", result);

        let mut air_instances = pctx.air_instances.write().unwrap();
        air_instances.push(AirInstanceCtx {
            air_group_id: MODULE_SUBPROOF_ID[0],
            air_id: MODULE_AIR_IDS[0],
            buffer: Some(buffer),
        });
    }
}

impl<F: AbstractField + Copy> WitnessComponent<F> for Module {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance_id: usize,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
    }
}
