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
    pub fn fri_proof_new(pStarks: *mut ::std::os::raw::c_void) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z18fri_proof_get_rootPvmm"]
    pub fn fri_proof_get_root(
        pFriProof: *mut ::std::os::raw::c_void,
        root_index: u64,
        root_subindex: u64,
    ) -> *mut ::std::os::raw::c_void;
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
        pChelpersSteps: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z14fri_proof_freePv"]
    pub fn fri_proof_free(pFriProof: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z13starkinfo_newPc"]
    pub fn starkinfo_new(filename: *mut ::std::os::raw::c_char) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z13get_mapTotalNPv"]
    pub fn get_mapTotalN(pStarkInfo: *mut ::std::os::raw::c_void) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z14set_mapOffsetsPvS_"]
    pub fn set_mapOffsets(
        pStarkInfo: *mut ::std::os::raw::c_void,
        pChelpers: *mut ::std::os::raw::c_void,
    );
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
    #[link_name = "\u{1}_Z18get_map_sections_nPvPc"]
    pub fn get_map_sections_n(
        pStarkInfo: *mut ::std::os::raw::c_void,
        stage: *mut ::std::os::raw::c_char,
    ) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z14starkinfo_freePv"]
    pub fn starkinfo_free(pStarkInfo: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z10starks_newPvS_S_"]
    pub fn starks_new(
        pConfig: *mut ::std::os::raw::c_void,
        starkInfo: *mut ::std::os::raw::c_void,
        pCHelpersSteps: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z18starks_new_defaultPvS_"]
    pub fn starks_new_default(
        starkInfo: *mut ::std::os::raw::c_void,
        pCHelpersSteps: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z14get_stark_infoPv"]
    pub fn get_stark_info(pStarks: *mut ::std::os::raw::c_void) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z11starks_freePv"]
    pub fn starks_free(pStarks: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z12chelpers_newPc"]
    pub fn chelpers_new(cHelpers: *mut ::std::os::raw::c_char) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z13chelpers_freePv"]
    pub fn chelpers_free(pChelpers: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z20extend_and_merkelizePvmS_S_"]
    pub fn extend_and_merkelize(
        pStarks: *mut ::std::os::raw::c_void,
        step: u64,
        pChelpersSteps: *mut ::std::os::raw::c_void,
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
    #[link_name = "\u{1}_Z29calculate_quotient_polynomialPv"]
    pub fn calculate_quotient_polynomial(pChelpersSteps: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z28calculate_impols_expressionsPvm"]
    pub fn calculate_impols_expressions(pChelpersSteps: *mut ::std::os::raw::c_void, step: u64);
}
extern "C" {
    #[link_name = "\u{1}_Z25compute_stage_expressionsPvjmS_S_"]
    pub fn compute_stage_expressions(
        pStarks: *mut ::std::os::raw::c_void,
        elementType: u32,
        step: u64,
        pChelpersSteps: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z12commit_stagePvjmS_S_"]
    pub fn commit_stage(
        pStarks: *mut ::std::os::raw::c_void,
        elementType: u32,
        step: u64,
        pChelpersSteps: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z13compute_evalsPvS_S_"]
    pub fn compute_evals(
        pStarks: *mut ::std::os::raw::c_void,
        pChelpersSteps: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z15compute_fri_polPvmS_"]
    pub fn compute_fri_pol(
        pStarks: *mut ::std::os::raw::c_void,
        step: u64,
        cHelpersSteps: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z11get_fri_polPvS_"]
    pub fn get_fri_pol(
        pStarks: *mut ::std::os::raw::c_void,
        pChelpersSteps: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z19compute_fri_foldingPvmS_S_S_"]
    pub fn compute_fri_folding(
        pStarks: *mut ::std::os::raw::c_void,
        step: u64,
        pChelpersSteps: *mut ::std::os::raw::c_void,
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
    #[link_name = "\u{1}_Z14get_proof_rootPvmm"]
    pub fn get_proof_root(
        pProof: *mut ::std::os::raw::c_void,
        stage_id: u64,
        index: u64,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z13resize_vectorPvmb"]
    pub fn resize_vector(pVector: *mut ::std::os::raw::c_void, newSize: u64, value: bool);
}
extern "C" {
    #[link_name = "\u{1}_Z21set_bool_vector_valuePvmb"]
    pub fn set_bool_vector_value(pVector: *mut ::std::os::raw::c_void, index: u64, value: bool);
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
    #[link_name = "\u{1}_Z22commit_pols_starks_newPvmm"]
    pub fn commit_pols_starks_new(
        pAddress: *mut ::std::os::raw::c_void,
        degree: u64,
        nCommitedPols: u64,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z23commit_pols_starks_freePv"]
    pub fn commit_pols_starks_free(pCommitPolsStarks: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z14const_pols_newPvPc"]
    pub fn const_pols_new(
        pStarkInfo: *mut ::std::os::raw::c_void,
        constPolsFile: *mut ::std::os::raw::c_char,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z14const_pols_newPvPcS0_"]
    pub fn const_pols_new1(
        pStarkInfo: *mut ::std::os::raw::c_void,
        constPolsFile: *mut ::std::os::raw::c_char,
        constTreeFile: *mut ::std::os::raw::c_char,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z15const_pols_freePv"]
    pub fn const_pols_free(pConstPols: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z8zkin_newPvS_mS_mS_"]
    pub fn zkin_new(
        pStarkInfo: *mut ::std::os::raw::c_void,
        pFriProof: *mut ::std::os::raw::c_void,
        numPublicInputs: ::std::os::raw::c_ulong,
        pPublicInputs: *mut ::std::os::raw::c_void,
        numRootC: ::std::os::raw::c_ulong,
        pRootC: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
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
    #[link_name = "\u{1}_Z18chelpers_steps_newPvS_S_"]
    pub fn chelpers_steps_new(
        pStarkInfo: *mut ::std::os::raw::c_void,
        pChelpers: *mut ::std::os::raw::c_void,
        pConstPols: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z17set_trace_pointerPvS_"]
    pub fn set_trace_pointer(
        pCHelpersSteps: *mut ::std::os::raw::c_void,
        ptr: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z11init_paramsPvS_S_S_S_"]
    pub fn init_params(
        pCHelpersSteps: *mut ::std::os::raw::c_void,
        pChallenges: *mut ::std::os::raw::c_void,
        pSubproofValues: *mut ::std::os::raw::c_void,
        pEvals: *mut ::std::os::raw::c_void,
        pPublicInputs: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z12reset_paramsPv"]
    pub fn reset_params(pCHelpersSteps: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z18verify_constraintsPvm"]
    pub fn verify_constraints(pCHelpersSteps: *mut ::std::os::raw::c_void, step: u64) -> bool;
}
extern "C" {
    #[link_name = "\u{1}_Z21set_commit_calculatedPvm"]
    pub fn set_commit_calculated(pCHelpersSteps: *mut ::std::os::raw::c_void, id: u64);
}
extern "C" {
    #[link_name = "\u{1}_Z23can_stage_be_calculatedPvm"]
    pub fn can_stage_be_calculated(pCHelpersSteps: *mut ::std::os::raw::c_void, step: u64);
}
extern "C" {
    #[link_name = "\u{1}_Z24can_impols_be_calculatedPvm"]
    pub fn can_impols_be_calculated(pCHelpersSteps: *mut ::std::os::raw::c_void, step: u64);
}
extern "C" {
    #[link_name = "\u{1}_Z19chelpers_steps_freePv"]
    pub fn chelpers_steps_free(pCHelpersSteps: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z20get_hint_ids_by_namePvPc"]
    pub fn get_hint_ids_by_name(
        pChelpersSteps: *mut ::std::os::raw::c_void,
        hintName: *mut ::std::os::raw::c_char,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z14get_hint_fieldPvmPcb"]
    pub fn get_hint_field(
        pChelpersSteps: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
        dest: bool,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z14set_hint_fieldPvS_mPc"]
    pub fn set_hint_field(
        pChelpersSteps: *mut ::std::os::raw::c_void,
        values: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z14polinomial_newmmPc"]
    pub fn polinomial_new(
        degree: u64,
        dim: u64,
        name: *mut ::std::os::raw::c_char,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z24polinomial_get_p_elementPvm"]
    pub fn polinomial_get_p_element(
        pPolinomial: *mut ::std::os::raw::c_void,
        index: u64,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z15polinomial_freePv"]
    pub fn polinomial_free(pPolinomial: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z22goldilocks_linear_hashPvS_"]
    pub fn goldilocks_linear_hash(
        pInput: *mut ::std::os::raw::c_void,
        pOutput: *mut ::std::os::raw::c_void,
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
    ) -> bool;
}