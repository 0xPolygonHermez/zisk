use std::{os::raw::c_void, path::Path};

use log::trace;

use starks_lib_c::{stark_info_new_c, chelpers_new_c, const_pols_new_c, chelpers_steps_new_c};

use crate::GlobalInfo;

/// Air instance context for managing air instances (traces)
#[allow(dead_code)]
pub struct Setup {
    pub air_group_id: usize,
    pub air_id: usize,
    pub p_steps: *mut c_void,
}

impl Setup {
    const MY_NAME: &'static str = "Setup";

    pub fn new(proving_key_path: &Path, air_group_id: usize, air_id: usize) -> Self {
        let global_info = GlobalInfo::from_file(&proving_key_path.join("pilout.globalInfo.json"));

        let air_setup_folder = proving_key_path.join(global_info.get_air_setup_path(air_group_id, air_id));
        trace!("{}   : ··· Setup AIR folder: {:?}", Self::MY_NAME, air_setup_folder);

        // Check path exists and is a folder
        if !air_setup_folder.exists() {
            panic!("Setup AIR folder not found at path: {:?}", air_setup_folder);
        }
        if !air_setup_folder.is_dir() {
            panic!("Setup AIR path is not a folder: {:?}", air_setup_folder);
        }

        let base_filename_path =
            air_setup_folder.join(global_info.get_air_name(air_group_id, air_id)).display().to_string();

        let stark_info_path = base_filename_path.clone() + ".starkinfo.json";
        let chelpers_path = base_filename_path.clone() + ".bin";

        let p_starkinfo = stark_info_new_c(&stark_info_path);

        let p_chelpers = chelpers_new_c(&chelpers_path);

        let const_pols_filename = base_filename_path.clone() + ".const";
        let p_constpols = const_pols_new_c(p_starkinfo, const_pols_filename.as_str());

        let p_steps = chelpers_steps_new_c(p_starkinfo, p_chelpers, p_constpols);

        Self { air_id, air_group_id, p_steps }
    }
}
