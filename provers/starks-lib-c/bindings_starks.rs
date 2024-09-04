extern "C" {
    #[link_name = "\u{1}_Z15save_challengesPvPcS0_"]
    pub fn save_challenges(
        pChallenges: *mut ::std::os::raw::c_void,
        globalInfoFile: *mut ::std::os::raw::c_char,
        fileDir: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z12save_publicsmPvPc"]
    pub fn save_publics(
        numPublicInputs: ::std::os::raw::c_ulong,
        pPublicInputs: *mut ::std::os::raw::c_void,
        fileDir: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z10save_proofmPvS_Pc"]
    pub fn save_proof(
        proof_id: u64,
        pStarkInfo: *mut ::std::os::raw::c_void,
        pFriProof: *mut ::std::os::raw::c_void,
        fileDir: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z13fri_proof_newPv"]
    pub fn fri_proof_new(pSetupCtx: *mut ::std::os::raw::c_void) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z23fri_proof_get_tree_rootPvmm"]
    pub fn fri_proof_get_tree_root(
        pFriProof: *mut ::std::os::raw::c_void,
        tree_index: u64,
        root_index: u64,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z28fri_proof_set_subproofvaluesPvS_"]
    pub fn fri_proof_set_subproofvalues(
        pFriProof: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z14fri_proof_freePv"]
    pub fn fri_proof_free(pFriProof: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z13setup_ctx_newPvS_S_"]
    pub fn setup_ctx_new(
        p_stark_info: *mut ::std::os::raw::c_void,
        p_expression_bin: *mut ::std::os::raw::c_void,
        p_const_pols: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z14setup_ctx_freePv"]
    pub fn setup_ctx_free(pSetupCtx: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z14stark_info_newPc"]
    pub fn stark_info_new(filename: *mut ::std::os::raw::c_char) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z15get_map_total_nPv"]
    pub fn get_map_total_n(pStarkInfo: *mut ::std::os::raw::c_void) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z15get_map_offsetsPvPcb"]
    pub fn get_map_offsets(
        pStarkInfo: *mut ::std::os::raw::c_void,
        stage: *mut ::std::os::raw::c_char,
        flag: bool,
    ) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z15stark_info_freePv"]
    pub fn stark_info_free(pStarkInfo: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z14const_pols_newPcPv"]
    pub fn const_pols_new(
        filename: *mut ::std::os::raw::c_char,
        pStarkInfo: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z15const_pols_freePv"]
    pub fn const_pols_free(pConstPols: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z19expressions_bin_newPc"]
    pub fn expressions_bin_new(
        filename: *mut ::std::os::raw::c_char,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z20expressions_bin_freePv"]
    pub fn expressions_bin_free(pExpressionsBin: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z19expressions_ctx_newPv"]
    pub fn expressions_ctx_new(
        pSetupCtx: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z18verify_constraintsPvS_m"]
    pub fn verify_constraints(
        pExpressionsCtx: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        step: u64,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z11get_fri_polPvS_"]
    pub fn get_fri_pol(
        pExpressionsCtx: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z20get_hint_ids_by_namePvPc"]
    pub fn get_hint_ids_by_name(
        pExpressionsCtx: *mut ::std::os::raw::c_void,
        hintName: *mut ::std::os::raw::c_char,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z14get_hint_fieldPvS_mPcbb"]
    pub fn get_hint_field(
        pExpressionsCtx: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
        dest: bool,
        print_expression: bool,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z14set_hint_fieldPvS_S_mPc"]
    pub fn set_hint_field(
        pExpressionsCtx: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        values: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z20expressions_ctx_freePv"]
    pub fn expressions_ctx_free(pExpressionsCtx: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z21set_commit_calculatedPvm"]
    pub fn set_commit_calculated(pExpressionsCtx: *mut ::std::os::raw::c_void, id: u64);
}
extern "C" {
    #[link_name = "\u{1}_Z23can_stage_be_calculatedPvm"]
    pub fn can_stage_be_calculated(pExpressionsCtx: *mut ::std::os::raw::c_void, step: u64);
}
extern "C" {
    #[link_name = "\u{1}_Z24can_impols_be_calculatedPvm"]
    pub fn can_impols_be_calculated(pExpressionsCtx: *mut ::std::os::raw::c_void, step: u64);
}
extern "C" {
    #[link_name = "\u{1}_Z11init_paramsPvS_S_S_S_"]
    pub fn init_params(
        ptr: *mut ::std::os::raw::c_void,
        public_inputs: *mut ::std::os::raw::c_void,
        challenges: *mut ::std::os::raw::c_void,
        evals: *mut ::std::os::raw::c_void,
        subproofValues: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z10starks_newPvS_S_"]
    pub fn starks_new(
        pConfig: *mut ::std::os::raw::c_void,
        pSetupCtx: *mut ::std::os::raw::c_void,
        pExpressionsCtx: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z18starks_new_defaultPvS_"]
    pub fn starks_new_default(
        pSetupCtx: *mut ::std::os::raw::c_void,
        pExpressionsCtx: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z11starks_freePv"]
    pub fn starks_free(pStarks: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z20extend_and_merkelizePvmS_S_"]
    pub fn extend_and_merkelize(
        pStarks: *mut ::std::os::raw::c_void,
        step: u64,
        pParams: *mut ::std::os::raw::c_void,
        proof: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z16treesGL_get_rootPvmS_"]
    pub fn treesGL_get_root(
        pStarks: *mut ::std::os::raw::c_void,
        index: u64,
        root: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z29calculate_quotient_polynomialPvS_"]
    pub fn calculate_quotient_polynomial(
        pExpressionsCtx: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z28calculate_impols_expressionsPvS_m"]
    pub fn calculate_impols_expressions(
        pExpressionsCtx: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        step: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z12commit_stagePvjmS_S_"]
    pub fn commit_stage(
        pStarks: *mut ::std::os::raw::c_void,
        elementType: u32,
        step: u64,
        pParams: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z13compute_evalsPvS_S_"]
    pub fn compute_evals(
        pStarks: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z15compute_fri_polPvmS_"]
    pub fn compute_fri_pol(
        pStarks: *mut ::std::os::raw::c_void,
        step: u64,
        pParams: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z19compute_fri_foldingPvmS_S_S_"]
    pub fn compute_fri_folding(
        pStarks: *mut ::std::os::raw::c_void,
        step: u64,
        pParams: *mut ::std::os::raw::c_void,
        pChallenge: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z19compute_fri_queriesPvS_Pm"]
    pub fn compute_fri_queries(
        pStarks: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
        friQueries: *mut u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z14calculate_hashPvS_S_m"]
    pub fn calculate_hash(
        pStarks: *mut ::std::os::raw::c_void,
        pHhash: *mut ::std::os::raw::c_void,
        pBuffer: *mut ::std::os::raw::c_void,
        nElements: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z14transcript_newjmb"]
    pub fn transcript_new(
        elementType: u32,
        arity: u64,
        custom: bool,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z14transcript_addPvS_m"]
    pub fn transcript_add(
        pTranscript: *mut ::std::os::raw::c_void,
        pInput: *mut ::std::os::raw::c_void,
        size: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z25transcript_add_polinomialPvS_"]
    pub fn transcript_add_polinomial(
        pTranscript: *mut ::std::os::raw::c_void,
        pPolinomial: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z15transcript_freePvj"]
    pub fn transcript_free(pTranscript: *mut ::std::os::raw::c_void, elementType: u32);
}
extern "C" {
    #[link_name = "\u{1}_Z13get_challengePvS_S_"]
    pub fn get_challenge(
        pStarks: *mut ::std::os::raw::c_void,
        pTranscript: *mut ::std::os::raw::c_void,
        pElement: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z16get_permutationsPvPmmm"]
    pub fn get_permutations(
        pTranscript: *mut ::std::os::raw::c_void,
        res: *mut u64,
        n: u64,
        nBits: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z25verify_global_constraintsPcS_PvS0_m"]
    pub fn verify_global_constraints(
        globalInfoFile: *mut ::std::os::raw::c_char,
        globalConstraintsBinFile: *mut ::std::os::raw::c_char,
        publics: *mut ::std::os::raw::c_void,
        pProofs: *mut ::std::os::raw::c_void,
        nProofs: u64,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z13print_by_namePvS_PcPmmmb"]
    pub fn print_by_name(
        pExpressionsCtx: *mut ::std::os::raw::c_void,
        pParams: *mut ::std::os::raw::c_void,
        name: *mut ::std::os::raw::c_char,
        lengths: *mut u64,
        first_value: u64,
        last_value: u64,
        return_values: bool,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z16print_expressionPvS_mmm"]
    pub fn print_expression(
        pExpressionCtx: *mut ::std::os::raw::c_void,
        pol: *mut ::std::os::raw::c_void,
        dim: u64,
        first_value: u64,
        last_value: u64,
    );
}
