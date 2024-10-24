use std::{path::PathBuf, sync::Arc};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use zisk_pil::{ROM_AIRGROUP_ID, ROM_L_AIR_IDS, ROM_M_AIR_IDS, ROM_S_AIR_IDS};

pub struct RomSM<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: Field> RomSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let rom_sm = Self { _phantom: std::marker::PhantomData };
        let rom_sm = Arc::new(rom_sm);

        let rom_air_ids = &[ROM_S_AIR_IDS[0], ROM_M_AIR_IDS[0], ROM_L_AIR_IDS[0]];
        wcm.register_component(rom_sm.clone(), Some(ROM_AIRGROUP_ID), Some(rom_air_ids));

        rom_sm
    }

    pub fn prove(&self, _rom_path: PathBuf) -> Result<(), Box<dyn std::error::Error + Send>> {
        // FIXME! Implement proof logic
        println!("Proving ROM");

        Ok(())
    }
}

impl<F: Field> WitnessComponent<F> for RomSM<F> {}
