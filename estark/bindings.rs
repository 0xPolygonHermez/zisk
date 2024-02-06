// Rust FFI declaration for the C function `int zkevm_prover_c(char* config_filename)`
#[allow(dead_code)]
extern "C" {
    #[link_name = "\u{1}_Z10zkevm_mainPcPv"]
    pub fn zkevm_main(
        config_filename: *mut ::std::os::raw::c_char,
        pAddress: *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;

    // Steps
    // ========================================================================================
    #[link_name = "\u{1}_Z15zkevm_steps_newv"]
    pub fn zkevm_steps_new() -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z16zkevm_steps_freePv"]
    pub fn zkevm_steps_free(pZkevmSteps: *mut ::std::os::raw::c_void);

    #[link_name = "\u{1}_Z14c12a_steps_newv"]
    pub fn c12a_steps_new() -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z15c12a_steps_freePv"]
    pub fn c12a_steps_free(pC12aSteps: *mut ::std::os::raw::c_void);

    #[link_name = "\u{1}_Z20recursive1_steps_newv"]
    pub fn recursive1_steps_new() -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z21recursive1_steps_freePv"]
    pub fn recursive1_steps_free(pRecursive1Steps: *mut ::std::os::raw::c_void);

    #[link_name = "\u{1}_Z20recursive2_steps_newv"]
    pub fn recursive2_steps_new() -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z21recursive2_steps_freePv"]
    pub fn recursive2_steps_free(Recursive2Steps: *mut ::std::os::raw::c_void);

    // FRIProof
    // ========================================================================================
    #[link_name = "\u{1}_Z13fri_proof_newmmmmm"]
    pub fn fri_proof_new(
        polN: u64,
        dim: u64,
        numTrees: u64,
        evalSize: u64,
        nPublics: u64,
    ) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z14fri_proof_freePv"]
    pub fn fri_proof_free(pFriProof: *mut ::std::os::raw::c_void);

    // Config
    // ========================================================================================
    #[link_name = "\u{1}_Z10config_newPc"]
    pub fn config_new(filename: *mut ::std::os::raw::c_char) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z11config_freePv"]
    pub fn config_free(pConfig: *mut ::std::os::raw::c_void);

    // Starks
    // ========================================================================================
    #[link_name = "\u{1}_Z10starks_newPvPcbS0_S0_S_"]
    pub fn starks_new(
        pConfig: *mut ::std::os::raw::c_void,
        constPols: *mut ::std::os::raw::c_char,
        mapConstPolsFile: bool,
        constantsTree: *mut ::std::os::raw::c_char,
        starkInfo: *mut ::std::os::raw::c_char,
        pAddress: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z16starks_gen_proofPvS_S_S_S_"]
    pub fn starks_genproof(
        pStarks: *mut ::std::os::raw::c_void,
        pFRIProof: *mut ::std::os::raw::c_void,
        pPublicInputs: *mut ::std::os::raw::c_void,
        pVerkey: *mut ::std::os::raw::c_void,
        pSteps: *mut ::std::os::raw::c_void,
    );

    #[link_name = "\u{1}_Z11starks_freePv"]
    pub fn starks_free(pStarks: *mut ::std::os::raw::c_void);

    // CommitPolsStarks
    // ========================================================================================
    #[link_name = "\u{1}_Z22commit_pols_starks_newPvmm"]
    pub fn commit_pols_starks_new(
        pAddress: *mut ::std::os::raw::c_void,
        degree: u64,
        nCommitedPols: u64,
    ) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z23commit_pols_starks_freePv"]
    pub fn commit_pols_starks_free(pCommitPolsStarks: *mut ::std::os::raw::c_void);

    // Circom
    // ========================================================================================
    #[link_name = "\u{1}_Z24circom_get_commited_polsPvPcS0_S_mm"]
    pub fn circom_get_commited_pols(
        pCommitPolsStarks: *mut ::std::os::raw::c_void,
        zkevmVerifier: *mut ::std::os::raw::c_char,
        execFile: *mut ::std::os::raw::c_char,
        zkin: *mut ::std::os::raw::c_void,
        N: u64,
        nCols: u64,
    );

    #[link_name = "\u{1}_Z35circom_recursive1_get_commited_polsPvPcS0_S_mm"]
    pub fn circom_recursive1_get_commited_pols(
        pCommitPolsStarks: *mut ::std::os::raw::c_void,
        zkevmVerifier: *mut ::std::os::raw::c_char,
        execFile: *mut ::std::os::raw::c_char,
        zkin: *mut ::std::os::raw::c_void,
        N: u64,
        nCols: u64,
    );

    // zkin
    // ========================================================================================
    #[link_name = "\u{1}_Z8zkin_newPvmS_mS_"]
    pub fn zkin_new(
        pFriProof: *mut ::std::os::raw::c_void,
        numPublicInputs: ::std::os::raw::c_ulong,
        pPublicInputs: *mut ::std::os::raw::c_void,
        numRootC: ::std::os::raw::c_ulong,
        pRootC: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
