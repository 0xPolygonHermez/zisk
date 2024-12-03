// use std::sync::Arc;

// use p3_field::PrimeField;
// use proofman::WitnessManager;
// use proofman_common::AirInstance;
// use sm_common::InstanceExpanderCtx;
// use zisk_common::InstObserver;
// use zisk_core::{InstContext, ZiskInst, ZiskRom};
// use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};
// use ziskemu::EmuTrace;

// use crate::RomSM;

// pub struct RomExpander<F: PrimeField> {
//     wcm: Arc<WitnessManager<F>>,

//     zisk_rom: Arc<ZiskRom>,
// }

// impl<F: PrimeField> RomExpander<F> {
//     pub fn new(wcm: Arc<WitnessManager<F>>, zisk_rom: Arc<ZiskRom>) -> Self {
//         RomExpander { wcm, zisk_rom }
//     }
// }

// impl<F: PrimeField> Expander<F> for RomExpander<F> {
//     fn expand(
//         &self,
//         iectx: &mut InstanceExpanderCtx<F>,
//         min_traces: Arc<Vec<EmuTrace>>,
//     ) -> Result<(), Box<dyn std::error::Error + Send>> {
//         Ok(())
//     }
// }

// impl<F: PrimeField> InstObserver for RomExpander<F> {
//     #[inline(always)]
//     fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) -> bool {
//         false
//     }
// }
