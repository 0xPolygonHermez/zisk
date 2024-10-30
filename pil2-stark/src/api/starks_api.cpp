#include "zkglobals.hpp"
#include "proof2zkinStark.hpp"
#include "starks.hpp"
#include "verify_constraints.hpp"
#include "hints.hpp"
#include "global_constraints.hpp"
#include "gen_recursive_proof.hpp"
#include "logger.hpp"
#include <filesystem>

#include <nlohmann/json.hpp>
using json = nlohmann::json;
using ordered_json = nlohmann::ordered_json;

using namespace CPlusPlusLogging;

void save_challenges(void *pChallenges, char* globalInfoFile, char *fileDir) {

    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    Goldilocks::Element *challenges = (Goldilocks::Element *)pChallenges;
    
    ordered_json challengesJson = challenges2proof(globalInfo, challenges);

    json2file(challengesJson, string(fileDir) + "/challenges.json");
}


void save_publics(unsigned long numPublicInputs, void *pPublicInputs, char *fileDir) {

    Goldilocks::Element* publicInputs = (Goldilocks::Element *)pPublicInputs;

    // Generate publics
    ordered_json publicStarkJson;
    for (uint64_t i = 0; i < numPublicInputs; i++)
    {
        publicStarkJson[i] = Goldilocks::toString(publicInputs[i]);
    }

    // save publics to filestarks
    json2file(publicStarkJson, string(fileDir) + "/publics.json");
}

void save_proof_values(unsigned long numProofValues, void *pProofValues, char *fileDir) {
    Goldilocks::Element* proofValues = (Goldilocks::Element *)pProofValues;

    ordered_json proofValuesJson;
    for(uint64_t i = 0; i < numProofValues; i++) {
        proofValuesJson[i] = ordered_json::array();
        for(uint64_t j = 0; j < FIELD_EXTENSION; ++j) {
            proofValuesJson[i][j] = Goldilocks::toString(proofValues[i*FIELD_EXTENSION + j]);
        }
    }

    json2file(proofValuesJson, string(fileDir) + "/proof_values.json");
}



void *fri_proof_new(void *pSetupCtx)
{
    SetupCtx setupCtx = *(SetupCtx *)pSetupCtx;
    FRIProof<Goldilocks::Element> *friProof = new FRIProof<Goldilocks::Element>(setupCtx.starkInfo);

    return friProof;
}


void fri_proof_get_tree_root(void *pFriProof, void* root, uint64_t tree_index)
{
    Goldilocks::Element *rootGL = (Goldilocks::Element *)root;
    FRIProof<Goldilocks::Element> *friProof = (FRIProof<Goldilocks::Element> *)pFriProof;
    for(uint64_t i = 0; i < friProof->proof.fri.treesFRI[tree_index].nFieldElements; ++i) {
        rootGL[i] = friProof->proof.fri.treesFRI[tree_index].root[i];
    }
}

void fri_proof_set_airgroupvalues(void *pFriProof, void *airgroupValues)
{
    FRIProof<Goldilocks::Element> *friProof = (FRIProof<Goldilocks::Element> *)pFriProof;
    friProof->proof.setAirgroupValues((Goldilocks::Element *)airgroupValues);
}

