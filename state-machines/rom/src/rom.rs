use std::{path::PathBuf, sync::Arc};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use zisk_pil::{ROM_AIRGROUP_ID, ROM_L_AIR_IDS, ROM_M_AIR_IDS, ROM_S_AIR_IDS};

pub struct RomSM {}

impl RomSM {
    pub fn new<F>(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let rom_sm = Self {};
        let rom_sm = Arc::new(rom_sm);

        let rom_air_ids = &[ROM_S_AIR_IDS[0], ROM_M_AIR_IDS[0], ROM_L_AIR_IDS[0]];
        wcm.register_component(rom_sm.clone(), Some(ROM_AIRGROUP_ID), Some(rom_air_ids));

        rom_sm
    }

    pub fn prove<F: Field>(&self, _rom_path: PathBuf) {
        // FIXME! Implement proof logic
        unimplemented!();
    }
}

impl<F> WitnessComponent<F> for RomSM {}
