use std::sync::Arc;

use p3_field::PrimeField;
use pil_std_lib::{RangeCheckAir, Std};
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, Instance, InstanceExpanderCtx, InstanceType};
use zisk_common::{BusDevice, BusId};

pub struct StdInstance<F: PrimeField> {
    std: Arc<Std<F>>,

    /// Instance expander context
    iectx: InstanceExpanderCtx,
}

impl<F: PrimeField> StdInstance<F> {
    pub fn new(std: Arc<Std<F>>, iectx: InstanceExpanderCtx) -> Self {
        Self { std, iectx }
    }
}

impl<F: PrimeField> Instance<F> for StdInstance<F> {
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        let plan = &self.iectx.plan;
        let rc_type = plan.meta.as_ref().unwrap().downcast_ref::<RangeCheckAir>().unwrap();

        self.std.drain_inputs(rc_type);

        None
    }

    fn check_point(&self) -> CheckPoint {
        self.iectx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl<F: PrimeField> BusDevice<u64> for StdInstance<F> {
    fn process_data(&mut self, _bus_id: &BusId, _data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        (true, vec![])
    }
}

unsafe impl<F: PrimeField> Sync for StdInstance<F> {}
