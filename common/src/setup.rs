use std::os::raw::c_void;
use std::path::PathBuf;

use proofman_starks_lib_c::{const_pols_new_c, const_pols_with_tree_new_c, expressions_bin_new_c, stark_info_new_c};

use crate::GlobalInfo;
use crate::ProofType;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct SetupC {
    pub p_stark_info: *mut c_void,
    pub p_expressions_bin: *mut c_void,
    pub p_const_pols: *mut c_void,
}

unsafe impl Send for SetupC {}
unsafe impl Sync for SetupC {}

impl From<&SetupC> for *mut c_void {
    fn from(setup: &SetupC) -> *mut c_void {
        setup as *const SetupC as *mut c_void
    }
}

/// Air instance context for managing air instances (traces)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Setup {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub p_setup: SetupC,
}

impl Setup {
    const MY_NAME: &'static str = "Setup";

    pub fn new(global_info: &GlobalInfo, airgroup_id: usize, air_id: usize, setup_type: &ProofType) -> Self {
        let setup_path = match setup_type {
            ProofType::Final => global_info.get_final_setup_path(),
            _ => global_info.get_air_setup_path(airgroup_id, air_id, setup_type),
        };

        let stark_info_path = setup_path.display().to_string() + ".starkinfo.json";
        let expressions_bin_path = setup_path.display().to_string() + ".bin";
        let const_pols_path = setup_path.display().to_string() + ".const";
        let const_pols_tree_path = setup_path.display().to_string() + ".consttree";

        let p_stark_info = stark_info_new_c(stark_info_path.as_str());
        let p_expressions_bin = expressions_bin_new_c(expressions_bin_path.as_str(), false);

        let p_const_pols = match PathBuf::from(&const_pols_tree_path).exists() {
            true => const_pols_with_tree_new_c(const_pols_path.as_str(), const_pols_tree_path.as_str(), p_stark_info),
            false => const_pols_new_c(const_pols_path.as_str(), p_stark_info, true),
        };

        Self { air_id, airgroup_id, p_setup: SetupC { p_stark_info, p_expressions_bin, p_const_pols } }
    }

    pub fn new_partial(global_info: &GlobalInfo, airgroup_id: usize, air_id: usize, setup_type: &ProofType) -> Self {
        let setup_path = global_info.get_air_setup_path(airgroup_id, air_id, setup_type);

        let air_name = &global_info.airs[airgroup_id][air_id].name;
        log::debug!("{}   : ··· Loading setup for AIR {}", Self::MY_NAME, air_name);

        let stark_info_path = setup_path.display().to_string() + ".starkinfo.json";
        let expressions_bin_path = setup_path.display().to_string() + ".bin";
        let p_stark_info = stark_info_new_c(stark_info_path.as_str());
        let p_expressions_bin = expressions_bin_new_c(expressions_bin_path.as_str(), false);

        Self {
            air_id,
            airgroup_id,
            p_setup: SetupC { p_stark_info, p_expressions_bin, p_const_pols: std::ptr::null_mut() },
        }
    }

    pub fn load_const_pols(&mut self, global_info: &GlobalInfo, setup_type: &ProofType) {
        if !self.p_setup.p_const_pols.is_null() {
            return;
        }
        assert!(!self.p_setup.p_stark_info.is_null());
        assert!(!self.p_setup.p_expressions_bin.is_null());

        let setup_path = global_info.get_air_setup_path(self.airgroup_id, self.air_id, setup_type);

        let air_name = &global_info.airs[self.airgroup_id][self.air_id].name;
        log::debug!("{}   : ··· Loading const pols for AIR {}", Self::MY_NAME, air_name);

        let const_pols_path = setup_path.display().to_string() + ".const";
        let const_pols_tree_path = setup_path.display().to_string() + ".consttree";

        let p_const_pols = match PathBuf::from(&const_pols_tree_path).exists() {
            true => const_pols_with_tree_new_c(
                const_pols_path.as_str(),
                const_pols_tree_path.as_str(),
                self.p_setup.p_stark_info,
            ),
            false => const_pols_new_c(const_pols_path.as_str(), self.p_setup.p_stark_info, true),
        };

        self.p_setup.p_const_pols = p_const_pols;
    }
}