void fri_proof_set_airvalues(void *pFriProof, void *airValues)
{
    FRIProof<Goldilocks::Element> *friProof = (FRIProof<Goldilocks::Element> *)pFriProof;
    friProof->proof.setAirValues((Goldilocks::Element *)airValues);
}
void *fri_proof_get_zkinproof(uint64_t proof_id, void *pFriProof, void* pPublics, void* pChallenges, void *pStarkInfo, char* globalInfoFile, char *fileDir)
{
    json globalInfo;
    file2json(globalInfoFile, globalInfo);
    
    auto starkInfo = *((StarkInfo *)pStarkInfo);
    FRIProof<Goldilocks::Element> *friProof = (FRIProof<Goldilocks::Element> *)pFriProof;
    nlohmann::ordered_json jProof = friProof->proof.proof2json();
    nlohmann::ordered_json zkin = proof2zkinStark(jProof, starkInfo);

    Goldilocks::Element *publics = (Goldilocks::Element *)pPublics;
    Goldilocks::Element *challenges = (Goldilocks::Element *)pChallenges;

    for (uint64_t i = 0; i < starkInfo.nPublics; i++)
    {
        zkin["publics"][i] = Goldilocks::toString(publics[i]);
    }

    ordered_json challengesJson = challenges2zkin(globalInfo, challenges);
    zkin["challenges"] = challengesJson["challenges"];
    zkin["challengesFRISteps"] = challengesJson["challengesFRISteps"];

    // Save output to file
    if(!string(fileDir).empty()) {
        if (!std::filesystem::exists(string(fileDir) + "/zkin")) {
            std::filesystem::create_directory(string(fileDir) + "/zkin");
        }
        if (!std::filesystem::exists(string(fileDir) + "/proofs")) {
            std::filesystem::create_directory(string(fileDir) + "/proofs");
        }
        json2file(jProof, string(fileDir) + "/proofs/proof_" + to_string(proof_id) + ".json");
        json2file(zkin, string(fileDir) + "/zkin/proof_" + to_string(proof_id) + "_zkin.json");
    }

    return (void *) new nlohmann::ordered_json(zkin);    
}
void fri_proof_free_zkinproof(void *pZkinProof){
    nlohmann::ordered_json* zkin = (nlohmann::ordered_json*) pZkinProof;
    delete zkin;
}

void fri_proof_free(void *pFriProof)
{
    FRIProof<Goldilocks::Element> *friProof = (FRIProof<Goldilocks::Element> *)pFriProof;
    delete friProof;
}

// SetupCtx
// ========================================================================================

void *setup_ctx_new(void* p_stark_info, void* p_expression_bin, void* p_const_pols) {
    SetupCtx *setupCtx = new SetupCtx(*(StarkInfo*)p_stark_info, *(ExpressionsBin*)p_expression_bin, *(ConstPols *)p_const_pols);
    return setupCtx;
}

void* get_hint_ids_by_name(void *p_expression_bin, char* hintName)
{
    ExpressionsBin *expressionsBin = (ExpressionsBin*)p_expression_bin;

    VecU64Result hintIds = expressionsBin->getHintIdsByName(string(hintName));
    return new VecU64Result(hintIds);
}

void setup_ctx_free(void *pSetupCtx) {
    SetupCtx *setupCtx = (SetupCtx *)pSetupCtx;
    delete setupCtx;
}

// StarkInfo
// ========================================================================================
void *stark_info_new(char *filename)
{
    auto starkInfo = new StarkInfo(filename);

    return starkInfo;
}

uint64_t get_map_total_n(void *pStarkInfo)
{
    return ((StarkInfo *)pStarkInfo)->mapTotalN;
}

uint64_t get_n_airvals(void *pStarkInfo) {
    return ((StarkInfo *)pStarkInfo)->airValuesMap.size();
}

uint64_t get_n_airgroupvals(void *pStarkInfo) {
    return ((StarkInfo *)pStarkInfo)->airgroupValuesMap.size();
}

uint64_t get_n_evals(void *pStarkInfo) {
    return ((StarkInfo *)pStarkInfo)->evMap.size();
}

int64_t get_airvalue_id_by_name(void *pStarkInfo, char* airValueName) {
    auto starkInfo = *(StarkInfo *)pStarkInfo;
    for(uint64_t i = 0; i < starkInfo.airValuesMap.size(); ++i) {
        if(starkInfo.airValuesMap[i].name == string(airValueName)) return i;
    }
    return -1;
}

int64_t get_airgroupvalue_id_by_name(void *pStarkInfo, char* airgroupValueName) {
    auto starkInfo = *(StarkInfo *)pStarkInfo;
    for(uint64_t i = 0; i < starkInfo.airgroupValuesMap.size(); ++i) {
        if(starkInfo.airgroupValuesMap[i].name == string(airgroupValueName)) return i;
    }
    return -1;
};

uint64_t get_stark_info_n(void *pStarkInfo) {
    uint64_t N = 1 << ((StarkInfo *)pStarkInfo)->starkStruct.nBits;
    return N;
}

uint64_t get_stark_info_n_publics(void *pStarkInfo) {
    return ((StarkInfo *)pStarkInfo)->nPublics;
}

