use std::sync::Arc;

use p3_field::PrimeField;
use proofman::WitnessManager;
use proofman_common::AirInstance;
use sm_common::{InstanceExpanderCtx, InstanceXXXX};
use zisk_core::ZiskRom;
use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};
use ziskemu::EmuTrace;

use crate::RomSM;

pub struct RomInstance<F: PrimeField> {
    wcm: Arc<WitnessManager<F>>,
    zisk_rom: Arc<ZiskRom>,
    iectx: InstanceExpanderCtx<F>,
}

impl<F: PrimeField> RomInstance<F> {
    pub fn new(
        wcm: Arc<WitnessManager<F>>,
        zisk_rom: Arc<ZiskRom>,
        iectx: InstanceExpanderCtx<F>,
    ) -> Self {
        Self { wcm, zisk_rom, iectx }
    }
}
impl<F: PrimeField> InstanceXXXX for RomInstance<F> {
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
            &self.zisk_rom,
            &mut self.iectx.plan,
            &mut self.iectx.buffer,
            air.num_rows(),
        );

        let buffer = std::mem::take(&mut self.iectx.buffer.buffer);

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
}

unsafe impl<F: PrimeField> Sync for RomInstance<F> {}
