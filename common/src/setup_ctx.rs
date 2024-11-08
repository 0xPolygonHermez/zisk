use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;

use log::info;
use proofman_starks_lib_c::expressions_bin_new_c;
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};

use crate::GlobalInfo;
use crate::Setup;
use crate::ProofType;

pub struct SetupsVadcop<F> {
    pub sctx: Arc<SetupCtx<F>>,
    pub sctx_compressor: Option<Arc<SetupCtx<F>>>,
    pub sctx_recursive1: Option<Arc<SetupCtx<F>>>,
    pub sctx_recursive2: Option<Arc<SetupCtx<F>>>,
    pub sctx_final: Option<Arc<SetupCtx<F>>>,
}

impl<F> SetupsVadcop<F> {
    pub fn new(global_info: &GlobalInfo, aggregation: bool) -> Self {
        info!("Initializing setups");
        timer_start_debug!(INITIALIZING_SETUP);
        let sctx: SetupCtx<F> = SetupCtx::new(global_info, &ProofType::Basic);
        timer_stop_and_log_debug!(INITIALIZING_SETUP);
        if aggregation {
            timer_start_debug!(INITIALIZING_SETUP_AGGREGATION);
            info!("Initializing setups aggregation");

            timer_start_debug!(INITIALIZING_SETUP_COMPRESSOR);
            info!(" ··· Initializing setups compressor");
            let sctx_compressor: SetupCtx<F> = SetupCtx::new(global_info, &ProofType::Compressor);
            timer_stop_and_log_debug!(INITIALIZING_SETUP_COMPRESSOR);

            timer_start_debug!(INITIALIZING_SETUP_RECURSIVE1);
            info!(" ··· Initializing setups recursive1");
            let sctx_recursive1: SetupCtx<F> = SetupCtx::new(global_info, &ProofType::Recursive1);
            timer_stop_and_log_debug!(INITIALIZING_SETUP_RECURSIVE1);

            timer_start_debug!(INITIALIZING_SETUP_RECURSIVE2);
            info!(" ··· Initializing setups recursive2");
            let sctx_recursive2: SetupCtx<F> = SetupCtx::new(global_info, &ProofType::Recursive2);
            timer_stop_and_log_debug!(INITIALIZING_SETUP_RECURSIVE2);

            timer_start_debug!(INITIALIZING_SETUP_FINAL);
            info!(" ··· Initializing setups final");
            let sctx_final: SetupCtx<F> = SetupCtx::new(global_info, &ProofType::Final);
            timer_stop_and_log_debug!(INITIALIZING_SETUP_FINAL);
            timer_stop_and_log_debug!(INITIALIZING_SETUP_AGGREGATION);
            SetupsVadcop {
                sctx: Arc::new(sctx),
                sctx_compressor: Some(Arc::new(sctx_compressor)),
                sctx_recursive1: Some(Arc::new(sctx_recursive1)),
                sctx_recursive2: Some(Arc::new(sctx_recursive2)),
                sctx_final: Some(Arc::new(sctx_final)),
            }
        } else {
            SetupsVadcop {
                sctx: Arc::new(sctx),
                sctx_compressor: None,
                sctx_recursive1: None,
                sctx_recursive2: None,
                sctx_final: None,
            }
        }
    }
}

#[derive(Debug)]
pub struct SetupRepository<F> {
    setups: HashMap<(usize, usize), Setup<F>>,
    global_bin: Option<*mut c_void>,
}

unsafe impl<F> Send for SetupRepository<F> {}
unsafe impl<F> Sync for SetupRepository<F> {}

impl<F> SetupRepository<F> {
    pub fn new(global_info: &GlobalInfo, setup_type: &ProofType) -> Self {
        timer_start_debug!(INITIALIZE_SETUPS);
        let mut setups = HashMap::new();

        let global_bin = match setup_type == &ProofType::Basic {
            true => {
                let global_bin_path =
                    &global_info.get_proving_key_path().join("pilout.globalConstraints.bin").display().to_string();
                Some(expressions_bin_new_c(global_bin_path.as_str(), true))
            }
            false => None,
        };

        // Initialize Hashmap for each airgroup_id, air_id
        if setup_type != &ProofType::Final {
            for (airgroup_id, air_group) in global_info.airs.iter().enumerate() {
                for (air_id, _) in air_group.iter().enumerate() {
                    setups.insert((airgroup_id, air_id), Setup::new(global_info, airgroup_id, air_id, setup_type));
                }
            }
        } else {
            setups.insert((0, 0), Setup::new(global_info, 0, 0, setup_type));
        }

        timer_stop_and_log_debug!(INITIALIZE_SETUPS);

        Self { setups, global_bin }
    }

    pub fn free(&self) {
        // TODO
    }
}
/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
pub struct SetupCtx<F> {
    global_info: GlobalInfo,
    setup_repository: SetupRepository<F>,
    setup_type: ProofType,
}

impl<F> SetupCtx<F> {
    pub fn new(global_info: &GlobalInfo, setup_type: &ProofType) -> Self {
        SetupCtx {
            setup_repository: SetupRepository::new(global_info, setup_type),
            global_info: global_info.clone(),
            setup_type: setup_type.clone(),
        }
    }

    pub fn get_setup(&self, airgroup_id: usize, air_id: usize) -> &Setup<F> {
        match self.setup_repository.setups.get(&(airgroup_id, air_id)) {
            Some(setup) => setup,
            None => {
                // Handle the error case as needed
                log::error!("Setup not found for airgroup_id: {}, air_id: {}", airgroup_id, air_id);
                // You might want to return a default value or panic
                panic!("Setup not found"); // or return a default value if applicable
            }
        }
    }

    pub fn get_global_bin(&self) -> *mut c_void {
        self.setup_repository.global_bin.unwrap()
    }
}