uint64_t get_map_offsets(void *pStarkInfo, char *stage, bool flag)
{
    auto starkInfo = (StarkInfo *)pStarkInfo;
    return starkInfo->mapOffsets[std::make_pair(stage, flag)];
}

void stark_info_free(void *pStarkInfo)
{
    auto starkInfo = (StarkInfo *)pStarkInfo;
    delete starkInfo;
}

// Const Pols
// ========================================================================================
void *const_pols_new(char* filename, void *pStarkInfo, bool calculate_tree) 
{
    auto const_pols = new ConstPols(*(StarkInfo *)pStarkInfo, filename, calculate_tree);

    return const_pols;
}

void *const_pols_with_tree_new(char* filename, char* treeFilename, void *pStarkInfo) 
{
    auto const_pols = new ConstPols(*(StarkInfo *)pStarkInfo, filename, treeFilename);

    return const_pols;
}

void load_const_tree(void *pConstPols, void *pStarkInfo, char *treeFilename) {
    ConstPols *constPols = (ConstPols *)pConstPols;
    constPols->loadConstTree(*(StarkInfo *)pStarkInfo, treeFilename);
}

void calculate_const_tree(void *pConstPols, void *pStarkInfo) {
    ConstPols *constPols = (ConstPols *)pConstPols;
    constPols->calculateConstTree(*(StarkInfo *)pStarkInfo);
}

void const_pols_free(void *pConstPols)
{
    auto constPols = (ConstPols *)pConstPols;
    delete constPols;
}

// Expressions Bin
// ========================================================================================
void *expressions_bin_new(char* filename, bool global)
{
    auto expressionsBin = new ExpressionsBin(filename, global);

    return expressionsBin;
};
void expressions_bin_free(void *pExpressionsBin)
{
    auto expressionsBin = (ExpressionsBin *)pExpressionsBin;
    delete expressionsBin;
};

// Hints
// ========================================================================================
void *get_hint_field(void *pSetupCtx, void* stepsParams, uint64_t hintId, char* hintFieldName, void* hintOptions) 
{
    HintFieldValues hintFieldValues = getHintField(*(SetupCtx *)pSetupCtx, *(StepsParams *)stepsParams, hintId, string(hintFieldName), *(HintFieldOptions *) hintOptions);
    return new HintFieldValues(hintFieldValues);
}

uint64_t mul_hint_fields(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldName1, char *hintFieldName2, void* hintOptions1, void *hintOptions2) 
{
    return multiplyHintFields(*(SetupCtx *)pSetupCtx, *(StepsParams *)stepsParams, hintId, string(hintFieldNameDest), string(hintFieldName1), string(hintFieldName2), *(HintFieldOptions *)hintOptions1,  *(HintFieldOptions *)hintOptions2);
}

void *acc_hint_field(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldNameAirgroupVal, char *hintFieldName) {
    return new VecU64Result(accHintField(*(SetupCtx *)pSetupCtx, *(StepsParams *)stepsParams, hintId, string(hintFieldNameDest), string(hintFieldNameAirgroupVal), string(hintFieldName)));
}

void *acc_mul_hint_fields(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldNameAirgroupVal, char *hintFieldName1, char *hintFieldName2, void* hintOptions1, void *hintOptions2) {
    return new VecU64Result(accMulHintFields(*(SetupCtx *)pSetupCtx, *(StepsParams *)stepsParams, hintId, string(hintFieldNameDest), string(hintFieldNameAirgroupVal), string(hintFieldName1), string(hintFieldName2),*(HintFieldOptions *)hintOptions1,  *(HintFieldOptions *)hintOptions2));
}


uint64_t set_hint_field(void *pSetupCtx, void* params, void *values, uint64_t hintId, char * hintFieldName) 
{
    return setHintField(*(SetupCtx *)pSetupCtx,  *(StepsParams *)params, (Goldilocks::Element *)values, hintId, string(hintFieldName));
}

// Starks
// ========================================================================================

void *starks_new(void *pSetupCtx)
{
    return new Starks<Goldilocks::Element>(*(SetupCtx *)pSetupCtx);
}

void starks_free(void *pStarks)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    delete starks;
}

