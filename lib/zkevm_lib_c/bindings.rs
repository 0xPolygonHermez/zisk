// Rust FFI declaration for the C function `int zkevm_prover_c(char* config_filename)`
#[allow(dead_code)]
extern "C" {

    #[link_name = "\u{1}_Z10zkevm_mainPcPvS0_"]
    pub fn zkevm_main(
        pConfigFile: *mut ::std::os::raw::c_char,
        pAddress: *mut ::std::os::raw::c_void,
        pSecondarySMInputs: *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;

    #[link_name = "\u{1}_Z10save_proofPvS_mS_PcS0_"]
    pub fn save_proof(
        pStarkInfo: *mut ::std::os::raw::c_void,
        pFriProof: *mut ::std::os::raw::c_void,
        numPublicInputs: ::std::os::raw::c_ulong,
        pPublicInputs: *mut ::std::os::raw::c_void,
        publicsOutputFile: *mut ::std::os::raw::c_char,
        filePrefix: *mut ::std::os::raw::c_char,
    );

    // Steps
    // ========================================================================================
    #[link_name = "\u{1}_Z15zkevm_steps_newv"]
    pub fn zkevm_steps_new() -> *mut c_void;

    #[link_name = "\u{1}_Z16zkevm_steps_freePv"]
    pub fn zkevm_steps_free(pZkevmSteps: *mut c_void);

    #[link_name = "\u{1}_Z14c12a_steps_newv"]
    pub fn c12a_steps_new() -> *mut c_void;

    #[link_name = "\u{1}_Z15c12a_steps_freePv"]
    pub fn c12a_steps_free(pC12aSteps: *mut c_void);

    #[link_name = "\u{1}_Z20recursive1_steps_newv"]
    pub fn recursive1_steps_new() -> *mut c_void;

    #[link_name = "\u{1}_Z21recursive1_steps_freePv"]
    pub fn recursive1_steps_free(pRecursive1Steps: *mut c_void);

    #[link_name = "\u{1}_Z20recursive2_steps_newv"]
    pub fn recursive2_steps_new() -> *mut c_void;

    #[link_name = "\u{1}_Z21recursive2_steps_freePv"]
    pub fn recursive2_steps_free(Recursive2Steps: *mut c_void);

    // FRIProof
    // ========================================================================================
    #[link_name = "\u{1}_Z13fri_proof_newPv"]
    pub fn fri_proof_new(pStarks: *mut c_void) -> *mut c_void;

    #[link_name = "\u{1}_Z18fri_proof_get_rootPvmm"]
    pub fn fri_proof_get_root(pFriProof: *mut c_void, root_index: u64, root_subindex: u64) -> *mut c_void;

    #[link_name = "\u{1}_Z23fri_proof_get_tree_rootPvmm"]
    pub fn fri_proof_get_tree_root(pFriProof: *mut c_void, tree_index: u64, root_index: u64) -> *mut c_void;

    #[link_name = "\u{1}_Z14fri_proof_freePv"]
    pub fn fri_proof_free(pFriProof: *mut c_void);

    // Config
    // ========================================================================================
    #[link_name = "\u{1}_Z10config_newPc"]
    pub fn config_new(filename: *mut ::std::os::raw::c_char) -> *mut c_void;

    #[link_name = "\u{1}_Z11config_freePv"]
    pub fn config_free(pConfig: *mut c_void);

    // Stark Info
    // ========================================================================================
    #[link_name = "\u{1}_Z13starkinfo_newPvPc"]
    pub fn starkinfo_new(
        pConfig: *mut ::std::os::raw::c_void,
        filename: *mut ::std::os::raw::c_char,
    ) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z14starkinfo_freePv"]
    pub fn starkinfo_free(pStarkInfo: *mut ::std::os::raw::c_void);

    // Starks
    // ========================================================================================
    #[link_name = "\u{1}_Z10starks_newPvPcbS0_S0_S0_S_"]
    pub fn starks_new(
        pConfig: *mut ::std::os::raw::c_void,
        constPols: *mut ::std::os::raw::c_char,
        mapConstPolsFile: bool,
        constantsTree: *mut ::std::os::raw::c_char,
        starkInfo: *mut ::std::os::raw::c_char,
        cHelpers: *mut ::std::os::raw::c_char,
        pAddress: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z14get_stark_infoPv"]
    pub fn get_stark_info(pStarks: *mut c_void) -> *mut c_void;

    #[link_name = "\u{1}_Z11starks_freePv"]
    pub fn starks_free(pStarks: *mut c_void);

    #[link_name = "\u{1}_Z16steps_params_newPvS_S_S_S_S_"]
    pub fn steps_params_new(
        pStarks: *mut ::std::os::raw::c_void,
        pChallenges: *mut ::std::os::raw::c_void,
        pSubproofValues: *mut ::std::os::raw::c_void,
        pEvals: *mut ::std::os::raw::c_void,
        pXDivXSubXi: *mut ::std::os::raw::c_void,
        pPublicInputs: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z17steps_params_freePv"]
    pub fn steps_params_free(pStepsParams: *mut c_void);

    #[link_name = "\u{1}_Z20extend_and_merkelizePvmS_S_"]
    pub fn extend_and_merkelize(pStarks: *mut c_void, step: u64, pParams: *mut c_void, proof: *mut c_void);

    #[link_name = "\u{1}_Z16treesGL_get_rootPvmS_"]
    pub fn treesGL_get_root(pStarks: *mut ::std::os::raw::c_void, index: u64, root: *mut ::std::os::raw::c_void);

    #[link_name = "\u{1}_Z15calculate_h1_h2PvS_"]
    pub fn calculate_h1_h2(pStarks: *mut c_void, pParams: *mut c_void);

    #[link_name = "\u{1}_Z11calculate_zPvS_"]
    pub fn calculate_z(pStarks: *mut c_void, pParams: *mut c_void);

    #[link_name = "\u{1}_Z21calculate_expressionsPvPcS_S_"]
    pub fn calculate_expressions(
        pStarks: *mut ::std::os::raw::c_void,
        step: *mut ::std::os::raw::c_char,
        pParams: *mut ::std::os::raw::c_void,
        pChelpersSteps: *mut ::std::os::raw::c_void,
    );

    #[link_name = "\u{1}_Z13compute_stagePvjmS_S_S_S_"]
    pub fn compute_stage(
        pStarks: *mut ::std::os::raw::c_void,
        elementType: u32,
        step: u64,
        pParams: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
        pTranscript: *mut ::std::os::raw::c_void,
        pChelpersSteps: *mut ::std::os::raw::c_void,
    );

    #[link_name = "\u{1}_Z9compute_qPvS_S_"]
    pub fn compute_q(pStarks: *mut c_void, pParams: *mut c_void, pProof: *mut c_void);

    #[link_name = "\u{1}_Z13compute_evalsPvS_S_"]
    pub fn compute_evals(pStarks: *mut c_void, pParams: *mut c_void, pProof: *mut c_void);

    #[link_name = "\u{1}_Z15compute_fri_polPvmS_S_"]
    pub fn compute_fri_pol(
        pStarks: *mut ::std::os::raw::c_void,
        step: u64,
        pParams: *mut ::std::os::raw::c_void,
        cHelpersSteps: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z19compute_fri_foldingPvS_S_mS_"]
    pub fn compute_fri_folding(
        pStarks: *mut c_void,
        pProof: *mut c_void,
        pFriPol: *mut c_void,
        step: u64,
        challenge: *mut c_void,
    );

    #[link_name = "\u{1}_Z19compute_fri_queriesPvS_S_Pm"]
    pub fn compute_fri_queries(pStarks: *mut c_void, pProof: *mut c_void, pFriPol: *mut c_void, friQueries: *mut u64);

    // CommitPolsStarks
    // ========================================================================================
    #[link_name = "\u{1}_Z22commit_pols_starks_newPvmm"]
    pub fn commit_pols_starks_new(pAddress: *mut c_void, degree: u64, nCommitedPols: u64) -> *mut c_void;

    #[link_name = "\u{1}_Z23commit_pols_starks_freePv"]
    pub fn commit_pols_starks_free(pCommitPolsStarks: *mut c_void);

    // Circom
    // ========================================================================================
    #[link_name = "\u{1}_Z24circom_get_commited_polsPvPcS0_S_mm"]
    pub fn circom_get_commited_pols(
        pCommitPolsStarks: *mut c_void,
        zkevmVerifier: *mut ::std::os::raw::c_char,
        execFile: *mut ::std::os::raw::c_char,
        zkin: *mut c_void,
        N: u64,
        nCols: u64,
    );

    #[link_name = "\u{1}_Z35circom_recursive1_get_commited_polsPvPcS0_S_mm"]
    pub fn circom_recursive1_get_commited_pols(
        pCommitPolsStarks: *mut c_void,
        zkevmVerifier: *mut ::std::os::raw::c_char,
        execFile: *mut ::std::os::raw::c_char,
        zkin: *mut c_void,
        N: u64,
        nCols: u64,
    );

    // zkin
    // ========================================================================================
    #[link_name = "\u{1}_Z8zkin_newPvS_mS_mS_"]
    pub fn zkin_new(
        pStarkInfo: *mut c_void,
        pFriProof: *mut c_void,
        numPublicInputs: ::std::os::raw::c_ulong,
        pPublicInputs: *mut c_void,
        numRootC: ::std::os::raw::c_ulong,
        pRootC: *mut c_void,
    ) -> *mut c_void;

    // Transcript
    // ========================================================================================
    #[link_name = "\u{1}_Z14transcript_newj"]
    pub fn transcript_new(element_type: u32) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z14transcript_addPvS_m"]
    pub fn transcript_add(pTranscript: *mut c_void, pInput: *mut c_void, size: u64);

    #[link_name = "\u{1}_Z25transcript_add_polinomialPvS_"]
    pub fn transcript_add_polinomial(pTranscript: *mut c_void, pPolinomial: *mut c_void);

    #[link_name = "\u{1}_Z15transcript_freePvj"]
    pub fn transcript_free(pTranscript: *mut ::std::os::raw::c_void, element_type: u32);

    #[link_name = "\u{1}_Z13get_challengePvS_S_"]
    pub fn get_challenge(pStarks: *mut c_void, pTranscript: *mut c_void, pElement: *mut c_void);
    
    #[link_name = "\u{1}_Z16get_permutationsPvPmmm"]
    pub fn get_permutations(pTranscript: *mut c_void, res: *mut u64, n: u64, nBits: u64);

    // Polinomial
    // ========================================================================================
    #[link_name = "\u{1}_Z14polinomial_newmmPc"]
    pub fn polinomial_new(degree: u64, dim: u64, name: *mut ::std::os::raw::c_char) -> *mut c_void;

    #[link_name = "\u{1}_Z24polinomial_get_p_elementPvm"]
    pub fn polinomial_get_p_element(pPolinomial: *mut c_void, index: u64) -> *mut c_void;

    #[link_name = "\u{1}_Z15polinomial_freePv"]
    pub fn polinomial_free(pPolinomial: *mut c_void);
}
