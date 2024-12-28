use std::sync::Arc;

use crate::RomSM;
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, Instance, InstanceExpanderCtx, InstanceType};
use zisk_common::BusDevice;
use zisk_core::ZiskRom;

pub struct RomInstance {
    // Zisk rom
    zisk_rom: Arc<ZiskRom>,
    
    // Instance expander context
    iectx: InstanceExpanderCtx,
}

impl RomInstance {
    pub fn new(zisk_rom: Arc<ZiskRom>, iectx: InstanceExpanderCtx) -> Self {
        Self { zisk_rom, iectx }
    }
}

impl<F: PrimeField> Instance<F> for RomInstance {
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(RomSM::prove_instance(&self.zisk_rom, &self.iectx.plan))
    }

    fn check_point(&self) -> CheckPoint {
        self.iectx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl BusDevice<u64> for RomInstance {}
