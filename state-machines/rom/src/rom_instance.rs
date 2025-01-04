use std::sync::Arc;

use crate::RomSM;
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, Instance, InstanceCtx, InstanceType};
use zisk_common::BusDevice;
use zisk_core::ZiskRom;

pub struct RomInstance {
    /// Zisk rom
    zisk_rom: Arc<ZiskRom>,

    /// Instance context
    ictx: InstanceCtx,
}

impl RomInstance {
    pub fn new(zisk_rom: Arc<ZiskRom>, ictx: InstanceCtx) -> Self {
        Self { zisk_rom, ictx }
    }
}

impl<F: PrimeField> Instance<F> for RomInstance {
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(RomSM::prove_instance(&self.zisk_rom, &self.ictx.plan))
    }

    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl BusDevice<u64> for RomInstance {}
