use std::sync::Arc;

use p3_field::PrimeField;
use proofman_common::{AirInstance, FromTrace, ProofCtx};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::ZiskRom;
use zisk_pil::{RomRomTrace, RomTrace};

use crate::RomSM;

pub struct RomInstance<F: PrimeField> {
    /// Instance expander context
    zisk_rom: Arc<ZiskRom>,
    iectx: InstanceExpanderCtx,
    rom_trace: RomTrace<F>,
    rom_custom_trace: RomRomTrace<F>,
}

impl<F: PrimeField> RomInstance<F> {
    pub fn new(zisk_rom: Arc<ZiskRom>, iectx: InstanceExpanderCtx) -> Self {
        let rom_trace = RomTrace::new();
        let rom_custom_trace = RomRomTrace::new();

        Self { zisk_rom, iectx, rom_trace, rom_custom_trace }
    }
}

impl<F: PrimeField> Instance<F> for RomInstance<F> {
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        RomSM::prove_instance(
            &self.zisk_rom,
            &self.iectx.plan,
            &mut self.rom_trace,
            &mut self.rom_custom_trace,
        );

        Some(AirInstance::new_from_trace(
            FromTrace::new(&mut self.rom_trace)
                .with_custom_traces(vec![&mut self.rom_custom_trace]),
        ))
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl<F: PrimeField> Sync for RomInstance<F> {}
