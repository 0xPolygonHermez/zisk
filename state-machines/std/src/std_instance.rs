use std::sync::Arc;

use p3_field::PrimeField;
use pil_std_lib::{RangeCheckAir, Std};
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, Instance, InstanceCtx, InstanceType};
use zisk_common::BusDevice;

pub struct StdInstance<F: PrimeField> {
    /// PIL2 standard library
    std: Arc<Std<F>>,

    /// Instance context
    ictx: InstanceCtx,
}

impl<F: PrimeField> StdInstance<F> {
    pub fn new(std: Arc<Std<F>>, ictx: InstanceCtx) -> Self {
        Self { std, ictx }
    }
}

impl<F: PrimeField> Instance<F> for StdInstance<F> {
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        let plan = &self.ictx.plan;
        let rc_type = plan.meta.as_ref().unwrap().downcast_ref::<RangeCheckAir>().unwrap();

        self.std.drain_inputs(rc_type);

        None
    }

    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl<F: PrimeField> BusDevice<u64> for StdInstance<F> {}
