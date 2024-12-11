use std::sync::Arc;

use p3_field::PrimeField;
use pil_std_lib::{RangeCheckAir, Std};
use proofman_common::AirInstance;
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::ZiskRom;
use ziskemu::EmuTrace;

pub struct StdInstance<F: PrimeField> {
    std: Arc<Std<F>>,
    iectx: InstanceExpanderCtx,
}

impl<F: PrimeField> StdInstance<F> {
    pub fn new(std: Arc<Std<F>>, iectx: InstanceExpanderCtx) -> Self {
        Self { std, iectx }
    }
}

impl<F: PrimeField> Instance<F> for StdInstance<F> {
    fn collect(
        &mut self,
        _: &ZiskRom,
        _: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    fn compute_witness(&mut self) -> Option<AirInstance<F>> {
        let plan = &self.iectx.plan;
        let rc_type = plan.meta.as_ref().unwrap().downcast_ref::<RangeCheckAir>().unwrap();

        self.std.drain_inputs(rc_type);

        None
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Table
    }
}

unsafe impl<F: PrimeField> Sync for StdInstance<F> {}
