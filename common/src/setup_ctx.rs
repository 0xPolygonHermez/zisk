use std::cell::OnceCell;
use std::collections::HashMap;

use crate::GlobalInfo;
use crate::Setup;
use crate::ProofType;

#[derive(Debug)]
pub struct SetupRepository {
    setups: HashMap<(usize, usize), OnceCell<Setup>>,
    setup_airs: Vec<Vec<usize>>,
}

unsafe impl Send for SetupRepository {}
unsafe impl Sync for SetupRepository {}

impl SetupRepository {
    pub fn new(global_info: &GlobalInfo, setup_type: &ProofType) -> Self {
        let mut setups = HashMap::new();

        // Initialize Hashmao for each airgroup_id, air_id
        let setup_airs = match setup_type != &ProofType::Final {
            true => global_info
                .airs
                .iter()
                .enumerate()
                .map(|(airgroup_id, air_group)| {
                    let mut air_group_setups = Vec::new();
                    air_group.iter().enumerate().for_each(|(air_id, _)| {
                        setups.insert((airgroup_id, air_id), OnceCell::new());
                        air_group_setups.push(air_id);
                    });
                    air_group_setups
                })
                .collect::<Vec<Vec<usize>>>(),
            false => {
                let mut air_group_setups: Vec<Vec<usize>> = Vec::new();
                setups.insert((0, 0), OnceCell::new());
                air_group_setups.push(vec![0]);
                air_group_setups
            }
        };

        Self { setups, setup_airs }
    }
}
/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
pub struct SetupCtx {
    global_info: GlobalInfo,
    setup_repository: SetupRepository,
    setup_type: ProofType,
}

impl SetupCtx {
    pub fn new(global_info: &GlobalInfo, setup_type: &ProofType) -> Self {
        SetupCtx {
            setup_repository: SetupRepository::new(global_info, setup_type),
            global_info: global_info.clone(),
            setup_type: setup_type.clone(),
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
            let _setup = Setup::new(&self.global_info, airgroup_id, air_id, &self.setup_type);
            setup.set(_setup).unwrap();
            return Ok(setup.get().unwrap());
        }
    }

    pub fn get_setup_airs(&self) -> Vec<Vec<usize>> {
        self.setup_repository.setup_airs.clone()
    }
}