void treesGL_get_root(void *pStarks, uint64_t index, void *dst)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;

    starks->ffi_treesGL_get_root(index, (Goldilocks::Element *)dst);
}

void calculate_fri_polynomial(void *pStarks, void* stepsParams)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->calculateFRIPolynomial(*(StepsParams *)stepsParams);
}


void calculate_quotient_polynomial(void *pStarks, void* stepsParams)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->calculateQuotientPolynomial(*(StepsParams *)stepsParams);
}

void calculate_impols_expressions(void *pStarks, uint64_t step, void* stepsParams)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->calculateImPolsExpressions(step, *(StepsParams *)stepsParams);
}

void commit_stage(void *pStarks, uint32_t elementType, uint64_t step, void *buffer, void *pProof, void *pBuffHelper) {
    // type == 1 => Goldilocks
    // type == 2 => BN128
    switch (elementType)
    {
    case 1:
        ((Starks<Goldilocks::Element> *)pStarks)->commitStage(step, (Goldilocks::Element *)buffer, *(FRIProof<Goldilocks::Element> *)pProof, (Goldilocks::Element *)pBuffHelper);
        break;
    default:
        cerr << "Invalid elementType: " << elementType << endl;
        break;
    }
}

void compute_lev(void *pStarks, void *xiChallenge, void* LEv) {
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->computeLEv((Goldilocks::Element *)xiChallenge, (Goldilocks::Element *)LEv);
}

void compute_evals(void *pStarks, void *buffer, void *LEv, void *evals, void *pProof)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->computeEvals((Goldilocks::Element *)buffer, (Goldilocks::Element *)LEv, (Goldilocks::Element *)evals, *(FRIProof<Goldilocks::Element> *)pProof);
}

void calculate_xdivxsub(void *pStarks, void* xiChallenge, void *xDivXSub)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->calculateXDivXSub((Goldilocks::Element *)xiChallenge, (Goldilocks::Element *)xDivXSub);
}

void *get_fri_pol(void *pSetupCtx, void *buffer)
{
    SetupCtx setupCtx = *(SetupCtx *)pSetupCtx;
    auto pols = (Goldilocks::Element *)buffer;
    
    return &pols[setupCtx.starkInfo.mapOffsets[std::make_pair("f", true)]];
}

void calculate_hash(void *pStarks, void *pHhash, void *pBuffer, uint64_t nElements)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->calculateHash((Goldilocks::Element *)pHhash, (Goldilocks::Element *)pBuffer, nElements);
}

void set_const_tree(void *pStarks, void *pConstPols) 
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->setConstTree(*(ConstPols *)pConstPols);
}

// MerkleTree
// =================================================================================
void *merkle_tree_new(uint64_t height, uint64_t width, uint64_t arity, bool custom) {
    MerkleTreeGL * mt =  new MerkleTreeGL(arity, custom, height, width, NULL);
    return mt;
}

void merkle_tree_free(void *pMerkleTree) {
    MerkleTreeGL *merkleTree = (MerkleTreeGL *)pMerkleTree;
    delete merkleTree;
}

// FRI
// =================================================================================

void compute_fri_folding(uint64_t step, void *buffer, void *pChallenge, uint64_t nBitsExt, uint64_t prevBits, uint64_t currentBits)
{
    FRI<Goldilocks::Element>::fold(step, (Goldilocks::Element *)buffer, (Goldilocks::Element *)pChallenge, nBitsExt, prevBits, currentBits);
}

void compute_fri_merkelize(void *pStarks, void *pProof, uint64_t step, void *buffer, uint64_t currentBits, uint64_t nextBits)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    FRI<Goldilocks::Element>::merkelize(step, *(FRIProof<Goldilocks::Element> *)pProof, (Goldilocks::Element *)buffer, starks->treesFRI[step], currentBits, nextBits);
}

void compute_queries(void *pStarks, void *pProof, uint64_t *friQueries, uint64_t nQueries, uint64_t nTrees)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    FRI<Goldilocks::Element>::proveQueries(friQueries, nQueries, *(FRIProof<Goldilocks::Element> *)pProof, starks->treesGL, nTrees);
}

