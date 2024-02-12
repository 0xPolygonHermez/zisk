#[allow(dead_code)]
extern "C" {
    #[link_name = "\u{1}_Z13generateProofPcS_S_S_S_"]
    fn generateProof(
        constPols_: *mut ::std::os::raw::c_char,
        constTree_: *mut ::std::os::raw::c_char,
        starkInfoFile_: *mut ::std::os::raw::c_char,
        commitPols_: *mut ::std::os::raw::c_char,
        verkey_: *mut ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int;
}
