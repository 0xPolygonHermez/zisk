use std::cell::OnceCell;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use crate::GlobalInfo;
use crate::Setup;
use crate::WitnessPilout;

#[derive(Debug)]
pub struct SetupRepository {
    setups: HashMap<(usize, usize), OnceCell<Setup>>,
    setup_airs: Vec<Vec<usize>>,
}

unsafe impl Send for SetupRepository {}
unsafe impl Sync for SetupRepository {}

impl SetupRepository {
    pub fn new(pilout: WitnessPilout) -> Self {
        let mut setups = HashMap::new();

        // Initialize Hashmao for each airgroup_id, air_id
        let setup_airs = pilout
            .air_groups()
            .iter()
            .enumerate()
            .map(|(airgroup_id, air_group)| {
                let mut air_group_setups = Vec::new();
                air_group
                    .airs()
                    .iter()
                    .enumerate()
                    .for_each(|(air_id, _)| {
                        setups.insert((airgroup_id, air_id), OnceCell::new());
                        air_group_setups.push(air_id);
                    });
                air_group_setups
            })
            .collect::<Vec<Vec<usize>>>();

        Self { setups, setup_airs }
    }
}
/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
pub struct SetupCtx {
    global_info: GlobalInfo,
    proving_key_path: PathBuf,

    setup_repository: SetupRepository,
}

impl SetupCtx {
    pub fn new(pilout: WitnessPilout, proving_key_path: &Path) -> Self {
        SetupCtx {
            global_info: GlobalInfo::new(proving_key_path),
            proving_key_path: proving_key_path.to_path_buf(),
            setup_repository: SetupRepository::new(pilout),
        }
    }

    pub fn get_setup(&self, airgroup_id: usize, air_id: usize) -> Result<&Setup, String> {
        let setup = self
            .setup_repository
            .setups
            .get(&(airgroup_id, air_id))
            .ok_or_else(|| format!("Setup not found for airgroup_id: {}, Air_id: {}", airgroup_id, air_id))?;

        if setup.get().is_some() {
            return Ok(setup.get().unwrap());
        } else {
            let _setup = Setup::new(&self.proving_key_path, &self.global_info, airgroup_id, air_id);
            setup.set(_setup).unwrap();
            return Ok(setup.get().unwrap());
        }
    }

    pub fn get_setup_airs(&self) -> Vec<Vec<usize>> {
        self.setup_repository.setup_airs.clone()
    }
}
