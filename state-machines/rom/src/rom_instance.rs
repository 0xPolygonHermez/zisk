use std::sync::Arc;

use p3_field::PrimeField;
use proofman::WitnessManager;
use proofman_common::{AirInstance, FromTrace};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::ZiskRom;
use zisk_pil::RomTrace;
use ziskemu::EmuTrace;

use crate::RomSM;

pub struct RomInstance<F: PrimeField> {
    wcm: Arc<WitnessManager<F>>,
    zisk_rom: Arc<ZiskRom>,
    iectx: InstanceExpanderCtx,
    rom_trace: RomTrace<F>,
}

impl<F: PrimeField> RomInstance<F> {
    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        zisk_rom: Arc<ZiskRom>,
        iectx: InstanceExpanderCtx,
    ) -> Self {
        let rom_trace = RomTrace::new();

        Self { wcm, zisk_rom, iectx, rom_trace }
    }
}

impl<F: PrimeField> Instance for RomInstance<F> {
    fn expand(
        &mut self,
        _zisk_rom: &ZiskRom,
        _min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    fn prove(
        &mut self,
        _min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        RomSM::prove_instance(
            &self.zisk_rom,
            &self.iectx.plan,
            &mut self.rom_trace,
            RomTrace::<F>::NUM_ROWS,
        );

        let air_instance =
            AirInstance::new_from_trace(self.wcm.get_sctx(), FromTrace::new(&mut self.rom_trace));

        self.wcm
            .get_pctx()
            .air_instance_repo
            .add_air_instance(air_instance, Some(self.iectx.global_idx));

        Ok(())
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl<F: PrimeField> Sync for RomInstance<F> {}
