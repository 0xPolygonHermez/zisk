// Rust FFI declaration for the C function `int zkevm_prover_c(char* config_filename)`
#[allow(dead_code)]
extern "C" {
    #[link_name = "\u{1}_Z10zkevm_mainPcPv"]
    pub fn zkevm_main(config_filename: *mut ::std::os::raw::c_char, pAddress: *mut c_void) -> ::std::os::raw::c_int;

    #[link_name = "\u{1}_Z10save_proofPvS_mS_PcS0_"]
    pub fn save_proof(
        pStarks: *mut ::std::os::raw::c_void,
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

    #[link_name = "\u{1}_Z26step2prev_parser_first_avxPvS_mm"]
    pub fn step2prev_parser_first_avx(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64, nrowsBatch: u64);

    #[link_name = "\u{1}_Z29step2prev_parser_first_avx512PvS_mm"]
    pub fn step2prev_parser_first_avx512(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64, nrowsBatch: u64);

    #[link_name = "\u{1}_Z24step2prev_first_parallelPvS_m"]
    pub fn step2prev_first_parallel(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64);

    #[link_name = "\u{1}_Z26step3prev_parser_first_avxPvS_mm"]
    pub fn step3prev_parser_first_avx(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64, nrowsBatch: u64);

    #[link_name = "\u{1}_Z29step3prev_parser_first_avx512PvS_mm"]
    pub fn step3prev_parser_first_avx512(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64, nrowsBatch: u64);

    #[link_name = "\u{1}_Z24step3prev_first_parallelPvS_m"]
    pub fn step3prev_first_parallel(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64);

    #[link_name = "\u{1}_Z22step3_parser_first_avxPvS_mm"]
    pub fn step3_parser_first_avx(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64, nrowsBatch: u64);

    #[link_name = "\u{1}_Z25step3_parser_first_avx512PvS_mm"]
    pub fn step3_parser_first_avx512(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64, nrowsBatch: u64);

    #[link_name = "\u{1}_Z20step3_first_parallelPvS_m"]
    pub fn step3_first_parallel(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64);

    #[link_name = "\u{1}_Z25step42ns_parser_first_avxPvS_mm"]
    pub fn step42ns_parser_first_avx(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64, nrowsBatch: u64);

    #[link_name = "\u{1}_Z28step42ns_parser_first_avx512PvS_mm"]
    pub fn step42ns_parser_first_avx512(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64, nrowsBatch: u64);

    #[link_name = "\u{1}_Z23step42ns_first_parallelPvS_m"]
    pub fn step42ns_first_parallel(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64);

    #[link_name = "\u{1}_Z25step52ns_parser_first_avxPvS_mm"]
    pub fn step52ns_parser_first_avx(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64, nrowsBatch: u64);

    #[link_name = "\u{1}_Z28step52ns_parser_first_avx512PvS_mm"]
    pub fn step52ns_parser_first_avx512(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64, nrowsBatch: u64);

    #[link_name = "\u{1}_Z23step52ns_first_parallelPvS_m"]
    pub fn step52ns_first_parallel(pSteps: *mut c_void, pParams: *mut c_void, nrows: u64);

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

    // Starks
    // ========================================================================================
    #[link_name = "\u{1}_Z10starks_newPvPcbS0_S0_S_"]
    pub fn starks_new(
        pConfig: *mut c_void,
        constPols: *mut ::std::os::raw::c_char,
        mapConstPolsFile: bool,
        constantsTree: *mut ::std::os::raw::c_char,
        starkInfo: *mut ::std::os::raw::c_char,
        pAddress: *mut c_void,
    ) -> *mut c_void;

    #[link_name = "\u{1}_Z14get_stark_infoPv"]
    pub fn get_stark_info(pStarks: *mut c_void) -> *mut c_void;

    #[link_name = "\u{1}_Z16starks_gen_proofPvS_S_S_S_"]
    pub fn starks_genproof(
        pStarks: *mut c_void,
        pFRIProof: *mut c_void,
        pPublicInputs: *mut c_void,
        pVerkey: *mut c_void,
        pSteps: *mut c_void,
    );

    #[link_name = "\u{1}_Z11starks_freePv"]
    pub fn starks_free(pStarks: *mut c_void);

    #[link_name = "\u{1}_Z16steps_params_newPvS_S_S_S_S_"]
    pub fn steps_params_new(
        pStarks: *mut c_void,
        pChallenges: *mut c_void,
        pEvals: *mut c_void,
        pXDivXSubXi: *mut c_void,
        pXDivXSubWXi: *mut c_void,
        pPublicInputs: *mut c_void,
    ) -> *mut c_void;

    #[link_name = "\u{1}_Z17steps_params_freePv"]
    pub fn steps_params_free(pStepsParams: *mut c_void);

    #[link_name = "\u{1}_Z20extend_and_merkelizePvmS_S_"]
    pub fn extend_and_merkelize(pStarks: *mut c_void, step: u64, pParams: *mut c_void, proof: *mut c_void);

    #[link_name = "\u{1}_Z15calculate_h1_h2PvS_"]
    pub fn calculate_h1_h2(pStarks: *mut c_void, pParams: *mut c_void);

    #[link_name = "\u{1}_Z11calculate_zPvS_"]
    pub fn calculate_z(pStarks: *mut c_void, pParams: *mut c_void);

    #[link_name = "\u{1}_Z21calculate_expressionsPvPcmS_S_m"]
    pub fn calculate_expressions(
        pStarks: *mut c_void,
        step: *mut ::std::os::raw::c_char,
        nrowsStepBatch: u64,
        pSteps: *mut c_void,
        pParams: *mut c_void,
        n: u64,
    );

    #[link_name = "\u{1}_Z9compute_qPvS_S_"]
    pub fn compute_q(pStarks: *mut c_void, pParams: *mut c_void, pProof: *mut c_void);

    #[link_name = "\u{1}_Z13compute_evalsPvS_S_"]
    pub fn compute_evals(pStarks: *mut c_void, pParams: *mut c_void, pProof: *mut c_void);

    #[link_name = "\u{1}_Z15compute_fri_polPvS_S_m"]
    pub fn compute_fri_pol(
        pStarks: *mut c_void,
        pParams: *mut c_void,
        steps: *mut c_void,
        nrowsStepBatch: u64,
    ) -> *mut c_void;

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

    #[link_name = "\u{1}_Z23get_num_rows_step_batchPv"]
    pub fn get_num_rows_step_batch(pStarks: *mut c_void) -> u64;

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
        pStarks: *mut c_void,
        pFriProof: *mut c_void,
        numPublicInputs: ::std::os::raw::c_ulong,
        pPublicInputs: *mut c_void,
        numRootC: ::std::os::raw::c_ulong,
        pRootC: *mut c_void,
    ) -> *mut c_void;

    // Transcript
    // ========================================================================================
    #[link_name = "\u{1}_Z14transcript_newv"]
    pub fn transcript_new() -> *mut c_void;

    #[link_name = "\u{1}_Z14transcript_addPvS_m"]
    pub fn transcript_add(pTranscript: *mut c_void, pInput: *mut c_void, size: u64);

    #[link_name = "\u{1}_Z25transcript_add_polinomialPvS_"]
    pub fn transcript_add_polinomial(pTranscript: *mut c_void, pPolinomial: *mut c_void);

    #[link_name = "\u{1}_Z20transcript_get_fieldPvS_"]
    pub fn transcript_get_field(pTranscript: *mut c_void, pOutput: *mut c_void);

    #[link_name = "\u{1}_Z15transcript_freePv"]
    pub fn transcript_free(pTranscript: *mut c_void);

    #[link_name = "\u{1}_Z14get_challengesPvS_mm"]
    pub fn get_challenges(pTranscript: *mut c_void, pPolinomial: *mut c_void, nChallenges: u64, index: u64);

    #[link_name = "\u{1}_Z16get_permutationsPvPmmm"]
    pub fn get_permutations(pTranscript: *mut c_void, res: *mut u64, n: u64, nBits: u64);

    // Polinomial
    // ========================================================================================
    #[link_name = "\u{1}_Z14polinomial_newmmPc"]
    pub fn polinomial_new(degree: u64, dim: u64, name: *mut ::std::os::raw::c_char) -> *mut c_void;

    #[link_name = "\u{1}_Z19polinomial_new_voidv"]
    pub fn polinomial_new_void() -> *mut c_void;

    #[link_name = "\u{1}_Z22polinomial_get_addressPv"]
    pub fn polinomial_get_address(pPolinomial: *mut c_void) -> *mut c_void;

    #[link_name = "\u{1}_Z24polinomial_get_p_elementPvm"]
    pub fn polinomial_get_p_element(pPolinomial: *mut c_void, index: u64) -> *mut c_void;

    #[link_name = "\u{1}_Z15polinomial_freePv"]
    pub fn polinomial_free(pPolinomial: *mut c_void);

    // Commit Pols
    // ========================================================================================
    #[link_name = "\u{1}_Z15commit_pols_newPvm"]
    pub fn commit_pols_new(pAddress: *mut c_void, degree: u64) -> *mut c_void;

    #[link_name = "\u{1}_Z16commit_pols_freePv"]
    pub fn commit_pols_free(pCommitPols: *mut c_void);
}
