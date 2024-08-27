use std::{os::raw::c_void, path::Path};

use proofman_common::WitnessPilout;

use crate::Setup;

/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
pub struct SetupCtx {
    pub setups: Vec<Setup>,
}

impl SetupCtx {
    pub fn new(pilout: WitnessPilout, proving_key_path: &Path) -> Self {
        let setups = pilout
            .air_groups()
            .iter()
            .enumerate()
            .flat_map(|(air_group_id, air_group)| {
                air_group
                    .airs()
                    .iter()
                    .enumerate()
                    .map(move |(air_id, _)| Setup::new(proving_key_path, air_group_id, air_id))
            })
            .collect::<Vec<Setup>>();

        SetupCtx { setups }
    }

    pub fn get_setup(&self, air_group_id: usize, air_id: usize) -> Result<*mut c_void, String> {
        println!("Searching setup for Air_group_id: {}, Air_id: {}", air_group_id, air_id);

        for setup in &self.setups {
            if setup.air_group_id == air_group_id && setup.air_id == air_id {
                return Ok(setup.p_steps);
            }
        }

        Err(format!("Setup not found for Air_group_id: {}, Air_id: {}", air_group_id, air_id))
    }
}
