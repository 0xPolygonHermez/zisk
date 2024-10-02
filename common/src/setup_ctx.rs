use std::cell::OnceCell;
use std::collections::HashMap;

use crate::GlobalInfo;
use crate::Setup;
use crate::ProofType;

#[derive(Debug)]
pub struct SetupRepository {
    // We store the setup in two stages: a partial setup in the first cell and a full setup in the second cell.
    // This allows for loading only the partial setup when constant polynomials are not needed, improving performance.
    // In C++, same SetupCtx structure is used to store either the partial or full setup for each instance.
    // A full setup can be loaded in one or two steps: partial first, then full (which includes constant polynomial data).
    // Since the setup is referenced immutably in the repository, we use OnceCell for both the partial and full setups.
    setups: HashMap<(usize, usize), (OnceCell<Setup>, OnceCell<Setup>)>, // (partial setup, full setup)
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
                        setups.insert((airgroup_id, air_id), (OnceCell::new(), OnceCell::new()));
                        air_group_setups.push(air_id);
                    });
                    air_group_setups
                })
                .collect::<Vec<Vec<usize>>>(),
            false => {
                let mut air_group_setups: Vec<Vec<usize>> = Vec::new();
                setups.insert((0, 0), (OnceCell::new(), OnceCell::new()));
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

        if let Some(setup_ref) = setup.1.get() {
            Ok(setup_ref)
        } else if let Some(setup_ref) = setup.0.get() {
            let mut new_setup = setup_ref.clone();
            new_setup.load_const_pols(&self.global_info, &self.setup_type);
            setup.1.set(new_setup).unwrap();

            Ok(setup.1.get().unwrap())
        } else {
            let new_setup = Setup::new(&self.global_info, airgroup_id, air_id, &self.setup_type);
            setup.1.set(new_setup).unwrap();

            Ok(setup.1.get().unwrap())
        }
    }

    pub fn get_partial_setup(&self, airgroup_id: usize, air_id: usize) -> Result<&Setup, String> {
        let setup = self
            .setup_repository
            .setups
            .get(&(airgroup_id, air_id))
            .ok_or_else(|| format!("Setup not found for airgroup_id: {}, Air_id: {}", airgroup_id, air_id))?;

        if setup.0.get().is_some() {
            Ok(setup.0.get().unwrap())
        } else if setup.1.get().is_some() {
            Ok(setup.1.get().unwrap())
        } else {
            let new_setup = Setup::new_partial(&self.global_info, airgroup_id, air_id, &self.setup_type);
            setup.0.set(new_setup).unwrap();

            Ok(setup.0.get().unwrap())
        }
    }

    pub fn get_setup_airs(&self) -> Vec<Vec<usize>> {
        self.setup_repository.setup_airs.clone()
    }
}
