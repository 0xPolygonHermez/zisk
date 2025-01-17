#include "zkglobals.hpp"
#include "proof2zkinStark.hpp"
#include "starks.hpp"
#include "verify_constraints.hpp"
#include "hints.hpp"
#include "global_constraints.hpp"
#include "gen_recursive_proof.hpp"
#include "logger.hpp"
#include <filesystem>
#include "setup_ctx.hpp"
#include "stark_verify.hpp"
#include "exec_file.hpp"
#include "final_snark_proof.hpp"

#include <nlohmann/json.hpp>
using json = nlohmann::json;

using namespace CPlusPlusLogging;

void save_challenges(void *pChallenges, char* globalInfoFile, char *fileDir) {

    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    Goldilocks::Element *challenges = (Goldilocks::Element *)pChallenges;
    
    json challengesJson = challenges2proof(globalInfo, challenges);

    json2file(challengesJson, string(fileDir) + "/challenges.json");
}


void save_publics(unsigned long numPublicInputs, void *pPublicInputs, char *fileDir) {

    Goldilocks::Element* publicInputs = (Goldilocks::Element *)pPublicInputs;

    // Generate publics
    json publicStarkJson;
    for (uint64_t i = 0; i < numPublicInputs; i++)
    {
        publicStarkJson[i] = Goldilocks::toString(publicInputs[i]);
    }

    // save publics to filestarks
    json2file(publicStarkJson, string(fileDir) + "/publics.json");
}

void save_proof_values(void *pProofValues, char* globalInfoFile, char *fileDir) {
    Goldilocks::Element* proofValues = (Goldilocks::Element *)pProofValues;

    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    json proofValuesJson;
    uint64_t p = 0;
    for(uint64_t i = 0; i < globalInfo["proofValuesMap"].size(); i++) {
        proofValuesJson[i] = json::array();
        if(globalInfo["proofValuesMap"][i]["stage"] == 1) {
            proofValuesJson[i][0] = Goldilocks::toString(proofValues[p++]);
            proofValuesJson[i][1] = "0";
            proofValuesJson[i][2] = "0";
        } else {
            proofValuesJson[i][0] = Goldilocks::toString(proofValues[p++]);
            proofValuesJson[i][1] = Goldilocks::toString(proofValues[p++]);
            proofValuesJson[i][2] = Goldilocks::toString(proofValues[p++]);
        }
        
    }

    json2file(proofValuesJson, string(fileDir) + "/proof_values.json");
}



