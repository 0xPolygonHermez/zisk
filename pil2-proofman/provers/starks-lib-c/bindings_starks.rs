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
    #[link_name = "\u{1}_Z17save_proof_valuesPvPcS0_"]
    pub fn save_proof_values(
        pProofValues: *mut ::std::os::raw::c_void,
        globalInfoFile: *mut ::std::os::raw::c_char,
        fileDir: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z13fri_proof_newPvm"]
    pub fn fri_proof_new(pSetupCtx: *mut ::std::os::raw::c_void, instanceId: u64) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z23fri_proof_get_tree_rootPvS_m"]
    pub fn fri_proof_get_tree_root(
        pFriProof: *mut ::std::os::raw::c_void,
        root: *mut ::std::os::raw::c_void,
        tree_index: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z28fri_proof_set_airgroupvaluesPvS_"]
    pub fn fri_proof_set_airgroupvalues(
        pFriProof: *mut ::std::os::raw::c_void,
        airgroupValues: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z23fri_proof_set_airvaluesPvS_"]
    pub fn fri_proof_set_airvalues(pFriProof: *mut ::std::os::raw::c_void, airValues: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z23fri_proof_get_zkinproofPvS_S_S_PcS0_"]
    pub fn fri_proof_get_zkinproof(
        pFriProof: *mut ::std::os::raw::c_void,
        pPublics: *mut ::std::os::raw::c_void,
        pChallenges: *mut ::std::os::raw::c_void,
        pProofValues: *mut ::std::os::raw::c_void,
        globalInfoFile: *mut ::std::os::raw::c_char,
        fileDir: *mut ::std::os::raw::c_char,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z24fri_proof_get_zkinproofsmPPvS0_S_S_S_PcS1_"]
    pub fn fri_proof_get_zkinproofs(
        nProofs: u64,
        proofs: *mut *mut ::std::os::raw::c_void,
        pFriProofs: *mut *mut ::std::os::raw::c_void,
        pPublics: *mut ::std::os::raw::c_void,
        pProofValues: *mut ::std::os::raw::c_void,
        pChallenges: *mut ::std::os::raw::c_void,
        globalInfoFile: *mut ::std::os::raw::c_char,
        fileDir: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z24fri_proof_free_zkinproofPv"]
    pub fn fri_proof_free_zkinproof(pZkinProof: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z14fri_proof_freePv"]
    pub fn fri_proof_free(pFriProof: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z11proofs_freemPPvS0_b"]
    pub fn proofs_free(
        nProofs: u64,
        pStarks: *mut *mut ::std::os::raw::c_void,
        pFriProofs: *mut *mut ::std::os::raw::c_void,
        background: bool,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z15n_hints_by_namePvPc"]
    pub fn n_hints_by_name(p_expression_bin: *mut ::std::os::raw::c_void, hintName: *mut ::std::os::raw::c_char)
        -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z20get_hint_ids_by_namePvPmPc"]
    pub fn get_hint_ids_by_name(
        p_expression_bin: *mut ::std::os::raw::c_void,
        hintIds: *mut u64,
        hintName: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z14stark_info_newPcb"]
    pub fn stark_info_new(filename: *mut ::std::os::raw::c_char, verifier: bool) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z15get_map_total_nPvb"]
    pub fn get_map_total_n(pStarkInfo: *mut ::std::os::raw::c_void, recursive: bool) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z15stark_info_freePv"]
    pub fn stark_info_free(pStarkInfo: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z18prover_helpers_newPvb"]
    pub fn prover_helpers_new(pStarkInfo: *mut ::std::os::raw::c_void, pil1: bool) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z19prover_helpers_freePv"]
    pub fn prover_helpers_free(pProverHelpers: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z15load_const_treePvPcm"]
    pub fn load_const_tree(
        pConstTree: *mut ::std::os::raw::c_void,
        treeFilename: *mut ::std::os::raw::c_char,
        constTreeSize: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z15load_const_polsPvPcm"]
    pub fn load_const_pols(
        pConstPols: *mut ::std::os::raw::c_void,
        constFilename: *mut ::std::os::raw::c_char,
        constSize: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z19get_const_tree_sizePv"]
    pub fn get_const_tree_size(pStarkInfo: *mut ::std::os::raw::c_void) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z14get_const_sizePv"]
    pub fn get_const_size(pStarkInfo: *mut ::std::os::raw::c_void) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z20calculate_const_treePvS_S_Pc"]
    pub fn calculate_const_tree(
        pStarkInfo: *mut ::std::os::raw::c_void,
        pConstPolsAddress: *mut ::std::os::raw::c_void,
        pConstTree: *mut ::std::os::raw::c_void,
        treeFilename: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z19expressions_bin_newPcbb"]
    pub fn expressions_bin_new(
        filename: *mut ::std::os::raw::c_char,
        global: bool,
        verifier: bool,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z20expressions_bin_freePv"]
    pub fn expressions_bin_free(pExpressionsBin: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z14get_hint_fieldPvS_S_mPcS_"]
    pub fn get_hint_field(
        pSetupCtx: *mut ::std::os::raw::c_void,
        stepsParams: *mut ::std::os::raw::c_void,
        hintFieldValues: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
        hintOptions: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z21get_hint_field_valuesPvmPc"]
    pub fn get_hint_field_values(
        pSetupCtx: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
    ) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z20get_hint_field_sizesPvS_mPcS_"]
    pub fn get_hint_field_sizes(
        pSetupCtx: *mut ::std::os::raw::c_void,
        hintFieldValues: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
        hintOptions: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z15mul_hint_fieldsPvS_mPcS0_S0_S_S_"]
    pub fn mul_hint_fields(
        pSetupCtx: *mut ::std::os::raw::c_void,
        stepsParams: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldNameDest: *mut ::std::os::raw::c_char,
        hintFieldName1: *mut ::std::os::raw::c_char,
        hintFieldName2: *mut ::std::os::raw::c_char,
        hintOptions1: *mut ::std::os::raw::c_void,
        hintOptions2: *mut ::std::os::raw::c_void,
    ) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z14acc_hint_fieldPvS_mPcS0_S0_b"]
    pub fn acc_hint_field(
        pSetupCtx: *mut ::std::os::raw::c_void,
        stepsParams: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldNameDest: *mut ::std::os::raw::c_char,
        hintFieldNameAirgroupVal: *mut ::std::os::raw::c_char,
        hintFieldName: *mut ::std::os::raw::c_char,
        add: bool,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z19acc_mul_hint_fieldsPvS_mPcS0_S0_S0_S_S_b"]
    pub fn acc_mul_hint_fields(
        pSetupCtx: *mut ::std::os::raw::c_void,
        stepsParams: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldNameDest: *mut ::std::os::raw::c_char,
        hintFieldNameAirgroupVal: *mut ::std::os::raw::c_char,
        hintFieldName1: *mut ::std::os::raw::c_char,
        hintFieldName2: *mut ::std::os::raw::c_char,
        hintOptions1: *mut ::std::os::raw::c_void,
        hintOptions2: *mut ::std::os::raw::c_void,
        add: bool,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z20update_airgroupvaluePvS_mPcS0_S0_S_S_b"]
    pub fn update_airgroupvalue(
        pSetupCtx: *mut ::std::os::raw::c_void,
        stepsParams: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldNameAirgroupVal: *mut ::std::os::raw::c_char,
        hintFieldName1: *mut ::std::os::raw::c_char,
        hintFieldName2: *mut ::std::os::raw::c_char,
        hintOptions1: *mut ::std::os::raw::c_void,
        hintOptions2: *mut ::std::os::raw::c_void,
        add: bool,
    ) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z14set_hint_fieldPvS_S_mPc"]
    pub fn set_hint_field(
        pSetupCtx: *mut ::std::os::raw::c_void,
        stepsParams: *mut ::std::os::raw::c_void,
        values: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
    ) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z11get_hint_idPvmPc"]
    pub fn get_hint_id(
        pSetupCtx: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
    ) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z10starks_newPvS_"]
    pub fn starks_new(
        pSetupCtx: *mut ::std::os::raw::c_void,
        pConstTree: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z11starks_freePv"]
    pub fn starks_free(pStarks: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z16treesGL_get_rootPvmS_"]
    pub fn treesGL_get_root(pStarks: *mut ::std::os::raw::c_void, index: u64, root: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z16treesGL_set_rootPvmS_"]
    pub fn treesGL_set_root(pStarks: *mut ::std::os::raw::c_void, index: u64, pProof: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z18calculate_xdivxsubPvS_S_"]
    pub fn calculate_xdivxsub(
        pStarks: *mut ::std::os::raw::c_void,
        xiChallenge: *mut ::std::os::raw::c_void,
        xDivXSub: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z11get_fri_polPvS_"]
    pub fn get_fri_pol(
        pStarkInfo: *mut ::std::os::raw::c_void,
        buffer: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z24calculate_fri_polynomialPvS_"]
    pub fn calculate_fri_polynomial(pStarks: *mut ::std::os::raw::c_void, stepsParams: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z29calculate_quotient_polynomialPvS_"]
    pub fn calculate_quotient_polynomial(
        pStarks: *mut ::std::os::raw::c_void,
        stepsParams: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z28calculate_impols_expressionsPvmS_"]
    pub fn calculate_impols_expressions(
        pStarks: *mut ::std::os::raw::c_void,
        step: u64,
        stepsParams: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z34extend_and_merkelize_custom_commitPvmmS_S_S_S_Pc"]
    pub fn extend_and_merkelize_custom_commit(
        pStarks: *mut ::std::os::raw::c_void,
        commitId: u64,
        step: u64,
        buffer: *mut ::std::os::raw::c_void,
        bufferExt: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
        pBuffHelper: *mut ::std::os::raw::c_void,
        treeFile: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z18load_custom_commitPvmmS_S_S_Pc"]
    pub fn load_custom_commit(
        pStarks: *mut ::std::os::raw::c_void,
        commitId: u64,
        step: u64,
        buffer: *mut ::std::os::raw::c_void,
        bufferExt: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
        treeFile: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z12commit_stagePvjmS_S_S_S_"]
    pub fn commit_stage(
        pStarks: *mut ::std::os::raw::c_void,
        elementType: u32,
        step: u64,
        trace: *mut ::std::os::raw::c_void,
        buffer: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
        pBuffHelper: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z11compute_levPvS_S_"]
    pub fn compute_lev(
        pStarks: *mut ::std::os::raw::c_void,
        xiChallenge: *mut ::std::os::raw::c_void,
        LEv: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z13compute_evalsPvS_S_S_"]
    pub fn compute_evals(
        pStarks: *mut ::std::os::raw::c_void,
        params: *mut ::std::os::raw::c_void,
        LEv: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
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
    #[link_name = "\u{1}_Z19compute_fri_foldingmPvS_mmm"]
    pub fn compute_fri_folding(
        step: u64,
        buffer: *mut ::std::os::raw::c_void,
        pChallenge: *mut ::std::os::raw::c_void,
        nBitsExt: u64,
        prevBits: u64,
        currentBits: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z21compute_fri_merkelizePvS_mS_mm"]
    pub fn compute_fri_merkelize(
        pStarks: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
        step: u64,
        buffer: *mut ::std::os::raw::c_void,
        currentBits: u64,
        nextBits: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z15compute_queriesPvS_Pmmm"]
    pub fn compute_queries(
        pStarks: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
        friQueries: *mut u64,
        nQueries: u64,
        nTrees: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z19compute_fri_queriesPvS_Pmmmm"]
    pub fn compute_fri_queries(
        pStarks: *mut ::std::os::raw::c_void,
        pProof: *mut ::std::os::raw::c_void,
        friQueries: *mut u64,
        nQueries: u64,
        step: u64,
        currentBits: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z17set_fri_final_polPvS_m"]
    pub fn set_fri_final_pol(pProof: *mut ::std::os::raw::c_void, buffer: *mut ::std::os::raw::c_void, nBits: u64);
}
extern "C" {
    #[link_name = "\u{1}_Z14transcript_newjmb"]
    pub fn transcript_new(elementType: u32, arity: u64, custom: bool) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z14transcript_addPvS_m"]
    pub fn transcript_add(pTranscript: *mut ::std::os::raw::c_void, pInput: *mut ::std::os::raw::c_void, size: u64);
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
    pub fn get_permutations(pTranscript: *mut ::std::os::raw::c_void, res: *mut u64, n: u64, nBits: u64);
}
extern "C" {
    #[link_name = "\u{1}_Z17get_n_constraintsPv"]
    pub fn get_n_constraints(pSetupCtx: *mut ::std::os::raw::c_void) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z27get_constraints_lines_sizesPvPm"]
    pub fn get_constraints_lines_sizes(pSetupCtx: *mut ::std::os::raw::c_void, constraintsLinesSizes: *mut u64);
}
extern "C" {
    #[link_name = "\u{1}_Z21get_constraints_linesPvPPh"]
    pub fn get_constraints_lines(pSetupCtx: *mut ::std::os::raw::c_void, constraintsLines: *mut *mut u8);
}
extern "C" {
    #[link_name = "\u{1}_Z18verify_constraintsPvS_S_"]
    pub fn verify_constraints(
        pSetupCtx: *mut ::std::os::raw::c_void,
        stepsParams: *mut ::std::os::raw::c_void,
        constraintsInfo: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z24get_n_global_constraintsPv"]
    pub fn get_n_global_constraints(p_globalinfo_bin: *mut ::std::os::raw::c_void) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z34get_global_constraints_lines_sizesPvPm"]
    pub fn get_global_constraints_lines_sizes(
        p_globalinfo_bin: *mut ::std::os::raw::c_void,
        constraintsLinesSizes: *mut u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z28get_global_constraints_linesPvPPh"]
    pub fn get_global_constraints_lines(p_globalinfo_bin: *mut ::std::os::raw::c_void, constraintsLines: *mut *mut u8);
}
extern "C" {
    #[link_name = "\u{1}_Z25verify_global_constraintsPcPvS0_S0_S0_PS0_S0_"]
    pub fn verify_global_constraints(
        globalInfoFile: *mut ::std::os::raw::c_char,
        globalBin: *mut ::std::os::raw::c_void,
        publics: *mut ::std::os::raw::c_void,
        challenges: *mut ::std::os::raw::c_void,
        proofValues: *mut ::std::os::raw::c_void,
        airgroupValues: *mut *mut ::std::os::raw::c_void,
        globalConstraintsInfo: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z40get_hint_field_global_constraints_valuesPvmPc"]
    pub fn get_hint_field_global_constraints_values(
        p_globalinfo_bin: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
    ) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z39get_hint_field_global_constraints_sizesPcPvS0_mS_b"]
    pub fn get_hint_field_global_constraints_sizes(
        globalInfoFile: *mut ::std::os::raw::c_char,
        p_globalinfo_bin: *mut ::std::os::raw::c_void,
        hintFieldValues: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
        print_expression: bool,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z33get_hint_field_global_constraintsPcPvS0_S0_S0_S0_PS0_mS_b"]
    pub fn get_hint_field_global_constraints(
        globalInfoFile: *mut ::std::os::raw::c_char,
        p_globalinfo_bin: *mut ::std::os::raw::c_void,
        hintFieldValues: *mut ::std::os::raw::c_void,
        publics: *mut ::std::os::raw::c_void,
        challenges: *mut ::std::os::raw::c_void,
        proofValues: *mut ::std::os::raw::c_void,
        airgroupValues: *mut *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
        print_expression: bool,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z33set_hint_field_global_constraintsPcPvS0_S0_mS_"]
    pub fn set_hint_field_global_constraints(
        globalInfoFile: *mut ::std::os::raw::c_char,
        p_globalinfo_bin: *mut ::std::os::raw::c_void,
        proofValues: *mut ::std::os::raw::c_void,
        values: *mut ::std::os::raw::c_void,
        hintId: u64,
        hintFieldName: *mut ::std::os::raw::c_char,
    ) -> u64;
}
extern "C" {
    #[link_name = "\u{1}_Z9print_rowPvS_mm"]
    pub fn print_row(pSetupCtx: *mut ::std::os::raw::c_void, buffer: *mut ::std::os::raw::c_void, stage: u64, row: u64);
}
extern "C" {
    #[link_name = "\u{1}_Z19gen_recursive_proofPvPcmS_S_S_S_S_S0_b"]
    pub fn gen_recursive_proof(
        pSetupCtx: *mut ::std::os::raw::c_void,
        globalInfoFile: *mut ::std::os::raw::c_char,
        airgroupId: u64,
        witness: *mut ::std::os::raw::c_void,
        aux_trace: *mut ::std::os::raw::c_void,
        pConstPols: *mut ::std::os::raw::c_void,
        pConstTree: *mut ::std::os::raw::c_void,
        pPublicInputs: *mut ::std::os::raw::c_void,
        proof_file: *mut ::std::os::raw::c_char,
        vadcop: bool,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z12get_zkin_ptrPc"]
    pub fn get_zkin_ptr(zkin_file: *mut ::std::os::raw::c_char) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z21add_recursive2_verkeyPvPc"]
    pub fn add_recursive2_verkey(
        pZkin: *mut ::std::os::raw::c_void,
        recursive2VerKeyFilename: *mut ::std::os::raw::c_char,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z20join_zkin_recursive2PcmPvS0_S0_S0_S0_"]
    pub fn join_zkin_recursive2(
        globalInfoFile: *mut ::std::os::raw::c_char,
        airgroupId: u64,
        pPublics: *mut ::std::os::raw::c_void,
        pChallenges: *mut ::std::os::raw::c_void,
        zkin1: *mut ::std::os::raw::c_void,
        zkin2: *mut ::std::os::raw::c_void,
        starkInfoRecursive2: *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z15join_zkin_finalPvS_S_PcPS_S1_"]
    pub fn join_zkin_final(
        pPublics: *mut ::std::os::raw::c_void,
        pProofValues: *mut ::std::os::raw::c_void,
        pChallenges: *mut ::std::os::raw::c_void,
        globalInfoFile: *mut ::std::os::raw::c_char,
        zkinRecursive2: *mut *mut ::std::os::raw::c_void,
        starkInfoRecursive2: *mut *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z20get_serialized_proofPvPm"]
    pub fn get_serialized_proof(zkin: *mut ::std::os::raw::c_void, size: *mut u64) -> *mut ::std::os::raw::c_char;
}
extern "C" {
    #[link_name = "\u{1}_Z22deserialize_zkin_proofPc"]
    pub fn deserialize_zkin_proof(serialized_proof: *mut ::std::os::raw::c_char) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z14get_zkin_proofPc"]
    pub fn get_zkin_proof(zkin: *mut ::std::os::raw::c_char) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z15zkin_proof_freePv"]
    pub fn zkin_proof_free(pZkinProof: *mut ::std::os::raw::c_void);
}
extern "C" {
    #[link_name = "\u{1}_Z21serialized_proof_freePc"]
    pub fn serialized_proof_free(zkinCStr: *mut ::std::os::raw::c_char);
}
extern "C" {
    #[link_name = "\u{1}_Z18get_committed_polsPvPcS_S_mmmm"]
    pub fn get_committed_pols(
        circomWitness: *mut ::std::os::raw::c_void,
        execFile: *mut ::std::os::raw::c_char,
        witness: *mut ::std::os::raw::c_void,
        pPublics: *mut ::std::os::raw::c_void,
        sizeWitness: u64,
        N: u64,
        nPublics: u64,
        nCols: u64,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z21gen_final_snark_proofPvPcS0_"]
    pub fn gen_final_snark_proof(
        circomWitnessFinal: *mut ::std::os::raw::c_void,
        zkeyFile: *mut ::std::os::raw::c_char,
        outputDir: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z11setLogLevelm"]
    pub fn setLogLevel(level: u64);
}
extern "C" {
    #[link_name = "\u{1}_Z12stark_verifyPvS_S_PcS_S_S_"]
    pub fn stark_verify(
        jProof: *mut ::std::os::raw::c_void,
        pStarkInfo: *mut ::std::os::raw::c_void,
        pExpressionsBin: *mut ::std::os::raw::c_void,
        verkey: *mut ::std::os::raw::c_char,
        pPublics: *mut ::std::os::raw::c_void,
        pProofValues: *mut ::std::os::raw::c_void,
        challenges: *mut ::std::os::raw::c_void,
    ) -> bool;
}
extern "C" {
    #[link_name = "\u{1}_Z12save_to_filePvmS_mPc"]
    pub fn save_to_file(
        buffer: *mut ::std::os::raw::c_void,
        bufferSize: u64,
        publics: *mut ::std::os::raw::c_void,
        publicsSize: u64,
        name: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z14read_from_filePvmS_mPc"]
    pub fn read_from_file(
        buffer: *mut ::std::os::raw::c_void,
        bufferSize: u64,
        publics: *mut ::std::os::raw::c_void,
        publicsSize: u64,
        name: *mut ::std::os::raw::c_char,
    );
}
extern "C" {
    #[link_name = "\u{1}_Z13create_bufferm"]
    pub fn create_buffer(size: u64) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    #[link_name = "\u{1}_Z11free_bufferPv"]
    pub fn free_buffer(buffer: *mut ::std::os::raw::c_void);
}