void compute_fri_queries(void *pStarks, void *pProof, uint64_t *friQueries, uint64_t nQueries, uint64_t step, uint64_t currentBits)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    FRI<Goldilocks::Element>::proveFRIQueries(friQueries, nQueries, step, currentBits, *(FRIProof<Goldilocks::Element> *)pProof, starks->treesFRI[step - 1]);
}

void set_fri_final_pol(void *pProof, void *buffer, uint64_t nBits) {
    FRI<Goldilocks::Element>::setFinalPol(*(FRIProof<Goldilocks::Element> *)pProof, (Goldilocks::Element *)buffer, nBits);
}

// Transcript
// =================================================================================
void *transcript_new(uint32_t elementType, uint64_t arity, bool custom)
{
    // type == 1 => Goldilocks
    // type == 2 => BN128
    switch (elementType)
    {
    case 1:
        return new TranscriptGL(arity, custom);
    case 2:
        return new TranscriptBN128(arity, custom);
    default:
        return NULL;
    }
}

void transcript_add(void *pTranscript, void *pInput, uint64_t size)
{
    auto transcript = (TranscriptGL *)pTranscript;
    auto input = (Goldilocks::Element *)pInput;

    transcript->put(input, size);
}

void transcript_add_polinomial(void *pTranscript, void *pPolinomial)
{
    auto transcript = (TranscriptGL *)pTranscript;
    auto pol = (Polinomial *)pPolinomial;

    for (uint64_t i = 0; i < pol->degree(); i++)
    {
        transcript->put(pol->operator[](i), pol->dim());
    }
}

void transcript_free(void *pTranscript, uint32_t elementType)
{
    switch (elementType)
    {
    case 1:
        delete (TranscriptGL *)pTranscript;
        break;
    case 2:
        delete (TranscriptBN128 *)pTranscript;
        break;
    }
}

void get_challenge(void *pStarks, void *pTranscript, void *pElement)
{
    TranscriptGL *transcript = (TranscriptGL *)pTranscript;
    ((Starks<Goldilocks::Element> *)pStarks)->getChallenge(*transcript, *(Goldilocks::Element *)pElement);
}

void get_permutations(void *pTranscript, uint64_t *res, uint64_t n, uint64_t nBits)
{
    TranscriptGL *transcript = (TranscriptGL *)pTranscript;
    transcript->getPermutations(res, n, nBits);
}

// Constraints
// =================================================================================
void *verify_constraints(void *pSetupCtx, void* stepsParams)
{
    ConstraintsResults *constraintsInfo = verifyConstraints(*(SetupCtx *)pSetupCtx, *(StepsParams *)stepsParams);
    return constraintsInfo;
}

// Global Constraints
// =================================================================================
bool verify_global_constraints(void* p_globalinfo_bin, void *publics, void *proofValues, void **airgroupValues) {
    return verifyGlobalConstraints(*(ExpressionsBin*)p_globalinfo_bin, (Goldilocks::Element *)publics, (Goldilocks::Element *)proofValues, (Goldilocks::Element **)airgroupValues);
}

void *get_hint_field_global_constraints(void* p_globalinfo_bin, void *publics, void *proofValues, void **airgroupValues, uint64_t hintId, char *hintFieldName, bool print_expression) 
{
    HintFieldValues hintFieldValues = getHintFieldGlobalConstraint(*(ExpressionsBin*)p_globalinfo_bin, (Goldilocks::Element *)publics, (Goldilocks::Element *)proofValues, (Goldilocks::Element **)airgroupValues, hintId, string(hintFieldName), print_expression);
    return new HintFieldValues(hintFieldValues);
}

uint64_t set_hint_field_global_constraints(void* p_globalinfo_bin, void *proofValues, void *values, uint64_t hintId, char *hintFieldName) 
{
    return setHintFieldGlobalConstraint(*(ExpressionsBin*)p_globalinfo_bin, (Goldilocks::Element *)proofValues, (Goldilocks::Element *)values, hintId, string(hintFieldName));
}

// Debug functions
// =================================================================================  

