use std::os::raw::c_void;
use std::path::PathBuf;
use std::sync::RwLock;

use proofman_starks_lib_c::get_map_totaln_c;
use proofman_starks_lib_c::{
    get_const_tree_size_c, get_const_size_c, prover_helpers_new_c, expressions_bin_new_c, stark_info_new_c,
    load_const_tree_c, load_const_pols_c, calculate_const_tree_c, stark_info_free_c, expressions_bin_free_c,
    prover_helpers_free_c,
};
use proofman_util::create_buffer_fast_u8;

use crate::GlobalInfo;
use crate::ProofType;
use crate::StarkInfo;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct SetupC {
    pub p_stark_info: *mut c_void,
    pub p_expressions_bin: *mut c_void,
    pub p_prover_helpers: *mut c_void,
}

unsafe impl Send for SetupC {}
unsafe impl Sync for SetupC {}

impl From<&SetupC> for *mut c_void {
    fn from(setup: &SetupC) -> *mut c_void {
        setup as *const SetupC as *mut c_void
    }
}

#[derive(Debug)]
pub struct Pols {
    pub values: RwLock<Vec<u8>>,
}

impl Default for Pols {
    fn default() -> Self {
        Self { values: RwLock::new(Vec::new()) }
    }
}

/// Air instance context for managing air instances (traces)
#[derive(Debug)]
#[allow(dead_code)]
pub struct Setup {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub p_setup: SetupC,
    pub stark_info: StarkInfo,
    pub const_pols: Pols,
    pub const_tree: Pols,
    pub prover_buffer_size: u64,
}

impl Setup {
    const MY_NAME: &'static str = "Setup";

    pub fn new(global_info: &GlobalInfo, airgroup_id: usize, air_id: usize, setup_type: &ProofType) -> Self {
        let setup_path = match setup_type {
            ProofType::VadcopFinal => global_info.get_setup_path("vadcop_final"),
            ProofType::RecursiveF => global_info.get_setup_path("recursivef"),
            _ => global_info.get_air_setup_path(airgroup_id, air_id, setup_type),
        };

        let stark_info_path = setup_path.display().to_string() + ".starkinfo.json";
        let expressions_bin_path = setup_path.display().to_string() + ".bin";

        let (stark_info, p_stark_info, p_expressions_bin, p_prover_helpers, prover_buffer_size) =
            if setup_type == &ProofType::Compressor && !global_info.get_air_has_compressor(airgroup_id, air_id) {
                // If the condition is met, use None for each pointer
                (StarkInfo::default(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), 0)
            } else {
                // Otherwise, initialize the pointers with their respective values
                let stark_info_json = std::fs::read_to_string(&stark_info_path)
                    .unwrap_or_else(|_| panic!("Failed to read file {}", &stark_info_path));
                let stark_info = StarkInfo::from_json(&stark_info_json);
                let p_stark_info = stark_info_new_c(stark_info_path.as_str(), false);
                let recursive = &ProofType::Basic != setup_type;
                let prover_buffer_size = get_map_totaln_c(p_stark_info, recursive);
                let expressions_bin = expressions_bin_new_c(expressions_bin_path.as_str(), false, false);
                let prover_helpers = prover_helpers_new_c(p_stark_info, recursive);

                (stark_info, p_stark_info, expressions_bin, prover_helpers, prover_buffer_size)
            };

        Self {
            air_id,
            airgroup_id,
            stark_info,
            p_setup: SetupC { p_stark_info, p_expressions_bin, p_prover_helpers },
            const_pols: Pols::default(),
            const_tree: Pols::default(),
            prover_buffer_size,
        }
    }

    pub fn free(&self) {
        stark_info_free_c(self.p_setup.p_stark_info);
        expressions_bin_free_c(self.p_setup.p_expressions_bin);
        prover_helpers_free_c(self.p_setup.p_prover_helpers);
    }

    pub fn load_const_pols(&self, global_info: &GlobalInfo, setup_type: &ProofType) {
        let setup_path = match setup_type {
            ProofType::VadcopFinal => global_info.get_setup_path("vadcop_final"),
            ProofType::RecursiveF => global_info.get_setup_path("recursivef"),
            _ => global_info.get_air_setup_path(self.airgroup_id, self.air_id, setup_type),
        };

        let air_name = &global_info.airs[self.airgroup_id][self.air_id].name;
        log::debug!("{}   : ··· Loading const pols for AIR {} of type {:?}", Self::MY_NAME, air_name, setup_type);

        let const_pols_path = setup_path.display().to_string() + ".const";

        let p_stark_info = self.p_setup.p_stark_info;

        let const_size = get_const_size_c(p_stark_info) as usize;
        let const_pols = create_buffer_fast_u8(const_size);

        load_const_pols_c(const_pols.as_ptr() as *mut u8, const_pols_path.as_str(), const_size as u64);
        *self.const_pols.values.write().unwrap() = const_pols;
    }

    pub fn load_const_pols_tree(&self, global_info: &GlobalInfo, setup_type: &ProofType, save_file: bool) {
        let setup_path = match setup_type {
            ProofType::VadcopFinal => global_info.get_setup_path("vadcop_final"),
            ProofType::RecursiveF => global_info.get_setup_path("recursivef"),
            _ => global_info.get_air_setup_path(self.airgroup_id, self.air_id, setup_type),
        };

        let air_name = &global_info.airs[self.airgroup_id][self.air_id].name;
        log::debug!("{}   : ··· Loading const tree for AIR {} of type {:?}", Self::MY_NAME, air_name, setup_type);

        let const_pols_tree_path = setup_path.display().to_string() + ".consttree";

        let p_stark_info = self.p_setup.p_stark_info;

        let const_tree_size = get_const_tree_size_c(p_stark_info) as usize;

        let const_tree = create_buffer_fast_u8(const_tree_size);

        if PathBuf::from(&const_pols_tree_path).exists() {
            load_const_tree_c(const_tree.as_ptr() as *mut u8, const_pols_tree_path.as_str(), const_tree_size as u64);
        } else {
            let const_pols = self.const_pols.values.read().unwrap();
            let tree_filename = if save_file { const_pols_tree_path.as_str() } else { "" };
            calculate_const_tree_c(
                p_stark_info,
                (*const_pols).as_ptr() as *mut u8,
                const_tree.as_ptr() as *mut u8,
                tree_filename,
            );
        };
        *self.const_tree.values.write().unwrap() = const_tree;
    }

    pub fn get_const_ptr(&self) -> *mut u8 {
        let guard = &self.const_pols.values.read().unwrap();
        guard.as_ptr() as *mut u8
    }

    pub fn get_const_tree_ptr(&self) -> *mut u8 {
        let guard = &self.const_tree.values.read().unwrap();
        guard.as_ptr() as *mut u8
    }
}