void *fri_proof_new(void *pSetupCtx, uint64_t instanceId)
{
    SetupCtx setupCtx = *(SetupCtx *)pSetupCtx;
    FRIProof<Goldilocks::Element> *friProof = new FRIProof<Goldilocks::Element>(setupCtx.starkInfo, instanceId);

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

void fri_proof_get_zkinproofs(uint64_t nProofs, void **proofs, void **pFriProofs, void* pPublics, void *pProofValues, void* pChallenges, char* globalInfoFile, char *fileDir) {
    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    Goldilocks::Element *publics = (Goldilocks::Element *)pPublics;
    Goldilocks::Element *challenges = (Goldilocks::Element *)pChallenges;
    Goldilocks::Element *proofValues = (Goldilocks::Element *)pProofValues;
    
    json j;
    for (uint64_t i = 0; i < globalInfo["nPublics"]; i++)
    {
        j["publics"][i] = Goldilocks::toString(publics[i]);
    }

    uint64_t p = 0;
    for (uint64_t i = 0; i < globalInfo["proofValuesMap"].size(); i++)
    {
        if(globalInfo["proofValuesMap"][i]["stage"] == 1) {
            j["proofvalues"][i][0] = Goldilocks::toString(proofValues[p++]);
            j["proofvalues"][i][1] = "0";
            j["proofvalues"][i][2] = "0";
        } else {
            j["proofvalues"][i][0] = Goldilocks::toString(proofValues[p++]);
            j["proofvalues"][i][1] = Goldilocks::toString(proofValues[p++]);
            j["proofvalues"][i][2] = Goldilocks::toString(proofValues[p++]);
        }
    }

    j["challenges"] = challenges2zkin(globalInfo, challenges);

    if(!string(fileDir).empty()) {
        if (!std::filesystem::exists(string(fileDir) + "/proofs")) {
            std::filesystem::create_directory(string(fileDir) + "/proofs");
        }
    }

    #pragma omp parallel for
    for(uint64_t i = 0; i < nProofs; ++i) {
        FRIProof<Goldilocks::Element> *friProof = (FRIProof<Goldilocks::Element> *)pFriProofs[i];
        nlohmann::json zkin = friProof->proof.proof2json();

        zkin["publics"] = j["publics"];
        zkin["proofvalues"] = j["proofvalues"];
        zkin["challenges"] = j["challenges"]["challenges"];
        zkin["challengesFRISteps"] = j["challenges"]["challengesFRISteps"];

        std::string airName = globalInfo["airs"][friProof->airgroupId][friProof->airId]["name"];
        std::string proofName = airName + "_" + std::to_string(friProof->instanceId);

        if(!string(fileDir).empty()) {
            json2file(zkin, string(fileDir) + "/zkin/proof_" + proofName + "_zkin.json");
        }

        proofs[i] = (void *) new nlohmann::json(zkin);
    }
}


void *fri_proof_get_zkinproof(void *pFriProof, void* pPublics, void* pChallenges, void *pProofValues, char* globalInfoFile, char *fileDir)
{
    json globalInfo;
    file2json(globalInfoFile, globalInfo);
    
    FRIProof<Goldilocks::Element> *friProof = (FRIProof<Goldilocks::Element> *)pFriProof;
    nlohmann::json zkin = friProof->proof.proof2json();

    Goldilocks::Element *publics = (Goldilocks::Element *)pPublics;
    Goldilocks::Element *challenges = (Goldilocks::Element *)pChallenges;
    Goldilocks::Element *proofValues = (Goldilocks::Element *)pProofValues;

    for (uint64_t i = 0; i < globalInfo["nPublics"]; i++)
    {
        zkin["publics"][i] = Goldilocks::toString(publics[i]);
    }

    uint64_t p = 0;
    for (uint64_t i = 0; i < globalInfo["proofValuesMap"].size(); i++)
    {
        if(globalInfo["proofValuesMap"][i]["stage"] == 1) {
            zkin["proofvalues"][i][0] = Goldilocks::toString(proofValues[p++]);
            zkin["proofvalues"][i][1] = "0";
            zkin["proofvalues"][i][2] = "0";
        } else {
            zkin["proofvalues"][i][0] = Goldilocks::toString(proofValues[p++]);
            zkin["proofvalues"][i][1] = Goldilocks::toString(proofValues[p++]);
            zkin["proofvalues"][i][2] = Goldilocks::toString(proofValues[p++]);
        }
    }

    json challengesJson = challenges2zkin(globalInfo, challenges);
    zkin["challenges"] = challengesJson["challenges"];
    zkin["challengesFRISteps"] = challengesJson["challengesFRISteps"];

    std::string airName = globalInfo["airs"][friProof->airgroupId][friProof->airId]["name"];
    std::string proofName = airName + "_" + std::to_string(friProof->instanceId);

    // Save output to file
    if(!string(fileDir).empty()) {
        if (!std::filesystem::exists(string(fileDir) + "/proofs")) {
            std::filesystem::create_directory(string(fileDir) + "/proofs");
        }
        json2file(zkin, string(fileDir) + "/proofs/proof_" + proofName + ".json");
    }

    return (void *) new nlohmann::json(zkin);    
}

void fri_proof_free_zkinproof(void *pZkinProof){
    nlohmann::json* zkin = (nlohmann::json*) pZkinProof;
    delete zkin;
}

void fri_proof_free(void *pFriProof)
{
    FRIProof<Goldilocks::Element> *friProof = (FRIProof<Goldilocks::Element> *)pFriProof;
    delete friProof;
}

void proofs_free(uint64_t nProofs, void **pStarks, void **pFriProofs, bool background) {

#pragma omp parallel for
    for (uint64_t i = 0; i < nProofs; ++i) {
        FRIProof<Goldilocks::Element> *friProof = (FRIProof<Goldilocks::Element> *)pFriProofs[i];
        Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks[i];

        delete friProof;
        delete starks;
    }
}


// SetupCtx
// ========================================================================================

uint64_t n_hints_by_name(void *p_expression_bin, char* hintName) {
    ExpressionsBin *expressionsBin = (ExpressionsBin*)p_expression_bin;
    return expressionsBin->getNumberHintIdsByName(string(hintName));
}

void get_hint_ids_by_name(void *p_expression_bin, uint64_t* hintIds, char* hintName)
{
    ExpressionsBin *expressionsBin = (ExpressionsBin*)p_expression_bin;
    expressionsBin->getHintIdsByName(hintIds, string(hintName));
}

// StarkInfo
// ========================================================================================
void *stark_info_new(char *filename, bool verify)
{
    auto starkInfo = new StarkInfo(filename, verify);

    return starkInfo;
}

uint64_t get_map_total_n(void *pStarkInfo, bool recursive)
{
    StarkInfo *starkInfo = (StarkInfo *)pStarkInfo;
    if(recursive) {
        starkInfo->addMemoryRecursive();
    }
    return starkInfo->mapTotalN;
}

void stark_info_free(void *pStarkInfo)
{
    auto starkInfo = (StarkInfo *)pStarkInfo;
    delete starkInfo;
}

// Prover Helpers
// ========================================================================================
void *prover_helpers_new(void *pStarkInfo, bool pil1) {
    auto prover_helpers = new ProverHelpers(*(StarkInfo *)pStarkInfo, pil1);
    return prover_helpers;
}

void prover_helpers_free(void *pProverHelpers) {
    auto proverHelpers = (ProverHelpers *)pProverHelpers;
    delete proverHelpers;
};

// Const Pols
// ========================================================================================
void load_const_tree(void *pConstTree, char *treeFilename, uint64_t constTreeSize) {
    ConstTree constTree;
    constTree.loadConstTree(pConstTree, treeFilename, constTreeSize);
};

void load_const_pols(void *pConstPols, char *constFilename, uint64_t constSize) {
    ConstTree constTree;
    constTree.loadConstPols(pConstPols, constFilename, constSize);
};

uint64_t get_const_tree_size(void *pStarkInfo) {
    ConstTree constTree;
    auto starkInfo = *(StarkInfo *)pStarkInfo;
    if(starkInfo.starkStruct.verificationHashType == "GL") {
        return constTree.getConstTreeSizeBytesGL(starkInfo);
    } else {
        return constTree.getConstTreeSizeBytesBN128(starkInfo);
    }
    
};

uint64_t get_const_size(void *pStarkInfo) {
    auto starkInfo = *(StarkInfo *)pStarkInfo;
    uint64_t N = 1 << starkInfo.starkStruct.nBits;
    return N * starkInfo.nConstants * sizeof(Goldilocks::Element);
}


void calculate_const_tree(void *pStarkInfo, void *pConstPolsAddress, void *pConstTreeAddress, char *treeFilename) {
    ConstTree constTree;
    auto starkInfo = *(StarkInfo *)pStarkInfo;
    if(starkInfo.starkStruct.verificationHashType == "GL") {
        constTree.calculateConstTreeGL(*(StarkInfo *)pStarkInfo, (Goldilocks::Element *)pConstPolsAddress, pConstTreeAddress, treeFilename);
    } else {
        constTree.calculateConstTreeBN128(*(StarkInfo *)pStarkInfo, (Goldilocks::Element *)pConstPolsAddress, pConstTreeAddress, treeFilename);
    }
};

// Expressions Bin
// ========================================================================================
void *expressions_bin_new(char* filename, bool global, bool verifier)
{
    auto expressionsBin = new ExpressionsBin(filename, global, verifier);

    return expressionsBin;
};
void expressions_bin_free(void *pExpressionsBin)
{
    auto expressionsBin = (ExpressionsBin *)pExpressionsBin;
    delete expressionsBin;
};

// Hints
// ========================================================================================
void get_hint_field(void *pSetupCtx, void* stepsParams, void* hintFieldValues, uint64_t hintId, char* hintFieldName, void* hintOptions) 
{
    getHintField(*(SetupCtx *)pSetupCtx, *(StepsParams *)stepsParams, (HintFieldInfo *) hintFieldValues, hintId, string(hintFieldName), *(HintFieldOptions *) hintOptions);
}

uint64_t get_hint_field_values(void *pSetupCtx, uint64_t hintId, char* hintFieldName) {
    return getHintFieldValues(*(SetupCtx *)pSetupCtx, hintId, string(hintFieldName));
}

void get_hint_field_sizes(void *pSetupCtx, void* hintFieldValues, uint64_t hintId, char* hintFieldName, void* hintOptions)
{
    getHintFieldSizes(*(SetupCtx *)pSetupCtx, (HintFieldInfo *) hintFieldValues, hintId, string(hintFieldName), *(HintFieldOptions *) hintOptions);
}

uint64_t mul_hint_fields(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldName1, char *hintFieldName2, void* hintOptions1, void *hintOptions2) 
{
    return multiplyHintFields(*(SetupCtx *)pSetupCtx, *(StepsParams *)stepsParams, hintId, string(hintFieldNameDest), string(hintFieldName1), string(hintFieldName2), *(HintFieldOptions *)hintOptions1,  *(HintFieldOptions *)hintOptions2);
}

void acc_hint_field(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldNameAirgroupVal, char *hintFieldName, bool add) {
    accHintField(*(SetupCtx *)pSetupCtx, *(StepsParams *)stepsParams, hintId, string(hintFieldNameDest), string(hintFieldNameAirgroupVal), string(hintFieldName), add);
}

void acc_mul_hint_fields(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameDest, char *hintFieldNameAirgroupVal, char *hintFieldName1, char *hintFieldName2, void* hintOptions1, void *hintOptions2, bool add) {
    accMulHintFields(*(SetupCtx *)pSetupCtx, *(StepsParams *)stepsParams, hintId, string(hintFieldNameDest), string(hintFieldNameAirgroupVal), string(hintFieldName1), string(hintFieldName2),*(HintFieldOptions *)hintOptions1,  *(HintFieldOptions *)hintOptions2, add);
}

uint64_t update_airgroupvalue(void *pSetupCtx, void* stepsParams, uint64_t hintId, char *hintFieldNameAirgroupVal, char *hintFieldName1, char *hintFieldName2, void* hintOptions1, void *hintOptions2, bool add) {
    return updateAirgroupValue(*(SetupCtx *)pSetupCtx, *(StepsParams *)stepsParams, hintId, string(hintFieldNameAirgroupVal), string(hintFieldName1), string(hintFieldName2),*(HintFieldOptions *)hintOptions1,  *(HintFieldOptions *)hintOptions2, add);
}

uint64_t get_hint_id(void *pSetupCtx, uint64_t hintId, char * hintFieldName) {
    return getHintId(*(SetupCtx *)pSetupCtx, hintId, string(hintFieldName));
}

uint64_t set_hint_field(void *pSetupCtx, void* params, void *values, uint64_t hintId, char * hintFieldName) 
{
    return setHintField(*(SetupCtx *)pSetupCtx,  *(StepsParams *)params, (Goldilocks::Element *)values, hintId, string(hintFieldName));
}

// Starks
// ========================================================================================

void *starks_new(void *pSetupCtx, void* pConstTree)
{
    return new Starks<Goldilocks::Element>(*(SetupCtx *)pSetupCtx, (Goldilocks::Element*) pConstTree);
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

void treesGL_set_root(void *pStarks, uint64_t index, void *pProof)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;

    starks->ffi_treesGL_set_root(index, *(FRIProof<Goldilocks::Element> *)pProof);
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

void extend_and_merkelize_custom_commit(void *pStarks, uint64_t commitId, uint64_t step, void *buffer, void* bufferExt, void *pProof, void *pBuffHelper, char *bufferFile)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->extendAndMerkelizeCustomCommit(commitId, step, (Goldilocks::Element *)buffer, (Goldilocks::Element *)bufferExt, *(FRIProof<Goldilocks::Element> *)pProof, (Goldilocks::Element *)pBuffHelper, string(bufferFile));
}

void load_custom_commit(void *pStarks, uint64_t commitId, uint64_t step, void *buffer, void *bufferExt, void *pProof, char *bufferFile)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->loadCustomCommit(commitId, step, (Goldilocks::Element *)buffer, (Goldilocks::Element *)bufferExt, *(FRIProof<Goldilocks::Element> *)pProof, string(bufferFile));
}

