use std::{os::raw::c_void, path::Path};

use log::info;

use proofman_starks_lib_c::{const_pols_new_c, expressions_bin_new_c, setup_ctx_new_c, stark_info_new_c};

use crate::GlobalInfo;

/// Air instance context for managing air instances (traces)
#[derive(Debug)]
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

    pub fn new(proving_key_path: &Path, global_info: &GlobalInfo, airgroup_id: usize, air_id: usize) -> Self {
        let air_setup_folder = proving_key_path.join(global_info.get_air_setup_path(airgroup_id, air_id));

        // Check path exists and is a folder
        if !air_setup_folder.exists() {
            panic!("Setup AIR folder not found at path: {:?}", air_setup_folder);
        }
        if !air_setup_folder.is_dir() {
            panic!("Setup AIR path is not a folder: {:?}", air_setup_folder);
        }

        let base_filename_path =
            air_setup_folder.join(global_info.get_air_name(airgroup_id, air_id)).display().to_string();

        info!("{}   : ··· Loading setup for AIR [{}:{}]: {:?}", Self::MY_NAME, airgroup_id, air_id, air_setup_folder);

        let stark_info_path = base_filename_path.clone() + ".starkinfo.json";
        let expressions_bin_path = base_filename_path.clone() + ".bin";
        let const_pols_path = base_filename_path.clone() + ".const";

        let p_stark_info = stark_info_new_c(stark_info_path.as_str());
        let p_expressions_bin = expressions_bin_new_c(expressions_bin_path.as_str());
        let p_const_pols = const_pols_new_c(const_pols_path.as_str(), p_stark_info);

        let p_setup = setup_ctx_new_c(p_stark_info, p_expressions_bin, p_const_pols);

        Self { air_id, airgroup_id, p_setup, p_stark_info }
    }
}
