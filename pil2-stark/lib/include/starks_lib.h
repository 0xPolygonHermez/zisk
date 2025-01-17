#ifndef LIB_API_H
#define LIB_API_H
#include <stdint.h>

    // Save Proof
    // ========================================================================================
    void save_challenges(void *pChallenges, char* globalInfoFile, char *fileDir);
    void save_publics(unsigned long numPublicInputs, void *pPublicInputs, char *fileDir);
    void save_proof_values(void *pProofValues, char* globalInfoFile, char *fileDir);

    // FRIProof
    // ========================================================================================
    void *fri_proof_new(void *pSetupCtx, uint64_t instanceId);
    void fri_proof_get_tree_root(void *pFriProof, void* root, uint64_t tree_index);
    void fri_proof_set_airgroupvalues(void *pFriProof, void *airgroupValues);
    void fri_proof_set_airvalues(void *pFriProof, void *airValues);
    void *fri_proof_get_zkinproof(void *pFriProof, void* pPublics, void* pChallenges, void *pProofValues, char* globalInfoFile, char *fileDir);
    void fri_proof_get_zkinproofs(uint64_t nProofs, void**proofs, void **pFriProofs, void* pPublics, void *pProofValues, void* pChallenges, char* globalInfoFile, char *fileDir);
    void fri_proof_free_zkinproof(void *pZkinProof);
    void fri_proof_free(void *pFriProof);

    void proofs_free(uint64_t nProofs, void **pStarks, void **pFriProofs, bool background);

    // SetupCtx
    // ========================================================================================
    uint64_t n_hints_by_name(void *p_expression_bin, char* hintName);
    void get_hint_ids_by_name(void *p_expression_bin, uint64_t* hintIds, char* hintName);

    // Stark Info
    // ========================================================================================
    void *stark_info_new(char* filename, bool verifier);
    uint64_t get_map_total_n(void *pStarkInfo, bool recursive);
    void stark_info_free(void *pStarkInfo);

    // Prover Helpers
    // ========================================================================================
    void *prover_helpers_new(void *pStarkInfo, bool pil1);
    void prover_helpers_free(void *pProverHelpers);

    // Const Pols
    // ========================================================================================
    void load_const_tree(void *pConstTree, char *treeFilename, uint64_t constTreeSize);
    void load_const_pols(void *pConstPols, char *constFilename, uint64_t constSize);
    uint64_t get_const_tree_size(void *pStarkInfo);
    uint64_t get_const_size(void *pStarkInfo);
    void calculate_const_tree(void *pStarkInfo, void *pConstPolsAddress, void *pConstTree, char *treeFilename);

    // Expressions Bin
    // ========================================================================================
    void *expressions_bin_new(char* filename, bool global, bool verifier);
    void expressions_bin_free(void *pExpressionsBin);

    // Hints
    // ========================================================================================
    void get_hint_field(void *pSetupCtx, void* stepsParams, void* hintFieldValues, uint64_t hintId, char* hintFieldName, void* hintOptions);
    uint64_t get_hint_field_values(void *pSetupCtx, uint64_t hintId, char* hintFieldName);
    void get_hint_field_sizes(void *pSetupCtx, void* hintFieldValues, uint64_t hintId, char* hintFieldName, void* hintOptions);
    uint64_t mul_hint_fields(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldName1, char *hintFieldName2, void* hintOptions1, void *hintOptions2); 
    void acc_hint_field(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldNameAirgroupVal, char *hintFieldName, bool add);
    void acc_mul_hint_fields(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldNameAirgroupVal, char *hintFieldName1, char *hintFieldName2,  void* hintOptions1, void *hintOptions2, bool add);
    uint64_t update_airgroupvalue(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameAirgroupVal, char *hintFieldName1, char *hintFieldName2, void* hintOptions1, void *hintOptions2, bool add);
    uint64_t set_hint_field(void *pSetupCtx, void* stepsParams, void *values, uint64_t hintId, char* hintFieldName);
    uint64_t get_hint_id(void *pSetupCtx, uint64_t hintId, char * hintFieldName);

    // Starks
    // ========================================================================================
    void *starks_new(void *pSetupCtx, void *pConstTree);
    void starks_free(void *pStarks);

    void treesGL_get_root(void *pStarks, uint64_t index, void *root);
    void treesGL_set_root(void *pStarks, uint64_t index, void *pProof);

    void calculate_xdivxsub(void *pStarks, void* xiChallenge, void *xDivXSub);
    void *get_fri_pol(void *pStarkInfo, void *buffer);

    void calculate_fri_polynomial(void *pStarks, void* stepsParams);
    void calculate_quotient_polynomial(void *pStarks, void* stepsParams);
    void calculate_impols_expressions(void *pStarks, uint64_t step, void* stepsParams);

    void extend_and_merkelize_custom_commit(void *pStarks, uint64_t commitId, uint64_t step, void *buffer, void *bufferExt, void *pProof, void *pBuffHelper, char *treeFile);
    void load_custom_commit(void *pStarks, uint64_t commitId, uint64_t step, void *buffer, void *bufferExt, void *pProof, char *treeFile);

    void commit_stage(void *pStarks, uint32_t elementType, uint64_t step, void *trace, void *buffer, void *pProof, void *pBuffHelper);
    
    void compute_lev(void *pStarks, void *xiChallenge, void* LEv);
    void compute_evals(void *pStarks, void *params, void *LEv, void *pProof);

    void calculate_hash(void *pStarks, void *pHhash, void *pBuffer, uint64_t nElements);

    // FRI 
    // =================================================================================

    void compute_fri_folding(uint64_t step, void *buffer, void *pChallenge, uint64_t nBitsExt, uint64_t prevBits, uint64_t currentBits);
    void compute_fri_merkelize(void *pStarks, void *pProof, uint64_t step, void *buffer, uint64_t currentBits, uint64_t nextBits);
    void compute_queries(void *pStarks, void *pProof, uint64_t *friQueries, uint64_t nQueries, uint64_t nTrees);
    void compute_fri_queries(void *pStarks, void *pProof, uint64_t *friQueries, uint64_t nQueries, uint64_t step, uint64_t currentBits);
    void set_fri_final_pol(void *pProof, void *buffer, uint64_t nBits);

    // Transcript
    // =================================================================================
    void *transcript_new(uint32_t elementType, uint64_t arity, bool custom);
    void transcript_add(void *pTranscript, void *pInput, uint64_t size);
    void transcript_add_polinomial(void *pTranscript, void *pPolinomial);
    void transcript_free(void *pTranscript, uint32_t elementType);
    void get_challenge(void *pStarks, void *pTranscript, void *pElement);
    void get_permutations(void *pTranscript, uint64_t *res, uint64_t n, uint64_t nBits);

    // Constraints
    // =================================================================================
    uint64_t get_n_constraints(void *pSetupCtx);
    void get_constraints_lines_sizes(void* pSetupCtx, uint64_t *constraintsLinesSizes);
    void get_constraints_lines(void* pSetupCtx, uint8_t **constraintsLines);
    void verify_constraints(void *pSetupCtx, void* stepsParams, void* constraintsInfo);

    // Global constraints
    // =================================================================================
    uint64_t get_n_global_constraints(void* p_globalinfo_bin);
    void get_global_constraints_lines_sizes(void* p_globalinfo_bin, uint64_t *constraintsLinesSizes);
    void get_global_constraints_lines(void* p_globalinfo_bin, uint8_t **constraintsLines);
    void verify_global_constraints(char* globalInfoFile, void *globalBin, void *publics, void* challenges, void *proofValues, void **airgroupValues, void* globalConstraintsInfo);
    uint64_t get_hint_field_global_constraints_values(void* p_globalinfo_bin, uint64_t hintId, char* hintFieldName);
    void get_hint_field_global_constraints_sizes(char* globalInfoFile, void* p_globalinfo_bin, void* hintFieldValues, uint64_t hintId, char *hintFieldName, bool print_expression);
    void get_hint_field_global_constraints(char* globalInfoFile, void* p_globalinfo_bin, void* hintFieldValues, void *publics, void *challenges, void *proofValues, void **airgroupValues, uint64_t hintId, char *hintFieldName, bool print_expression);
    uint64_t set_hint_field_global_constraints(char* globalInfoFile, void* p_globalinfo_bin, void *proofValues, void *values, uint64_t hintId, char *hintFieldName);
    
    // Debug functions
    // =================================================================================
    void print_row(void *pSetupCtx, void *buffer, uint64_t stage, uint64_t row);

    // Recursive proof
    // =================================================================================
    void *gen_recursive_proof(void *pSetupCtx, char* globalInfoFile, uint64_t airgroupId, void* witness, void* aux_trace, void *pConstPols, void *pConstTree, void* pPublicInputs, char *proof_file, bool vadcop);
    void *get_zkin_ptr(char *zkin_file);
    void *add_recursive2_verkey(void *pZkin, char* recursive2VerKeyFilename);
    void *join_zkin_recursive2(char* globalInfoFile, uint64_t airgroupId, void* pPublics, void* pChallenges, void *zkin1, void *zkin2, void *starkInfoRecursive2);
    void *join_zkin_final(void* pPublics, void *pProofValues, void* pChallenges, char* globalInfoFile, void **zkinRecursive2, void **starkInfoRecursive2);
    char *get_serialized_proof(void *zkin, uint64_t* size);
    void *deserialize_zkin_proof(char* serialized_proof);
    void *get_zkin_proof(char* zkin);
    void zkin_proof_free(void *pZkinProof);
    void serialized_proof_free(char *zkinCStr);
    void get_committed_pols(void *circomWitness, char* execFile, void *witness, void* pPublics, uint64_t sizeWitness, uint64_t N, uint64_t nPublics, uint64_t nCols);

    // Final proof
    // =================================================================================
    void gen_final_snark_proof(void *circomWitnessFinal, char* zkeyFile, char* outputDir);

    // Util calls
    // =================================================================================
    void setLogLevel(uint64_t level);

    // Stark Verify
    // =================================================================================
    bool stark_verify(void* jProof, void *pStarkInfo, void *pExpressionsBin, char *verkey, void *pPublics, void *pProofValues, void *challenges);

    // Debug circom
    // =================================================================================
    void save_to_file(void *buffer, uint64_t bufferSize, void* publics, uint64_t publicsSize, char* name);
    void read_from_file(void* buffer, uint64_t bufferSize, void* publics, uint64_t publicsSize, char* name);

    void *create_buffer(uint64_t size);
    void free_buffer(void *buffer);
    
#endif