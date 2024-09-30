use std::os::raw::c_void;
use std::path::PathBuf;

use proofman_starks_lib_c::{
    const_pols_new_c, const_pols_with_tree_new_c, expressions_bin_new_c, setup_ctx_new_c, stark_info_new_c,
};

use crate::GlobalInfo;
use crate::ProofType;

/// Air instance context for managing air instances (traces)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Setup {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub p_setup: *mut c_void,
    pub p_stark_info: *mut c_void,
}

unsafe impl Send for Setup {}
unsafe impl Sync for Setup {}

impl Setup {
    const MY_NAME: &'static str = "Setup";

    pub fn new(global_info: &GlobalInfo, airgroup_id: usize, air_id: usize, setup_type: &ProofType) -> Self {
        let setup_folder = match setup_type {
            ProofType::Final => global_info.get_final_setup_path(),
            _ => global_info.get_air_setup_path(airgroup_id, air_id, setup_type),
        };

        // Check path exists and is a folder
        if !setup_folder.exists() {
            panic!("Setup AIR folder not found at path: {:?}", setup_folder);
        }
        if !setup_folder.is_dir() {
            panic!("Setup AIR path is not a folder: {:?}", setup_folder);
        }

        let base_filename_path = match setup_type {
            ProofType::Basic => setup_folder.join(global_info.get_air_name(airgroup_id, air_id)).display().to_string(),
            ProofType::Compressor => setup_folder.join("compressor").display().to_string(),
            ProofType::Recursive1 => setup_folder.join("recursive1").display().to_string(),
            ProofType::Recursive2 => setup_folder.join("recursive2").display().to_string(),
            ProofType::Final => setup_folder.join("final").display().to_string(),
        };
        let stark_info_path = base_filename_path.clone() + ".starkinfo.json";
        let expressions_bin_path = base_filename_path.clone() + ".bin";
        let const_pols_path = base_filename_path.clone() + ".const";

        let const_pols_tree_path = base_filename_path.clone() + ".consttree";

        println!("{}   : ··· Setup STARK info path: {:?}", Self::MY_NAME, stark_info_path); //rick
        let p_stark_info = stark_info_new_c(stark_info_path.as_str());
        let p_expressions_bin = expressions_bin_new_c(expressions_bin_path.as_str());

        let p_const_pols = match PathBuf::from(&const_pols_tree_path).exists() {
            true => const_pols_with_tree_new_c(const_pols_path.as_str(), const_pols_tree_path.as_str(), p_stark_info),
            false => const_pols_new_c(const_pols_path.as_str(), p_stark_info),
        };

        let p_setup = setup_ctx_new_c(p_stark_info, p_expressions_bin, p_const_pols);

        Self { air_id, airgroup_id, p_setup, p_stark_info }
    }
}
