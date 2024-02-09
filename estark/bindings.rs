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

    #[link_name = "\u{1}_Z26step2prev_parser_first_avxPvS_mm"]
    pub fn step2prev_parser_first_avx(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
        nrowsBatch: u64,
    );

    #[link_name = "\u{1}_Z29step2prev_parser_first_avx512PvS_mm"]
    pub fn step2prev_parser_first_avx512(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
        nrowsBatch: u64,
    );

    #[link_name = "\u{1}_Z24step2prev_first_parallelPvS_m"]
    pub fn step2prev_first_parallel(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
    );

    #[link_name = "\u{1}_Z26step3prev_parser_first_avxPvS_mm"]
    pub fn step3prev_parser_first_avx(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
        nrowsBatch: u64,
    );

    #[link_name = "\u{1}_Z29step3prev_parser_first_avx512PvS_mm"]
    pub fn step3prev_parser_first_avx512(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
        nrowsBatch: u64,
    );

    #[link_name = "\u{1}_Z24step3prev_first_parallelPvS_m"]
    pub fn step3prev_first_parallel(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
    );

    #[link_name = "\u{1}_Z22step3_parser_first_avxPvS_mm"]
    pub fn step3_parser_first_avx(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
        nrowsBatch: u64,
    );

    #[link_name = "\u{1}_Z25step3_parser_first_avx512PvS_mm"]
    pub fn step3_parser_first_avx512(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
        nrowsBatch: u64,
    );

    #[link_name = "\u{1}_Z20step3_first_parallelPvS_m"]
    pub fn step3_first_parallel(pSteps: *mut ::std::os::raw::c_void, pParams: *mut ::std::os::raw::c_void, nrows: u64);

    #[link_name = "\u{1}_Z25step42ns_parser_first_avxPvS_mm"]
    pub fn step42ns_parser_first_avx(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
        nrowsBatch: u64,
    );

    #[link_name = "\u{1}_Z28step42ns_parser_first_avx512PvS_mm"]
    pub fn step42ns_parser_first_avx512(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
        nrowsBatch: u64,
    );

    #[link_name = "\u{1}_Z23step42ns_first_parallelPvS_m"]
    pub fn step42ns_first_parallel(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
    );

    #[link_name = "\u{1}_Z25step52ns_parser_first_avxPvS_mm"]
    pub fn step52ns_parser_first_avx(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
        nrowsBatch: u64,
    );

    #[link_name = "\u{1}_Z28step52ns_parser_first_avx512PvS_mm"]
    pub fn step52ns_parser_first_avx512(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
        nrowsBatch: u64,
    );

    #[link_name = "\u{1}_Z23step52ns_first_parallelPvS_m"]
    pub fn step52ns_first_parallel(
        pSteps: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        nrows: u64,
    );

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

    #[link_name = "\u{1}_Z20transposeH1H2ColumnsPvS_PmS_"]
    pub fn transpose_h1_h2_columns(
        pStarks: *mut ::std::os::raw::c_void,
        pAddress: *mut ::std::os::raw::c_void,
        numCommited: *mut u64,
        pBuffer: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z17transposeH1H2RowsPvS_PmS_"]
    pub fn transpose_h1_h2_rows(
        pStarks: *mut ::std::os::raw::c_void,
        pAddress: *mut ::std::os::raw::c_void,
        numCommited: *mut u64,
        transPols: *mut ::std::os::raw::c_void,
    );

    #[link_name = "\u{1}_Z17transposeZColumnsPvS_PmS_"]
    pub fn transpose_z_columns(
        pStarks: *mut ::std::os::raw::c_void,
        pAddress: *mut ::std::os::raw::c_void,
        numCommited: *mut u64,
        pBuffer: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z14transposeZRowsPvS_PmS_"]
    pub fn transpose_z_rows(
        pStarks: *mut ::std::os::raw::c_void,
        pAddress: *mut ::std::os::raw::c_void,
        numCommited: *mut u64,
        transPols: *mut ::std::os::raw::c_void,
    );

    #[link_name = "\u{1}_Z5evmapPvS_S_S_S_"]
    pub fn evmap(
        pStarks: *mut ::std::os::raw::c_void,
        pAddress: *mut ::std::os::raw::c_void,
        evals: *mut ::std::os::raw::c_void,
        LEv: *mut ::std::os::raw::c_void,
        LpEv: *mut ::std::os::raw::c_void,
    );

    // #[link_name = "\u{1}_Z20transcript_add_arrayPvS_mS_"]
    // pub fn transcript_add_array(
    //     pStarks: *mut ::std::os::raw::c_void,
    //     pTranscript: *mut ::std::os::raw::c_void,
    //     numElements: ::std::os::raw::c_ulong,
    //     pArray: *mut ::std::os::raw::c_void,
    // );

    // #[link_name = "\u{1}_Z25transcript_add_polynomialPvS_S_"]
    // pub fn transcript_add_polynomial(
    //     pStarks: *mut ::std::os::raw::c_void,
    //     pTranscript: *mut ::std::os::raw::c_void,
    //     pPolynomial: *mut ::std::os::raw::c_void,
    // );

    // #[link_name = "\u{1}_Z14get_challengesPvS_mm"]
    // pub fn get_challenges(
    //     pTranscript: *mut ::std::os::raw::c_void,
    //     pStepsParams: *mut ::std::os::raw::c_void,
    //     nChallenges: u64,
    //     index: u64,
    // );

    #[link_name = "\u{1}_Z16steps_params_newPvS_S_S_S_S_"]
    pub fn steps_params_new(
        pStarks: *mut ::std::os::raw::c_void,
        pChallenges: *mut ::std::os::raw::c_void,
        pEvals: *mut ::std::os::raw::c_void,
        pXDivXSubXi: *mut ::std::os::raw::c_void,
        pXDivXSubWXi: *mut ::std::os::raw::c_void,
        pPublicInputs: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z17steps_params_freePv"]
    pub fn steps_params_free(pStepsParams: *mut ::std::os::raw::c_void);

    #[link_name = "\u{1}_Z14tree_merkelizePvm"]
    pub fn tree_merkelize(pStarks: *mut ::std::os::raw::c_void, index: u64);

    #[link_name = "\u{1}_Z13tree_get_rootPvmS_"]
    pub fn tree_get_root(pStarks: *mut ::std::os::raw::c_void, index: u64, root: *mut ::std::os::raw::c_void);

    #[link_name = "\u{1}_Z9extendPolPvm"]
    pub fn extendPol(pStarks: *mut ::std::os::raw::c_void, step: u64);

    #[link_name = "\u{1}_Z11get_pbufferPv"]
    pub fn get_pbuffer(pStarks: *mut ::std::os::raw::c_void) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z15calculate_h1_h2PvS_"]
    pub fn calculate_h1_h2(pStarks: *mut ::std::os::raw::c_void, pTransPols: *mut ::std::os::raw::c_void);

    #[link_name = "\u{1}_Z11calculate_zPvS_"]
    pub fn calculate_z(pStarks: *mut ::std::os::raw::c_void, pNewPols: *mut ::std::os::raw::c_void);

    #[link_name = "\u{1}_Z18calculate_exps_2nsPvS_S_"]
    pub fn calculate_exps_2ns(
        pStarks: *mut ::std::os::raw::c_void,
        pQq1: *mut ::std::os::raw::c_void,
        pQq2: *mut ::std::os::raw::c_void,
    );

    #[link_name = "\u{1}_Z18calculate_lev_lpevPvS_S_S_S_S_S_"]
    pub fn calculate_lev_lpev(
        pStarks: *mut ::std::os::raw::c_void,
        pLEv: *mut ::std::os::raw::c_void,
        pLpEv: *mut ::std::os::raw::c_void,
        pXis: *mut ::std::os::raw::c_void,
        pWxis: *mut ::std::os::raw::c_void,
        pC_w: *mut ::std::os::raw::c_void,
        pChallenges: *mut ::std::os::raw::c_void,
    );

    #[link_name = "\u{1}_Z20calculate_xdivxsubxiPvmS_S_S_S_S_"]
    pub fn calculate_xdivxsubxi(
        pStarks: *mut ::std::os::raw::c_void,
        extendBits: u64,
        xi: *mut ::std::os::raw::c_void,
        wxi: *mut ::std::os::raw::c_void,
        challenges: *mut ::std::os::raw::c_void,
        xDivXSubXi: *mut ::std::os::raw::c_void,
        xDivXSubWXi: *mut ::std::os::raw::c_void,
    );

    #[link_name = "\u{1}_Z14finalize_proofPvS_S_S_S_S_S_S_"]
    pub fn finalize_proof(
        pStarks: *mut ::std::os::raw::c_void,
        proof: *mut ::std::os::raw::c_void,
        transcript: *mut ::std::os::raw::c_void,
        evals: *mut ::std::os::raw::c_void,
        root0: *mut ::std::os::raw::c_void,
        root1: *mut ::std::os::raw::c_void,
        root2: *mut ::std::os::raw::c_void,
        root3: *mut ::std::os::raw::c_void,
    );

    // #[link_name = "\u{1}_Z20extend_and_merkelizePvmS_S_S_"]
    // pub fn extend_and_merkelize(
    //     pStarks: *mut ::std::os::raw::c_void,
    //     step: u64,
    //     pStepsParams: *mut ::std::os::raw::c_void,
    //     pStarkInfo: *mut ::std::os::raw::c_void,
    //     pProof: *mut ::std::os::raw::c_void,
    // );

    // #[link_name = "\u{1}_Z21calculate_expressionsPvPcmS_S_m"]
    // pub fn calculate_expressions(
    //     pStarks: *mut ::std::os::raw::c_void,
    //     step: *mut ::std::os::raw::c_char,
    //     nRowsStepBatch: u64,
    //     pSteps: *mut ::std::os::raw::c_void,
    //     pStepsParams: *mut ::std::os::raw::c_void,
    //     N: u64,
    // );

    // #[link_name = "\u{1}_Z14get_stark_infoPv"]
    // pub fn get_stark_info(pStarks: *mut ::std::os::raw::c_void) -> *mut ::std::os::raw::c_void;

    // #[link_name = "\u{1}_Z9get_proofPv"]
    // pub fn get_proof(pStarks: *mut ::std::os::raw::c_void) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z23get_num_rows_step_batchPv"]
    pub fn get_num_rows_step_batch(pStarks: *mut ::std::os::raw::c_void) -> u64;

    // #[link_name = "\u{1}_Z14calculate_h1h2PvS_S_"]
    // pub fn calculate_h1h2(
    //     pStarks: *mut ::std::os::raw::c_void,
    //     pStepsParams: *mut ::std::os::raw::c_void,
    //     pStarkInfo: *mut ::std::os::raw::c_void,
    // );

    // #[link_name = "\u{1}_Z11calculate_zPvS_S_"]
    // pub fn calculate_z(
    //     pStarks: *mut ::std::os::raw::c_void,
    //     pStepsParams: *mut ::std::os::raw::c_void,
    //     pStarkInfo: *mut ::std::os::raw::c_void,
    // );

    // #[link_name = "\u{1}_Z11calculate_qPvS_S_S_"]
    // pub fn calculate_q(
    //     pStarks: *mut ::std::os::raw::c_void,
    //     pStepsParams: *mut ::std::os::raw::c_void,
    //     pStarkInfo: *mut ::std::os::raw::c_void,
    //     pProof: *mut ::std::os::raw::c_void,
    // );

    // #[link_name = "\u{1}_Z15calculate_evalsPvS_S_S_"]
    // pub fn calculate_evals(
    //     pStarks: *mut ::std::os::raw::c_void,
    //     pStepsParams: *mut ::std::os::raw::c_void,
    //     pStarkInfo: *mut ::std::os::raw::c_void,
    //     pProof: *mut ::std::os::raw::c_void,
    // );

    // #[link_name = "\u{1}_Z17calculate_fri_polPvS_S_S_m"]
    // pub fn calculate_fri_pol(
    //     pStarks: *mut ::std::os::raw::c_void,
    //     pStepsParams: *mut ::std::os::raw::c_void,
    //     pStarkInfo: *mut ::std::os::raw::c_void,
    //     pSteps: *mut ::std::os::raw::c_void,
    //     nRowsStepBatch: u64,
    // ) -> *mut ::std::os::raw::c_void;

    // #[link_name = "\u{1}_Z21calculate_fri_foldingPvS_S_S_mS_"]
    // pub fn calculate_fri_folding(
    //     pStarks: *mut ::std::os::raw::c_void,
    //     pStarkInfo: *mut ::std::os::raw::c_void,
    //     pProof: *mut ::std::os::raw::c_void,
    //     pFriPol: *mut ::std::os::raw::c_void,
    //     step: u64,
    //     pPolinomial: *mut ::std::os::raw::c_void,
    // );

    // #[link_name = "\u{1}_Z21calculate_fri_queriesPvS_S_S_Pm"]
    // pub fn calculate_fri_queries(
    //     pStarks: *mut ::std::os::raw::c_void,
    //     pStarkInfo: *mut ::std::os::raw::c_void,
    //     pProof: *mut ::std::os::raw::c_void,
    //     pFriPol: *mut ::std::os::raw::c_void,
    //     friQueries: *mut u64,
    // );

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

    #[link_name = "\u{1}_Z10save_proofPvmS_PcS0_"]
    pub fn save_proof(
        pFriProof: *mut ::std::os::raw::c_void,
        numPublicInputs: ::std::os::raw::c_ulong,
        pPublicInputs: *mut ::std::os::raw::c_void,
        publicsOutputFile: *mut ::std::os::raw::c_char,
        filePrefix: *mut ::std::os::raw::c_char,
    );

    // Transcript
    // ========================================================================================
    #[link_name = "\u{1}_Z14transcript_newv"]
    pub fn transcript_new() -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z3putPvS_m"]
    pub fn transcript_put(pTranscript: *mut ::std::os::raw::c_void, pInput: *mut ::std::os::raw::c_void, size: u64);

    #[link_name = "\u{1}_Z20transcript_get_fieldPvS_"]
    pub fn transcript_get_field(pTranscript: *mut ::std::os::raw::c_void, pOutput: *mut ::std::os::raw::c_void);
    // #[link_name = "\u{1}_Z16get_permutationsPvPmmm"]
    // pub fn get_permutations(pTranscript: *mut ::std::os::raw::c_void, res: *mut u64, n: u64, nBits: u64);

    #[link_name = "\u{1}_Z15transcript_freePv"]
    pub fn transcript_free(pTranscript: *mut ::std::os::raw::c_void);

    // Polinomial
    // ========================================================================================
    #[link_name = "\u{1}_Z14polinomial_newmmPc"]
    pub fn polinomial_new(degree: u64, dim: u64, name: *mut ::std::os::raw::c_char) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z11get_addressPv"]
    pub fn polinomial_get_address(pPolinomial: *mut ::std::os::raw::c_void) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z13get_p_elementPvm"]
    pub fn polinomial_get_p_element(
        pPolinomial: *mut ::std::os::raw::c_void,
        index: u64,
    ) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z15polinomial_freePv"]
    pub fn polinomial_free(pPolinomial: *mut ::std::os::raw::c_void);

    // Commit Pols
    // ========================================================================================
    #[link_name = "\u{1}_Z15commit_pols_newPvm"]
    pub fn commit_pols_new(pAddress: *mut ::std::os::raw::c_void, degree: u64) -> *mut ::std::os::raw::c_void;

    #[link_name = "\u{1}_Z16commit_pols_freePv"]
    pub fn commit_pols_free(pCommitPols: *mut ::std::os::raw::c_void);
}
