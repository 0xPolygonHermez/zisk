#ifndef LIB_API_H
#define LIB_API_H
#include <stdint.h>

    // Save Proof
    // ========================================================================================
    void save_challenges(void *pChallenges, char* globalInfoFile, char *fileDir);
    void save_publics(unsigned long numPublicInputs, void *pPublicInputs, char *fileDir);
    void save_proof_values(unsigned long numProofValues, void *pProofValues, char *fileDir);

    // FRIProof
    // ========================================================================================
    void *fri_proof_new(void *pSetupCtx);
    void fri_proof_get_tree_root(void *pFriProof, void* root, uint64_t tree_index);
    void fri_proof_set_airgroupvalues(void *pFriProof, void *airgroupValues);
    void fri_proof_set_airvalues(void *pFriProof, void *airValues);
    void *fri_proof_get_zkinproof(uint64_t proof_id, void *pFriProof, void* pPublics, void* pChallenges, void *pStarkInfo, char* globalInfoFile, char *fileDir);
    void fri_proof_free_zkinproof(void *pZkinProof);
    void fri_proof_free(void *pFriProof);

    // SetupCtx
    // ========================================================================================
    void *get_hint_ids_by_name(void *p_expression_bin, char* hintName);

    // Stark Info
    // ========================================================================================
    void *stark_info_new(char* filename);
    uint64_t get_stark_info_n(void *pStarkInfo);
    uint64_t get_stark_info_n_publics(void *pStarkInfo);
    uint64_t get_map_total_n(void *pStarkInfo);
    uint64_t get_custom_commit_id(void *pStarkInfo, char* name);
    uint64_t get_map_total_n_custom_commits(void *pStarkInfo, uint64_t commit_id);
    uint64_t get_map_offsets(void *pStarkInfo, char *stage, bool flag);
    uint64_t get_n_airvals(void *pStarkInfo);
    uint64_t get_n_airgroupvals(void *pStarkInfo);
    uint64_t get_n_evals(void *pStarkInfo);
    uint64_t get_n_custom_commits(void *pStarkInfo);
    int64_t get_airvalue_id_by_name(void *pStarkInfo, char* airValueName);
    int64_t get_airgroupvalue_id_by_name(void *pStarkInfo, char* airValueName);
    void *get_custom_commit_map_ids(void *pStarkInfo, uint64_t commit_id, uint64_t stage);
    void stark_info_free(void *pStarkInfo);

    // Prover Helpers
    // ========================================================================================
    void *prover_helpers_new(void *pStarkInfo);
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
    void *expressions_bin_new(char* filename, bool global);
    void expressions_bin_free(void *pExpressionsBin);

    // Hints
    // ========================================================================================
    void *get_hint_field(void *pSetupCtx, void* stepsParams, uint64_t hintId, char* hintFieldName, void* hintOptions);
    uint64_t mul_hint_fields(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldName1, char *hintFieldName2, void* hintOptions1, void *hintOptions2); 
    void *acc_hint_field(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldNameAirgroupVal, char *hintFieldName, bool add);
    void *acc_mul_hint_fields(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldNameAirgroupVal, char *hintFieldName1, char *hintFieldName2,  void* hintOptions1, void *hintOptions2, bool add);
    void *acc_mul_add_hint_fields(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldNameAirgroupVal, char *hintFieldName1, char *hintFieldName2, char *hintFieldName3, void* hintOptions1, void *hintOptions2, void *hintOptions3, bool add);
    uint64_t set_hint_field(void *pSetupCtx, void* stepsParams, void *values, uint64_t hintId, char* hintFieldName);

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

    void extend_and_merkelize_custom_commit(void *pStarks, uint64_t commitId, uint64_t step, void *buffer, void *pProof, void *pBuffHelper, char *treeFile);
    void load_custom_commit(void *pStarks, uint64_t commitId, uint64_t step, void *buffer, void *pProof, char *treeFile);

    void commit_stage(void *pStarks, uint32_t elementType, uint64_t step, void *buffer, void *pProof, void *pBuffHelper);
    
    void compute_lev(void *pStarks, void *xiChallenge, void* LEv);
    void compute_evals(void *pStarks, void *params, void *LEv, void *pProof);

    void calculate_hash(void *pStarks, void *pHhash, void *pBuffer, uint64_t nElements);

    
    // MerkleTree
    // =================================================================================
    void *merkle_tree_new(uint64_t height, uint64_t width, uint64_t arity, bool custom);
    void merkle_tree_free(void *pMerkleTree);

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
    void *verify_constraints(void *pSetupCtx, void* stepsParams);

    // Global constraints
    // =================================================================================
    bool verify_global_constraints(void *globalBin, void *publics, void* challenges, void *proofValues, void **airgroupValues);
    void *get_hint_field_global_constraints(void *globalBin, void *publics, void* challenges, void *proofValues, void **airgroupValues, uint64_t hintId, char *hintFieldName, bool print_expression);
    uint64_t set_hint_field_global_constraints(void* p_globalinfo_bin, void *proofValues, void *values, uint64_t hintId, char *hintFieldName);
    
    // Debug functions
    // =================================================================================
    void print_row(void *pSetupCtx, void *buffer, uint64_t stage, uint64_t row);
    void print_expression(void *pSetupCtx, void* pol, uint64_t dim, uint64_t first_value, uint64_t last_value);

    // Recursive proof
    // =================================================================================
    void *gen_recursive_proof(void *pSetupCtx, char* globalInfoFile, uint64_t airgroupId, void* pAddress, void *pConstPols, void *pConstTree, void* pPublicInputs, char *proof_file);
    void *get_zkin_ptr(char *zkin_file);
    void *add_recursive2_verkey(void *pZkin, char* recursive2VerKeyFilename);
    void *join_zkin_recursive2(char* globalInfoFile, uint64_t airgroupId, void* pPublics, void* pChallenges, void *zkin1, void *zkin2, void *starkInfoRecursive2);
    void *join_zkin_final(void* pPublics, void *pProofValues, void* pChallenges, char* globalInfoFile, void **zkinRecursive2, void **starkInfoRecursive2);
    char *get_serialized_proof(void *zkin, uint64_t* size);
    void *deserialize_zkin_proof(char* serialized_proof);
    void *get_zkin_proof(char* zkin);
    void zkin_proof_free(void *pZkinProof);
    void serialized_proof_free(char *zkinCStr);
    void get_committed_pols(void *pWitness, char* execFile, void *pAddress, void* pPublics, uint64_t sizeWitness, uint64_t N, uint64_t nPublics, uint64_t offsetCm1, uint64_t nCols);

    // Util calls
    // =================================================================================
    void setLogLevel(uint64_t level);

#endif