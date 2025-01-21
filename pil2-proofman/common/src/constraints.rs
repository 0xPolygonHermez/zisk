use std::sync::Arc;

use proofman_starks_lib_c::{
    get_n_constraints_c, get_constraints_lines_sizes_c, get_constraints_lines_c, get_n_global_constraints_c,
    get_global_constraints_lines_sizes_c, get_global_constraints_lines_c,
};

use crate::SetupCtx;

#[derive(Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct ConstraintRowInfo {
    pub row: u64,
    pub dim: u64,
    pub value: [u64; 3usize],
}

#[derive(Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct ConstraintInfo {
    pub id: u64,
    pub stage: u64,
    pub im_pol: bool,
    pub n_rows: u64,
    pub skip: bool,
    pub rows: [ConstraintRowInfo; 10usize],
}

#[derive(Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct GlobalConstraintInfo {
    pub id: u64,
    pub dim: u64,
    pub valid: bool,
    pub skip: bool,
    pub value: [u64; 3usize],
}

pub fn get_constraints_lines_str(sctx: Arc<SetupCtx>, airgroup_id: usize, air_id: usize) -> Vec<String> {
    let setup = sctx.get_setup(airgroup_id, air_id);

    let p_setup = (&setup.p_setup).into();
    let n_constraints = get_n_constraints_c(p_setup);

    let mut constraints_sizes = vec![0u64; n_constraints as usize];

    get_constraints_lines_sizes_c(p_setup, constraints_sizes.as_mut_ptr());

    let mut constraints_lines = vec![Vec::new(); n_constraints as usize];
    for i in 0..n_constraints as usize {
        constraints_lines[i] = vec![0u8; constraints_sizes[i] as usize];
    }

    get_constraints_lines_c(
        p_setup,
        constraints_lines.iter_mut().map(|v| v.as_mut_ptr()).collect::<Vec<_>>().as_mut_ptr(),
    );

    let mut constraints_lines_str = Vec::new();
    for constraint_line in constraints_lines {
        constraints_lines_str.push(std::str::from_utf8(&constraint_line).unwrap().to_string());
    }

    constraints_lines_str
}

pub fn get_global_constraints_lines_str(sctx: Arc<SetupCtx>) -> Vec<String> {
    let n_global_constraints = get_n_global_constraints_c(sctx.get_global_bin());

    let mut global_constraints_sizes = vec![0u64; n_global_constraints as usize];

    get_global_constraints_lines_sizes_c(sctx.get_global_bin(), global_constraints_sizes.as_mut_ptr());

    let mut global_constraints_lines = vec![Vec::new(); n_global_constraints as usize];
    for i in 0..n_global_constraints as usize {
        global_constraints_lines[i] = vec![0u8; global_constraints_sizes[i] as usize];
    }

    get_global_constraints_lines_c(
        sctx.get_global_bin(),
        global_constraints_lines.iter_mut().map(|v| v.as_mut_ptr()).collect::<Vec<_>>().as_mut_ptr(),
    );

    let mut global_constraints_lines_str = Vec::new();
    for global_constraint_line in global_constraints_lines {
        global_constraints_lines_str.push(std::str::from_utf8(&global_constraint_line).unwrap().to_string());
    }

    global_constraints_lines_str
}