void *print_by_name(void *pSetupCtx, void* pStepsParams, char* name, uint64_t *lengths, uint64_t first_value, uint64_t last_value, bool return_values) {
    HintFieldInfo hintFieldInfo = printByName(*(SetupCtx *)pSetupCtx, *(StepsParams *)pStepsParams, string(name), lengths, first_value, last_value, return_values);
    return new HintFieldInfo(hintFieldInfo);
}

void print_expression(void *pSetupCtx, void* pol, uint64_t dim, uint64_t first_value, uint64_t last_value) {
    printExpression((Goldilocks::Element *)pol, dim, first_value, last_value);
}

void print_row(void *pSetupCtx, void *buffer, uint64_t stage, uint64_t row) {
    printRow(*(SetupCtx *)pSetupCtx, (Goldilocks::Element *)buffer, stage, row);
}

// Recursive proof
// ================================================================================= 
void *gen_recursive_proof(void *pSetupCtx, void* pAddress, void* pPublicInputs, char* proof_file) {
    return genRecursiveProof<Goldilocks::Element>(*(SetupCtx *)pSetupCtx, (Goldilocks::Element *)pAddress,  (Goldilocks::Element *)pPublicInputs, string(proof_file));
}

void *get_zkin_ptr(char *zkin_file) {
    json zkin;
    file2json(zkin_file, zkin);

    return (void *) new nlohmann::ordered_json(zkin);
}

void *public2zkin(void *pZkin, void* pPublics, char* globalInfoFile, uint64_t airgroupId, bool isAggregated) {
    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    nlohmann::ordered_json zkin = *(nlohmann::ordered_json*) pZkin;
    return publics2zkin(zkin, (Goldilocks::Element *)pPublics, globalInfo, airgroupId, isAggregated);
}

void *add_recursive2_verkey(void *pZkin, char* recursive2VerKeyFilename) {
    json recursive2VerkeyJson;
    file2json(recursive2VerKeyFilename, recursive2VerkeyJson);

    Goldilocks::Element recursive2Verkey[4];
    for (uint64_t i = 0; i < 4; i++)
    {
        recursive2Verkey[i] = Goldilocks::fromU64(recursive2VerkeyJson[i]);
    }

    nlohmann::ordered_json zkin = *(nlohmann::ordered_json*) pZkin;
    return addRecursive2VerKey(zkin, recursive2Verkey);
}

void *join_zkin_recursive2(char* globalInfoFile, uint64_t airgroupId, void* pPublics, void* pChallenges, void *zkin1, void *zkin2, void *starkInfoRecursive2) {
    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    Goldilocks::Element *publics = (Goldilocks::Element *)pPublics;
    Goldilocks::Element *challenges = (Goldilocks::Element *)pChallenges;

    ordered_json zkinRecursive2 = joinzkinrecursive2(globalInfo, airgroupId, publics, challenges, *(nlohmann::ordered_json *)zkin1, *(nlohmann::ordered_json *)zkin2, *(StarkInfo *)starkInfoRecursive2);

    return (void *) new nlohmann::ordered_json(zkinRecursive2);
}

void *join_zkin_final(void* pPublics, void *pProofValues, void* pChallenges, char* globalInfoFile, void **zkinRecursive2, void **starkInfoRecursive2) {
    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    Goldilocks::Element *publics = (Goldilocks::Element *)pPublics;
    Goldilocks::Element *challenges = (Goldilocks::Element *)pChallenges;
    Goldilocks::Element *proofValues = (Goldilocks::Element *)pProofValues;

    ordered_json zkinFinal = joinzkinfinal(globalInfo, publics, proofValues, challenges, zkinRecursive2, starkInfoRecursive2);

    return (void *) new nlohmann::ordered_json(zkinFinal);    
}


void setLogLevel(uint64_t level) {
    LogLevel new_level;
    switch(level) {
        case 0:
            new_level = DISABLE_LOG;
            break;
        case 1:
        case 2:
        case 3:
            new_level = LOG_LEVEL_INFO;
            break;
        case 4:
            new_level = LOG_LEVEL_DEBUG;
            break;
        case 5:
            new_level = LOG_LEVEL_TRACE;
            break;
        default:
            cerr << "Invalid log level: " << level << endl;
            return;
    }

    Logger::getInstance(LOG_TYPE::CONSOLE)->updateLogLevel((LOG_LEVEL)new_level);
}