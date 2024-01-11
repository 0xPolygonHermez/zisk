#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!("../bindings.rs");

use std::ffi::CString;

pub fn generate_proof(
    const_pols: String,
    const_tree: String,
    stark_info_file: String,
    commit_pols: String,
    verkey: String,
) {
    unsafe {
        let const_pols = CString::new(const_pols).unwrap();
        let const_tree = CString::new(const_tree).unwrap();
        let stark_info_file = CString::new(stark_info_file).unwrap();
        let commit_pols = CString::new(commit_pols).unwrap();
        let verkey = CString::new(verkey).unwrap();

        generateProof(
            const_pols.as_ptr() as *mut std::os::raw::c_char,
            const_tree.as_ptr() as *mut std::os::raw::c_char,
            stark_info_file.as_ptr() as *mut std::os::raw::c_char,
            commit_pols.as_ptr() as *mut std::os::raw::c_char,
            verkey.as_ptr() as *mut std::os::raw::c_char,
        );
    }
}
