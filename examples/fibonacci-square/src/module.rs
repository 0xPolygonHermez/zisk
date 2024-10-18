use std::sync::{Arc, Mutex};

use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use proofman::{WitnessManager, WitnessComponent};
use pil_std_lib::Std;
use p3_field::{AbstractField, PrimeField};
use num_bigint::BigInt;

use crate::{FibonacciSquarePublics, Module0Trace, MODULE_AIRGROUP_ID, MODULE_AIR_IDS};

pub struct Module<F: PrimeField> {
    inputs: Mutex<Vec<(u64, u64)>>,
    std_lib: Arc<Std<F>>,
}

impl<F: PrimeField + AbstractField + Clone + Copy + Default + 'static> Module<F> {
    const MY_NAME: &'static str = "ModuleSM";

    pub fn new(wcm: Arc<WitnessManager<F>>, std_lib: Arc<Std<F>>) -> Arc<Self> {
        let module = Arc::new(Module { inputs: Mutex::new(Vec::new()), std_lib });

        wcm.register_component(module.clone(), Some(MODULE_AIRGROUP_ID), Some(MODULE_AIR_IDS));

        // Register dependency relations
        module.std_lib.register_predecessor();

        module
    }

    pub fn calculate_module(&self, x: u64, module: u64) -> u64 {
        let x_mod = x % module;

        let mut inputs = self.inputs.lock().unwrap();

        inputs.push((x, x_mod));

        x_mod
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        self.calculate_trace(pctx, ectx, sctx);
    }

    fn calculate_trace(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        let pi: FibonacciSquarePublics = pctx.public_inputs.inputs.read().unwrap().as_slice().into();
        let module = pi.module;

        let (buffer_size, offsets) =
            ectx.buffer_allocator.as_ref().get_buffer_info(&sctx, MODULE_AIRGROUP_ID, MODULE_AIR_IDS[0]).unwrap();

        let mut buffer = vec![F::zero(); buffer_size as usize];

        let num_rows = pctx.global_info.airs[MODULE_AIRGROUP_ID][MODULE_AIR_IDS[0]].num_rows;
        let mut trace = Module0Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize).unwrap();

        //range_check(colu: mod - x_mod, min: 1, max: 2**8-1);
        let range = self.std_lib.get_range(BigInt::from(1), BigInt::from((1 << 8) - 1), None);

        let inputs = self.inputs.lock().unwrap();

        for (i, input) in inputs.iter().enumerate() {
            let x = input.0;
            let q = x / module;
            let x_mod = input.1;

            trace[i].x = F::from_canonical_u64(x);
            trace[i].q = F::from_canonical_u64(q);
            trace[i].x_mod = F::from_canonical_u64(x_mod);

            // Check if x_mod is in the range
            self.std_lib.range_check(F::from_canonical_u64(module - x_mod), F::one(), range);
        }

        // Trivial range check for the remaining rows
        for _ in inputs.len()..num_rows {
            self.std_lib.range_check(F::from_canonical_u64(module), F::one(), range);
        }

        let air_instance = AirInstance::new(MODULE_AIRGROUP_ID, MODULE_AIR_IDS[0], Some(0), buffer);
        pctx.air_instance_repo.add_air_instance(air_instance);

        self.std_lib.unregister_predecessor(pctx, None);
    }
}

impl<F: PrimeField + AbstractField + Copy> WitnessComponent<F> for Module<F> {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance_id: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
    }
}
