use std::sync::Arc;

use p3_field::PrimeField;
use proofman::WitnessManager;
use proofman_common::AirInstance;
use sm_common::{Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::ZiskRom;
use zisk_pil::{RomTrace, ROM_AIR_IDS, ZISK_AIRGROUP_ID};
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
        let pctx = wcm.get_pctx();
        let plan = &iectx.plan;
        let air = pctx.pilout.get_air(plan.airgroup_id, plan.air_id);
        let rom_trace = RomTrace::new(air.num_rows());

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
        let pctx = self.wcm.get_pctx();
        let plan = &self.iectx.plan;
        let air = pctx.pilout.get_air(plan.airgroup_id, plan.air_id);

        RomSM::prove_instance(
            &self.wcm,
            &self.zisk_rom,
            &self.iectx.plan,
            &mut self.rom_trace,
            air.num_rows(),
        );

        let buffer = std::mem::take(&mut self.rom_trace.buffer);
        let buffer: Vec<F> = unsafe { std::mem::transmute(buffer) };

        let air_instance = AirInstance::new(
            self.wcm.get_sctx().clone(),
            ZISK_AIRGROUP_ID,
            ROM_AIR_IDS[0],
            self.iectx.plan.segment_id,
            buffer,
        );

        self.wcm
            .get_pctx()
            .air_instance_repo
            .add_air_instance(air_instance, Some(self.iectx.instance_global_idx));
        Ok(())
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl<F: PrimeField> Sync for RomInstance<F> {}
