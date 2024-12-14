use std::sync::Arc;

use p3_field::PrimeField;
use proofman_common::{AirInstance, FromTrace};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::ZiskRom;
use zisk_pil::RomTrace;

use crate::RomSM;

pub struct RomInstance<F: PrimeField> {
    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Zisk ROM
    zisk_rom: Arc<ZiskRom>,

    /// ROM trace
    trace: RomTrace<F>,
}

impl<F: PrimeField> RomInstance<F> {
    pub fn new(zisk_rom: Arc<ZiskRom>, iectx: InstanceExpanderCtx) -> Self {
        Self { iectx, zisk_rom, trace: RomTrace::new() }
    }
}

impl<F: PrimeField> Instance<F> for RomInstance<F> {
    fn compute_witness(&mut self) -> Option<AirInstance<F>> {
        RomSM::prove_instance(
            &self.zisk_rom,
            &self.iectx.plan,
            &mut self.trace,
            RomTrace::<F>::NUM_ROWS,
        );

        Some(AirInstance::new_from_trace(FromTrace::new(&mut self.trace)))
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl<F: PrimeField> Sync for RomInstance<F> {}
