use std::path::Path;
use std::sync::Arc;

use crate::Setup;
use crate::WitnessPilout;

pub struct SetupRepository {
    pub setups: Vec<Setup>,
}

impl SetupRepository {
    pub fn new(pilout: WitnessPilout, proving_key_path: &Path) -> Self {
        let setups = pilout
            .air_groups()
            .iter()
            .enumerate()
            .flat_map(|(airgroup_id, air_group)| {
                air_group
                    .airs()
                    .iter()
                    .enumerate()
                    .map(move |(air_id, _)| Setup::new(proving_key_path, airgroup_id, air_id))
            })
            .collect::<Vec<Setup>>();

        Self { setups }
    }

    pub fn get_setup(&self, airgroup_id: usize, air_id: usize) -> Result<&Setup, String> {
        for setup in self.setups.iter() {
            if setup.airgroup_id == airgroup_id && setup.air_id == air_id {
                return Ok(setup);
            }
        }

        Err(format!("Setup not found for airgroup_id: {}, Air_id: {}", airgroup_id, air_id))
    }
}
/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
pub struct SetupCtx {
    pub setups: Arc<SetupRepository>,
}

impl SetupCtx {
    pub fn new(pilout: WitnessPilout, proving_key_path: &Path) -> Self {
        let setups = Arc::new(SetupRepository::new(pilout, proving_key_path));

        SetupCtx { setups }
    }
}
