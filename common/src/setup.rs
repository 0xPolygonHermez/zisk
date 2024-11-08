use std::mem::MaybeUninit;
use std::os::raw::c_void;
use std::path::PathBuf;
use std::sync::RwLock;

use proofman_starks_lib_c::{
    get_const_tree_size_c, get_const_size_c, prover_helpers_new_c, expressions_bin_new_c, stark_info_new_c,
    load_const_tree_c, load_const_pols_c, calculate_const_tree_c, stark_info_free_c, expressions_bin_free_c,
    prover_helpers_free_c,
};

use crate::GlobalInfo;
use crate::ProofType;

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
pub struct Pols<F> {
    pub values: RwLock<Vec<MaybeUninit<F>>>,
}

impl<F> Default for Pols<F> {
    fn default() -> Self {
        Self { values: RwLock::new(Vec::new()) }
    }
}

/// Air instance context for managing air instances (traces)
#[derive(Debug)]
#[allow(dead_code)]
pub struct Setup<F> {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub p_setup: SetupC,
    pub const_pols: Pols<F>,
    pub const_tree: Pols<F>,
}

impl<F> Setup<F> {
    const MY_NAME: &'static str = "Setup";

    pub fn new(global_info: &GlobalInfo, airgroup_id: usize, air_id: usize, setup_type: &ProofType) -> Self {
        let setup_path = match setup_type {
            ProofType::Final => global_info.get_final_setup_path(),
            _ => global_info.get_air_setup_path(airgroup_id, air_id, setup_type),
        };

        let stark_info_path = setup_path.display().to_string() + ".starkinfo.json";
        let expressions_bin_path = setup_path.display().to_string() + ".bin";

        let (p_stark_info, p_expressions_bin, p_prover_helpers) =
            if setup_type == &ProofType::Compressor && !global_info.get_air_has_compressor(airgroup_id, air_id) {
                // If the condition is met, use None for each pointer
                (std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
            } else {
                // Otherwise, initialize the pointers with their respective values
                let stark_info = stark_info_new_c(stark_info_path.as_str());
                let expressions_bin = expressions_bin_new_c(expressions_bin_path.as_str(), false);
                let prover_helpers = prover_helpers_new_c(stark_info);

                (stark_info, expressions_bin, prover_helpers)
            };

        Self {
            air_id,
            airgroup_id,
            p_setup: SetupC { p_stark_info, p_expressions_bin, p_prover_helpers },
            const_pols: Pols::default(),
            const_tree: Pols::default(),
        }
    }

    pub fn free(&self) {
        stark_info_free_c(self.p_setup.p_stark_info);
        expressions_bin_free_c(self.p_setup.p_expressions_bin);
        prover_helpers_free_c(self.p_setup.p_prover_helpers);
    }

    pub fn load_const_pols(&self, global_info: &GlobalInfo, setup_type: &ProofType) {
        let setup_path = match setup_type {
            ProofType::Final => global_info.get_final_setup_path(),
            _ => global_info.get_air_setup_path(self.airgroup_id, self.air_id, setup_type),
        };

        let air_name = &global_info.airs[self.airgroup_id][self.air_id].name;
        log::debug!("{}   : ··· Loading const pols for AIR {} of type {:?}", Self::MY_NAME, air_name, setup_type);

        let const_pols_path = setup_path.display().to_string() + ".const";

        let p_stark_info = self.p_setup.p_stark_info;

        let const_size = get_const_size_c(p_stark_info) as usize;
        let const_pols: Vec<MaybeUninit<F>> = Vec::with_capacity(const_size);

        let p_const_pols_address = const_pols.as_ptr() as *mut c_void;
        load_const_pols_c(p_const_pols_address, const_pols_path.as_str(), const_size as u64);
        *self.const_pols.values.write().unwrap() = const_pols;
    }

    pub fn load_const_pols_tree(&self, global_info: &GlobalInfo, setup_type: &ProofType, save_file: bool) {
        let setup_path = match setup_type {
            ProofType::Final => global_info.get_final_setup_path(),
            _ => global_info.get_air_setup_path(self.airgroup_id, self.air_id, setup_type),
        };

        let air_name = &global_info.airs[self.airgroup_id][self.air_id].name;
        log::debug!("{}   : ··· Loading const tree for AIR {}", Self::MY_NAME, air_name);

        let const_pols_tree_path = setup_path.display().to_string() + ".consttree";

        let p_stark_info = self.p_setup.p_stark_info;

        let const_tree_size = get_const_tree_size_c(p_stark_info) as usize;
        let const_tree: Vec<MaybeUninit<F>> = Vec::with_capacity(const_tree_size);

        let p_const_tree_address = const_tree.as_ptr() as *mut c_void;
        if PathBuf::from(&const_pols_tree_path).exists() {
            load_const_tree_c(p_const_tree_address, const_pols_tree_path.as_str(), const_tree_size as u64);
        } else {
            let const_pols = self.const_pols.values.read().unwrap();
            let p_const_pols_address = (*const_pols).as_ptr() as *mut c_void;
            let tree_filename = if save_file { const_pols_tree_path.as_str() } else { "" };
            calculate_const_tree_c(p_stark_info, p_const_pols_address, p_const_tree_address, tree_filename);
        };
        *self.const_tree.values.write().unwrap() = const_tree;
    }
}