void commit_stage(void *pStarks, uint32_t elementType, uint64_t step, void *trace, void *buffer, void *pProof, void *pBuffHelper) {
    // type == 1 => Goldilocks
    // type == 2 => BN128
    switch (elementType)
    {
    case 1:
        ((Starks<Goldilocks::Element> *)pStarks)->commitStage(step, (Goldilocks::Element *)trace, (Goldilocks::Element *)buffer, *(FRIProof<Goldilocks::Element> *)pProof, (Goldilocks::Element *)pBuffHelper);
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

void compute_evals(void *pStarks, void *params, void *LEv, void *pProof)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->computeEvals(*(StepsParams *)params, (Goldilocks::Element *)LEv, *(FRIProof<Goldilocks::Element> *)pProof);
}

void calculate_xdivxsub(void *pStarks, void* xiChallenge, void *xDivXSub)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->calculateXDivXSub((Goldilocks::Element *)xiChallenge, (Goldilocks::Element *)xDivXSub);
}

void *get_fri_pol(void *pStarkInfo, void *buffer)
{
    StarkInfo starkInfo = *(StarkInfo *)pStarkInfo;
    auto pols = (Goldilocks::Element *)buffer;
    
    return &pols[starkInfo.mapOffsets[std::make_pair("f", true)]];
}

