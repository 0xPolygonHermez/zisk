use std::sync::Arc;

use p3_field::PrimeField;
use proofman_common::{AirInstance, FromTrace};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::ZiskRom;
use zisk_pil::RomTrace;

use crate::RomSM;

pub struct RomInstance<F: PrimeField> {
    zisk_rom: Arc<ZiskRom>,
    iectx: InstanceExpanderCtx,
    rom_trace: RomTrace<F>,
}

impl<F: PrimeField> RomInstance<F> {
    pub fn new(zisk_rom: Arc<ZiskRom>, iectx: InstanceExpanderCtx) -> Self {
        let rom_trace = RomTrace::new();

        Self { zisk_rom, iectx, rom_trace }
    }
}

impl<F: PrimeField> Instance<F> for RomInstance<F> {
    fn compute_witness(&mut self) -> Option<AirInstance<F>> {
        RomSM::prove_instance(
            &self.zisk_rom,
            &self.iectx.plan,
            &mut self.rom_trace,
            RomTrace::<F>::NUM_ROWS,
        );

        let instance = AirInstance::new_from_trace(FromTrace::new(&mut self.rom_trace));

        Some(instance)
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl<F: PrimeField> Sync for RomInstance<F> {}