void calculate_hash(void *pStarks, void *pHhash, void *pBuffer, uint64_t nElements)
{
    Starks<Goldilocks::Element> *starks = (Starks<Goldilocks::Element> *)pStarks;
    starks->calculateHash((Goldilocks::Element *)pHhash, (Goldilocks::Element *)pBuffer, nElements);
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
uint64_t get_n_constraints(void *pSetupCtx)
{
    auto setupCtx = *(SetupCtx *)pSetupCtx;
    return setupCtx.expressionsBin.constraintsInfoDebug.size();
}

void get_constraints_lines_sizes(void* pSetupCtx, uint64_t *constraintsLinesSizes)
{
    auto setupCtx = *(SetupCtx *)pSetupCtx;
    for(uint64_t i = 0; i < setupCtx.expressionsBin.constraintsInfoDebug.size(); ++i) {
        constraintsLinesSizes[i] = setupCtx.expressionsBin.constraintsInfoDebug[i].line.size();
    }
}

void get_constraints_lines(void* pSetupCtx, uint8_t **constraintsLines)
{
    auto setupCtx = *(SetupCtx *)pSetupCtx;
    for(uint64_t i = 0; i < setupCtx.expressionsBin.constraintsInfoDebug.size(); ++i) {
        std::memcpy(constraintsLines[i], setupCtx.expressionsBin.constraintsInfoDebug[i].line.data(), setupCtx.expressionsBin.constraintsInfoDebug[i].line.size());
    }
}

void verify_constraints(void *pSetupCtx, void* stepsParams, void* constraintsInfo)
{
    verifyConstraints(*(SetupCtx *)pSetupCtx, *(StepsParams *)stepsParams, (ConstraintInfo *)constraintsInfo);
}

// Global Constraints
// =================================================================================
uint64_t get_n_global_constraints(void* p_globalinfo_bin)
{
    return getNumberGlobalConstraints(*(ExpressionsBin*)p_globalinfo_bin);
}

void get_global_constraints_lines_sizes(void* p_globalinfo_bin, uint64_t *constraintsLinesSizes)
{
    return getGlobalConstraintsLinesSizes(*(ExpressionsBin*)p_globalinfo_bin, constraintsLinesSizes);
}

void get_global_constraints_lines(void* p_globalinfo_bin, uint8_t **constraintsLines)
{
    return getGlobalConstraintsLines(*(ExpressionsBin*)p_globalinfo_bin, constraintsLines);
}

void verify_global_constraints(char* globalInfoFile, void* p_globalinfo_bin, void *publics, void *challenges, void *proofValues, void **airgroupValues, void *globalConstraintsInfo) {
    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    verifyGlobalConstraints(globalInfo, *(ExpressionsBin*)p_globalinfo_bin, (Goldilocks::Element *)publics, (Goldilocks::Element *)challenges, (Goldilocks::Element *)proofValues, (Goldilocks::Element **)airgroupValues, (GlobalConstraintInfo *)globalConstraintsInfo);
}
 
uint64_t get_hint_field_global_constraints_values(void* p_globalinfo_bin, uint64_t hintId, char* hintFieldName) {
    return getHintFieldGlobalConstraintValues(*(ExpressionsBin*)p_globalinfo_bin, hintId, string(hintFieldName));
}

void get_hint_field_global_constraints_sizes(char* globalInfoFile, void* p_globalinfo_bin, void* hintFieldValues, uint64_t hintId, char *hintFieldName, bool print_expression)
{
    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    getHintFieldGlobalConstraintSizes(globalInfo, *(ExpressionsBin*)p_globalinfo_bin, (HintFieldInfo *)hintFieldValues, hintId, string(hintFieldName), print_expression);
}


void get_hint_field_global_constraints(char* globalInfoFile, void* p_globalinfo_bin, void* hintFieldValues, void *publics, void *challenges, void *proofValues, void **airgroupValues, uint64_t hintId, char *hintFieldName, bool print_expression) 
{
    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    getHintFieldGlobalConstraint(globalInfo, *(ExpressionsBin*)p_globalinfo_bin, (HintFieldInfo *)hintFieldValues, (Goldilocks::Element *)publics, (Goldilocks::Element *)challenges, (Goldilocks::Element *)proofValues, (Goldilocks::Element **)airgroupValues, hintId, string(hintFieldName), print_expression);
}

uint64_t set_hint_field_global_constraints(char* globalInfoFile, void* p_globalinfo_bin, void *proofValues, void *values, uint64_t hintId, char *hintFieldName) 
{
    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    return setHintFieldGlobalConstraint(globalInfo, *(ExpressionsBin*)p_globalinfo_bin, (Goldilocks::Element *)proofValues, (Goldilocks::Element *)values, hintId, string(hintFieldName));
}

// Debug functions
// =================================================================================  

void print_row(void *pSetupCtx, void *buffer, uint64_t stage, uint64_t row) {
    printRow(*(SetupCtx *)pSetupCtx, (Goldilocks::Element *)buffer, stage, row);
}

// Recursive proof
// ================================================================================= 
void *gen_recursive_proof(void *pSetupCtx, char* globalInfoFile, uint64_t airgroupId, void* witness, void* aux_trace, void *pConstPols, void *pConstTree, void* pPublicInputs, char* proof_file, bool vadcop) {
    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    auto setup = *(SetupCtx *)pSetupCtx;
    if(setup.starkInfo.starkStruct.verificationHashType == "GL") {
        return genRecursiveProof<Goldilocks::Element>(*(SetupCtx *)pSetupCtx, globalInfo, airgroupId, (Goldilocks::Element *)witness,  (Goldilocks::Element *)aux_trace, (Goldilocks::Element *)pConstPols, (Goldilocks::Element *)pConstTree, (Goldilocks::Element *)pPublicInputs, string(proof_file), vadcop);
    } else {
        return genRecursiveProof<RawFr::Element>(*(SetupCtx *)pSetupCtx, globalInfo, airgroupId, (Goldilocks::Element *)witness, (Goldilocks::Element *)aux_trace, (Goldilocks::Element *)pConstPols, (Goldilocks::Element *)pConstTree, (Goldilocks::Element *)pPublicInputs, string(proof_file), false);
    }
}

void *get_zkin_ptr(char *zkin_file) {
    json zkin;
    file2json(zkin_file, zkin);

    return (void *) new nlohmann::json(zkin);
}

void *add_recursive2_verkey(void *pZkin, char* recursive2VerKeyFilename) {
    json recursive2VerkeyJson;
    file2json(recursive2VerKeyFilename, recursive2VerkeyJson);

    Goldilocks::Element recursive2Verkey[4];
    for (uint64_t i = 0; i < 4; i++)
    {
        recursive2Verkey[i] = Goldilocks::fromU64(recursive2VerkeyJson[i]);
    }

    json zkin = addRecursive2VerKey(*(nlohmann::json*) pZkin, recursive2Verkey);
    return (void *) new nlohmann::json(zkin);
}

void *join_zkin_recursive2(char* globalInfoFile, uint64_t airgroupId, void* pPublics, void* pChallenges, void *zkin1, void *zkin2, void *starkInfoRecursive2) {
    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    Goldilocks::Element *publics = (Goldilocks::Element *)pPublics;
    Goldilocks::Element *challenges = (Goldilocks::Element *)pChallenges;

    json zkinRecursive2 = joinzkinrecursive2(globalInfo, airgroupId, publics, challenges, *(nlohmann::json *)zkin1, *(nlohmann::json *)zkin2, *(StarkInfo *)starkInfoRecursive2);

    return (void *) new nlohmann::json(zkinRecursive2);
}

void *join_zkin_final(void* pPublics, void *pProofValues, void* pChallenges, char* globalInfoFile, void **zkinRecursive2, void **starkInfoRecursive2) {
    json globalInfo;
    file2json(globalInfoFile, globalInfo);

    Goldilocks::Element *publics = (Goldilocks::Element *)pPublics;
    Goldilocks::Element *challenges = (Goldilocks::Element *)pChallenges;
    Goldilocks::Element *proofValues = (Goldilocks::Element *)pProofValues;

    json zkinFinal = joinzkinfinal(globalInfo, publics, proofValues, challenges, zkinRecursive2, starkInfoRecursive2);

    return (void *) new nlohmann::json(zkinFinal);    
}

char *get_serialized_proof(void *zkin, uint64_t* size){
    nlohmann::json* zkinJson = (nlohmann::json*) zkin;
    string zkinStr = zkinJson->dump();
    char *zkinCStr = new char[zkinStr.length() + 1];
    strcpy(zkinCStr, zkinStr.c_str());
    *size = zkinStr.length()+1;
    return zkinCStr;
}

void *deserialize_zkin_proof(char* serialized_proof) {
    nlohmann::json* zkinJson = new nlohmann::json();
    try {
        *zkinJson = nlohmann::json::parse(serialized_proof);
    } catch (const nlohmann::json::parse_error& e) {
        std::cerr << "[ERROR] JSON parse error in deserialize_zkin_proof(): " << e.what() << std::endl;
        delete zkinJson;
        return nullptr;
    }
    return (void *) zkinJson;
}

void *get_zkin_proof(char* zkin) {
    nlohmann::json zkinJson;
    file2json(zkin, zkinJson);
    return (void *) new nlohmann::json(zkinJson);
}

void zkin_proof_free(void *pZkinProof) {
    nlohmann::json* zkin = (nlohmann::json*) pZkinProof;
    delete zkin;
}

void serialized_proof_free(char *zkinCStr) {
    delete[] zkinCStr;
}

void get_committed_pols(void *circomWitness, char* execFile, void *witness, void* pPublics, uint64_t sizeWitness, uint64_t N, uint64_t nPublics, uint64_t nCommitedPols) {
    getCommitedPols((Goldilocks::Element *)circomWitness, string(execFile), (Goldilocks::Element *)witness, (Goldilocks::Element *)pPublics, sizeWitness, N, nPublics, nCommitedPols);
}

void gen_final_snark_proof(void *circomWitnessFinal, char* zkeyFile, char* outputDir) {
    genFinalSnarkProof(circomWitnessFinal, string(zkeyFile), string(outputDir));
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


// Stark Verify
// =================================================================================
bool stark_verify(void* jProof, void *pStarkInfo, void *pExpressionsBin, char *verkeyFile, void *pPublics, void *pProofValues, void *pChallenges) {
    Goldilocks::Element *challenges = (Goldilocks::Element *)pChallenges;
    bool vadcop = challenges == nullptr ? false : true;
    StarkInfo starkInfo = *((StarkInfo *)pStarkInfo);
    if (starkInfo.starkStruct.verificationHashType == "GL") {
        return starkVerify<Goldilocks::Element>(*(nlohmann::json*) jProof, *(StarkInfo *)pStarkInfo, *(ExpressionsBin *)pExpressionsBin, string(verkeyFile), (Goldilocks::Element *)pPublics, (Goldilocks::Element *)pProofValues, vadcop, (Goldilocks::Element *)pChallenges);
    } else {
        return starkVerify<RawFr::Element>(*(nlohmann::json*) jProof, *(StarkInfo *)pStarkInfo, *(ExpressionsBin *)pExpressionsBin, string(verkeyFile), (Goldilocks::Element *)pPublics, (Goldilocks::Element *)pProofValues, vadcop, (Goldilocks::Element *)pChallenges);
    }
}

// Debug circom
// =================================================================================
void save_to_file(void *buffer, uint64_t bufferSize, void* publics, uint64_t publicsSize, char* name) {
    json j;
    Goldilocks::Element *buff = (Goldilocks::Element *)buffer;
    for(uint64_t i = 0; i < bufferSize; ++i) {
        j["buffer"][i] = Goldilocks::toString(buff[i]);
    }

    Goldilocks::Element *pubs = (Goldilocks::Element *)publics;
    for(uint64_t i = 0; i < publicsSize; ++i) {
        j["publics"][i] = Goldilocks::toString(pubs[i]);
    }

    json2file(j, string(name));
}

void read_from_file(void* buffer, uint64_t bufferSize, void* publics, uint64_t publicsSize, char* name) {
    json j;
    file2json(string(name), j);
    Goldilocks::Element *buff = (Goldilocks::Element *)buffer;
    for(uint64_t i = 0; i < bufferSize; ++i) {
        buff[i] = Goldilocks::fromString(j["buffer"][i]);
    }

    Goldilocks::Element *pubs = (Goldilocks::Element *)publics;
    for(uint64_t i = 0; i < publicsSize; ++i) {
        pubs[i] = Goldilocks::fromString(j["publics"][i]);
    }
}

void *create_buffer(uint64_t size) {
    Goldilocks::Element *buffer = new Goldilocks::Element[size];
    cout << buffer << std::endl;
    return (void *)buffer;
}

void free_buffer(void *buffer) {
    cout <<  (Goldilocks::Element *)buffer << endl;
    delete[] (Goldilocks::Element *)buffer;
}