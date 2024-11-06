

#include <string>
#include <iostream>
#include "proof2zkinStark.hpp"
using namespace std;

ordered_json proof2zkinStark(ordered_json &proof, StarkInfo &starkInfo)
{
    uint64_t friSteps = starkInfo.starkStruct.steps.size() - 1;
    uint64_t nQueries = starkInfo.starkStruct.nQueries;
    uint64_t nStages = starkInfo.nStages;
   
    string valsQ = "s0_vals" + to_string(nStages + 1);
    string siblingsQ = "s0_siblings" + to_string(nStages + 1);
    string rootQ = "root" + to_string(nStages + 1);

    ordered_json zkinOut = ordered_json::object();

    for(uint64_t stage = 1; stage <= nStages; stage++) {
        zkinOut["root" + to_string(stage)] = proof["root" + to_string(stage)];
    }

    zkinOut[rootQ] = proof["root" + to_string(nStages + 1)];
    zkinOut["evals"] = proof["evals"];

    for (uint64_t i = 0; i < friSteps; i++)
    {
        zkinOut["s" + std::to_string(i + 1) + "_root"] = proof["fri"][i]["root"];
        zkinOut["s" + std::to_string(i + 1) + "_vals"] = ordered_json::array();
        zkinOut["s" + std::to_string(i + 1) + "_siblings"] = ordered_json::array();
        for (uint q = 0; q < nQueries; q++)
        {
            zkinOut["s" + std::to_string(i + 1) + "_vals"][q] = proof["fri"][i]["polQueries"][q][0];
            zkinOut["s" + std::to_string(i + 1) + "_siblings"][q] = proof["fri"][i]["polQueries"][q][1];
        }
    }
  
    zkinOut["s0_valsC"] = ordered_json::array();
    zkinOut["s0_siblingsC"] = ordered_json::array();
    
    zkinOut[valsQ] = ordered_json::array();
    zkinOut[siblingsQ] = ordered_json::array();

    for(uint64_t i = 0; i < nStages; ++i) {
        uint64_t stage = i + 1;
        if (proof["queries"]["polQueries"][0][i][0].size()) {
            zkinOut["s0_siblings" + to_string(stage)] = ordered_json::array();
            zkinOut["s0_vals" + to_string(stage)] = ordered_json::array();
        }
    }

    for (uint64_t i = 0; i < nQueries; i++) {
        for (uint64_t j = 0; j < nStages; ++j) {
            uint64_t stage = j + 1;
            if (proof["queries"]["polQueries"][i][j][0].size()) {
                zkinOut["s0_vals" + to_string(stage)][i] = proof["queries"]["polQueries"][i][j][0];
                zkinOut["s0_siblings" + to_string(stage)][i] = proof["queries"]["polQueries"][i][j][1];
            }
        }

        zkinOut[valsQ][i] = proof["queries"]["polQueries"][i][nStages][0];
        zkinOut[siblingsQ][i] = proof["queries"]["polQueries"][i][nStages][1];

        zkinOut["s0_valsC"][i] = proof["queries"]["polQueries"][i][nStages + 1][0];
        zkinOut["s0_siblingsC"][i] = proof["queries"]["polQueries"][i][nStages + 1][1];
    }

    zkinOut["finalPol"] = proof["fri"][friSteps];

    if (starkInfo.airgroupValuesMap.size() > 0) {
        zkinOut["airgroupvalues"] = proof["airgroupValues"];
    }

    if (starkInfo.airValuesMap.size() > 0) {
        zkinOut["airvalues"] = proof["airValues"];
    }
    
    return zkinOut;
};

ordered_json joinzkin(ordered_json &zkin1, ordered_json &zkin2, ordered_json &verKey, StarkInfo &starkInfo)
{

    uint64_t friSteps = starkInfo.starkStruct.steps.size();
    uint64_t nStages = starkInfo.nStages;

    string valsQ = "s0_vals" + to_string(nStages + 1);
    string siblingsQ = "s0_siblings" + to_string(nStages + 1);
    string rootQ = "root" + to_string(nStages + 1);

    ordered_json zkinOut = ordered_json::object();

    // Load oldStateRoot
    for (int i = 0; i < 8; i++)
    {
        zkinOut["publics"][i] = zkin1["publics"][i];
    }

    // Load oldAccInputHash0
    for (int i = 0; i < 8; i++)
    {
        zkinOut["publics"][i + 8] = zkin1["publics"][8 + i];
    }

    zkinOut["publics"][16] = zkin1["publics"][16]; // oldBatchNum

    zkinOut["publics"][17] = zkin1["publics"][17]; // chainId

    zkinOut["publics"][18] = zkin1["publics"][18]; // forkid

    // newStateRoot
    for (int i = 0; i < 8; i++)
    {
        zkinOut["publics"][19 + i] = zkin2["publics"][19 + i];
    }
    // newAccInputHash0
    for (int i = 0; i < 8; i++)
    {
        zkinOut["publics"][27 + i] = zkin2["publics"][27 + i];
    }
    // newLocalExitRoot
    for (int i = 0; i < 8; i++)
    {
        zkinOut["publics"][35 + i] = zkin2["publics"][35 + i];
    }

    zkinOut["publics"][43] = zkin2["publics"][43]; // oldBatchNum

    zkinOut["a_publics"] = zkin1["publics"];

    for(uint64_t stage = 1; stage <= nStages; stage++) {
        zkinOut["a_root" + to_string(stage)] = zkin1["root" + to_string(stage)];
    }
    zkinOut["a_" + rootQ] = zkin1[rootQ];

    zkinOut["a_evals"] = zkin1["evals"];
    zkinOut["a_s0_valsC"] = zkin1["s0_valsC"];
    zkinOut["a_s0_siblingsC"] = zkin1["s0_siblingsC"];
    for(uint64_t stage = 1; stage <= nStages; ++stage) {
        if(starkInfo.mapSectionsN["cm" + to_string(stage)] > 0) {
            zkinOut["a_s0_vals" + to_string(stage)] = zkin1["s0_vals" + to_string(stage)];
            zkinOut["a_s0_siblings" + to_string(stage)] = zkin1["s0_siblings" + to_string(stage)];
        }
    }
    zkinOut["a_" + siblingsQ] = zkin1[siblingsQ];
    zkinOut["a_" + valsQ] = zkin1[valsQ];

    for (uint64_t i = 1; i < friSteps; i++)
    {
        zkinOut["a_s" + std::to_string(i) + "_root"] = zkin1["s" + std::to_string(i) + "_root"];
        zkinOut["a_s" + std::to_string(i) + "_siblings"] = zkin1["s" + std::to_string(i) + "_siblings"];
        zkinOut["a_s" + std::to_string(i) + "_vals"] = zkin1["s" + std::to_string(i) + "_vals"];
    }
    zkinOut["a_finalPol"] = zkin1["finalPol"];

    zkinOut["b_publics"] = zkin2["publics"];
    for(uint64_t stage = 1; stage <= nStages; stage++) {
        zkinOut["b_root" + to_string(stage)] = zkin2["root" + to_string(stage)];
    }
    zkinOut["b_" + rootQ] = zkin2[rootQ];

    zkinOut["b_evals"] = zkin2["evals"];
    zkinOut["b_s0_valsC"] = zkin2["s0_valsC"];
    zkinOut["b_s0_siblingsC"] = zkin2["s0_siblingsC"];
    for(uint64_t stage = 1; stage <= nStages; ++stage) {
        if(starkInfo.mapSectionsN["cm" + to_string(stage)] > 0) {
            zkinOut["b_s0_vals" + to_string(stage)] = zkin2["s0_vals" + to_string(stage)];
            zkinOut["b_s0_siblings" + to_string(stage)] = zkin2["s0_siblings" + to_string(stage)];
        }
    }
    zkinOut["b_" + siblingsQ] = zkin2[siblingsQ];
    zkinOut["b_" + valsQ] = zkin2[valsQ];

    for (uint64_t i = 1; i < friSteps; i++)
    {
        zkinOut["b_s" + std::to_string(i) + "_root"] = zkin2["s" + std::to_string(i) + "_root"];
        zkinOut["b_s" + std::to_string(i) + "_siblings"] = zkin2["s" + std::to_string(i) + "_siblings"];
        zkinOut["b_s" + std::to_string(i) + "_vals"] = zkin2["s" + std::to_string(i) + "_vals"];
    }
    zkinOut["b_finalPol"] = zkin2["finalPol"];

    zkinOut["rootC"] = ordered_json::array();
    for (int i = 0; i < 4; i++)
    {
        zkinOut["rootC"][i] = to_string(verKey["constRoot"][i]);
    }

    return zkinOut;
}

ordered_json challenges2proof(json& globalInfo, Goldilocks::Element* challenges) {
    
    ordered_json challengesJson;

    uint64_t nStages = globalInfo["numChallenges"].size();

    uint64_t c = 0;

    challengesJson["challenges"] = ordered_json::array();
    for(uint64_t i = 0; i < nStages; ++i) {
        challengesJson["challenges"][i] = ordered_json::array();
        for(uint64_t j = 0; j < globalInfo["numChallenges"][i]; ++j) {
            challengesJson["challenges"][i][j] = ordered_json::array();
            for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
                challengesJson["challenges"][i][j][k] = Goldilocks::toString(challenges[c++]);
            }
        }
    }

    challengesJson["challenges"][nStages] = ordered_json::array();
    challengesJson["challenges"][nStages][0] = ordered_json::array();
    for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
        challengesJson["challenges"][nStages][0][k] = Goldilocks::toString(challenges[c++]);
    }
    
    challengesJson["challenges"][nStages + 1] = ordered_json::array();
    challengesJson["challenges"][nStages + 1][0] = ordered_json::array();
    for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
        challengesJson["challenges"][nStages + 1][0][k] = Goldilocks::toString(challenges[c++]);
    }

    challengesJson["challenges"][nStages + 2] = ordered_json::array();
    challengesJson["challenges"][nStages + 2][0] = ordered_json::array();
    for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
        challengesJson["challenges"][nStages + 2][0][k] = Goldilocks::toString(challenges[c++]);
    }
    
    challengesJson["challenges"][nStages + 2][1] = ordered_json::array();
    for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
        challengesJson["challenges"][nStages + 2][1][k] = Goldilocks::toString(challenges[c++]);
    }

    challengesJson["challengesFRISteps"] = ordered_json::array();
    for(uint64_t i = 0; i < globalInfo["stepsFRI"].size() + 1; ++i) {
        challengesJson["challengesFRISteps"][i] = ordered_json::array();
        for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
            challengesJson["challengesFRISteps"][i][k] = Goldilocks::toString(challenges[c++]);
        }
    }

    return challengesJson;
}

ordered_json challenges2zkin(json& globalInfo, Goldilocks::Element* challenges) {
    
    ordered_json challengesJson;

    uint64_t nStages = globalInfo["numChallenges"].size();

    uint64_t nChallenges = 0;
    for(uint64_t i = 0; i < nStages; ++i) {
        nChallenges += uint64_t(globalInfo["numChallenges"][i]);
    }
    nChallenges += 4;

    challengesJson["challenges"] = ordered_json::array();
    for(uint64_t i = 0; i < nChallenges; ++i) {
        challengesJson["challenges"][i] = ordered_json::array();
        for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
            challengesJson["challenges"][i][k] = Goldilocks::toString(challenges[i*FIELD_EXTENSION + k]);
        }
    }
    
    challengesJson["challengesFRISteps"] = ordered_json::array();
    for(uint64_t i = 0; i < globalInfo["stepsFRI"].size() + 1; ++i) {
        challengesJson["challengesFRISteps"][i] = ordered_json::array();
        for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
            challengesJson["challengesFRISteps"][i][k] = Goldilocks::toString(challenges[nChallenges*FIELD_EXTENSION + i*FIELD_EXTENSION + k]);
        }
    }

    return challengesJson;
}

ordered_json publics2zkin(ordered_json &zkin_, Goldilocks::Element* publics, json& globalInfo, uint64_t airgroupId) {
    ordered_json zkin = ordered_json::object();
    zkin = zkin_;

    uint64_t p = 0;
    zkin["sv_circuitType"] = Goldilocks::toString(publics[p++]);
    if(globalInfo["aggTypes"][airgroupId].size() > 0) {
        zkin["sv_aggregationTypes"] = ordered_json::array();
        for(uint64_t i = 0; i < globalInfo["aggTypes"][airgroupId].size(); ++i) {
            zkin["sv_aggregationTypes"][i] = Goldilocks::toString(publics[p++]);
        }

        zkin["sv_airgroupvalues"] = ordered_json::array();
        for(uint64_t i = 0; i < globalInfo["aggTypes"][airgroupId].size(); ++i) {
            zkin["sv_airgroupvalues"][i] = ordered_json::array();
            for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
                zkin["sv_airgroupvalues"][i][k] = Goldilocks::toString(publics[p++]);
            }
        }
    }

    for(uint64_t i = 0; i < globalInfo["numChallenges"].size() + 1; ++i) {
        std::string sv_root = "sv_root" + to_string(i + 1);
        zkin[sv_root] = ordered_json::array();
        for(uint64_t j = 0; j < 4; ++j) {
            zkin[sv_root][j] = Goldilocks::toString(publics[p++]);
        }
    }

    zkin["sv_evalsHash"] = ordered_json::array();
    for(uint64_t j = 0; j < 4; ++j) {
        zkin["sv_evalsHash"][j] = Goldilocks::toString(publics[p++]);
    }

    for(uint64_t i = 0; i < globalInfo["stepsFRI"].size() - 1; ++i) {
        std::string sv_si_root = "sv_s" + to_string(i + 1) + "_root"; 
        zkin[sv_si_root] = ordered_json::array();
        for(uint64_t j = 0; j < 4; ++j) {
            zkin[sv_si_root][j] = Goldilocks::toString(publics[p++]);
        }
    }

    zkin["sv_finalPolHash"] = ordered_json::array();
    for(uint64_t j = 0; j < 4; ++j) {
        zkin["sv_finalPolHash"][j] = Goldilocks::toString(publics[p++]);
    }

    if(uint64_t(globalInfo["nPublics"]) > 0) {
        zkin["publics"] = ordered_json::array();
        for(uint64_t i = 0; i < uint64_t(globalInfo["nPublics"]); ++i) {
            zkin["publics"][i] = Goldilocks::toString(publics[p++]);
        }
    }

    zkin["challenges"] = ordered_json::array();
    
    uint64_t nChallenges = 0;
    for(uint64_t i = 0; i < globalInfo["numChallenges"].size(); ++i) {
        nChallenges += uint64_t(globalInfo["numChallenges"][i]);
    }
    nChallenges += 4;
    for(uint64_t i = 0; i < nChallenges; ++i) {
        zkin["challenges"][i] = ordered_json::array();
        for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
            zkin["challenges"][i][k] = Goldilocks::toString(publics[p++]);
        }
    }

    zkin["challengesFRISteps"] = ordered_json::array();
    for(uint64_t i = 0; i < globalInfo["stepsFRI"].size() + 1; ++i) {
        zkin["challengesFRISteps"][i] = ordered_json::array();
        for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
            zkin["challengesFRISteps"][i][k] = Goldilocks::toString(publics[p++]);
        }
    }
    

    return zkin;
}

ordered_json addRecursive2VerKey(ordered_json &zkin, Goldilocks::Element* recursive2VerKey) {
    ordered_json zkinUpdated = ordered_json::object();
    zkinUpdated = zkin;
    zkinUpdated["rootCAgg"] = ordered_json::array();
    for(uint64_t i = 0; i < 4; ++i) {
        zkinUpdated["rootCAgg"][i] = Goldilocks::toString(recursive2VerKey[i]);
    }

    return zkinUpdated;
}

ordered_json joinzkinfinal(json& globalInfo, Goldilocks::Element* publics, Goldilocks::Element* proofValues, Goldilocks::Element* challenges, void **zkin_vec, void **starkInfo_vec) {
    ordered_json zkinFinal = ordered_json::object();
    
    if(globalInfo["nPublics"] > 0) {
        for (uint64_t i = 0; i < globalInfo["nPublics"]; i++)
        {
            zkinFinal["publics"][i] = Goldilocks::toString(publics[i]);
        }
    }

    if(globalInfo["numProofValues"] > 0) {
        zkinFinal["proofValues"] = ordered_json::array();
        for (uint64_t i = 0; i < globalInfo["numProofValues"]; i++)
        {
            zkinFinal["proofValues"][i] = ordered_json::array();
            for(uint64_t j = 0; j < FIELD_EXTENSION; ++j) {
                zkinFinal["proofValues"][i][j] = Goldilocks::toString(proofValues[i*FIELD_EXTENSION + j]);
            }
        }
    }

    ordered_json challengesJson = challenges2zkin(globalInfo, challenges);
    zkinFinal["challenges"] = challengesJson["challenges"];
    zkinFinal["challengesFRISteps"] = challengesJson["challengesFRISteps"];

    for(uint64_t i = 0; i < globalInfo["air_groups"].size(); ++i) {
        ordered_json zkin = *(ordered_json *)zkin_vec[i];
        StarkInfo &starkInfo = *(StarkInfo *)starkInfo_vec[i];

        uint64_t nStages = starkInfo.nStages + 1;

        for(uint64_t stage = 1; stage <= nStages; stage++) {
            zkinFinal["s" + to_string(i) + "_root" + to_string(stage)] = zkin["root" + to_string(stage)];
        }

        for(uint64_t stage = 1; stage <= nStages; stage++) {
            if(starkInfo.mapSectionsN["cm" + to_string(stage)] > 0) {
                zkinFinal["s" + to_string(i) + "_s0_vals" + to_string(stage)] = zkin["s0_vals" + to_string(stage)];
                zkinFinal["s" + to_string(i) + "_s0_siblings" + to_string(stage)] = zkin["s0_siblings" + to_string(stage)];
            }
        }
        
        zkinFinal["s" + to_string(i) + "_s0_valsC"] = zkin["s0_valsC"];
        zkinFinal["s" + to_string(i) + "_s0_siblingsC"] = zkin["s0_siblingsC"];

        zkinFinal["s" + to_string(i) + "_evals"] = zkin["evals"];

        for(uint64_t s = 1; s < starkInfo.starkStruct.steps.size(); ++s) {
            zkinFinal["s" + to_string(i) + "_s" + to_string(s) + "_root"] = zkin["s" + to_string(s) + "_root"];
            zkinFinal["s" + to_string(i) + "_s" + to_string(s) + "_vals"] = zkin["s" + to_string(s) + "_vals"];
            zkinFinal["s" + to_string(i) + "_s" + to_string(s) + "_siblings"] = zkin["s" + to_string(s) + "_siblings"];
        }
        
        zkinFinal["s" + to_string(i) + "_finalPol"] = zkin["finalPol"];

        zkinFinal["s" + to_string(i) + "_sv_circuitType"] = zkin["sv_circuitType"];

        if(globalInfo["aggTypes"][i].size() > 0) {
            zkinFinal["s" + to_string(i) + "_sv_aggregationTypes"] = zkin["sv_aggregationTypes"];
            zkinFinal["s" + to_string(i) + "_sv_airgroupvalues"] = zkin["sv_airgroupvalues"];
        }

        for(uint64_t j = 0; j < globalInfo["numChallenges"].size() + 1; ++j) {
            zkinFinal["s" + to_string(i) + "_sv_root" + to_string(j + 1)] = zkin["sv_root" + to_string(j + 1)];
        }

        zkinFinal["s" + to_string(i) + "_sv_evalsHash"] = zkin["sv_evalsHash"];

        for(uint64_t j = 0; j < globalInfo["stepsFRI"].size() - 1; ++j) {
            zkinFinal["s" + to_string(i) + "_sv_s" + to_string(j + 1) + "_root"] = zkin["sv_s" + to_string(j + 1) + "_root"];
        }

        zkinFinal["s" + to_string(i) + "_sv_finalPolHash"] = zkin["sv_finalPolHash"];
    }

    return zkinFinal;
}

ordered_json joinzkinrecursive2(json& globalInfo, uint64_t airgroupId, Goldilocks::Element* publics, Goldilocks::Element* challenges, ordered_json &zkin1, ordered_json &zkin2, StarkInfo &starkInfo) {
    ordered_json zkinRecursive2 = ordered_json::object();

    uint64_t nStages = starkInfo.nStages + 1;

    for (uint64_t i = 0; i < globalInfo["nPublics"]; i++)
    {
        zkinRecursive2["publics"][i] = Goldilocks::toString(publics[i]);
    }

    ordered_json challengesJson = challenges2zkin(globalInfo, challenges);
    zkinRecursive2["challenges"] = challengesJson["challenges"];
    zkinRecursive2["challengesFRISteps"] = challengesJson["challengesFRISteps"];

    for(uint64_t stage = 1; stage <= nStages; stage++) {
        zkinRecursive2["a_root" + to_string(stage)] = zkin1["root" + to_string(stage)];
        zkinRecursive2["b_root" + to_string(stage)] = zkin2["root" + to_string(stage)];
    }

    for(uint64_t stage = 1; stage <= nStages; stage++) {
        if(starkInfo.mapSectionsN["cm" + to_string(stage)] > 0) {
            zkinRecursive2["a_s0_vals" + to_string(stage)] = zkin1["s0_vals" + to_string(stage)];
            zkinRecursive2["a_s0_siblings" + to_string(stage)] = zkin1["s0_siblings" + to_string(stage)];
            zkinRecursive2["b_s0_vals" + to_string(stage)] = zkin2["s0_vals" + to_string(stage)];
            zkinRecursive2["b_s0_siblings" + to_string(stage)] = zkin2["s0_siblings" + to_string(stage)];
        }
    }
    
    zkinRecursive2["a_s0_valsC"] = zkin1["s0_valsC"];
    zkinRecursive2["b_s0_valsC"] = zkin2["s0_valsC"];

    zkinRecursive2["a_s0_siblingsC"] = zkin1["s0_siblingsC"];
    zkinRecursive2["b_s0_siblingsC"] = zkin2["s0_siblingsC"];
    
    zkinRecursive2["a_evals"] = zkin1["evals"];
    zkinRecursive2["b_evals"] = zkin2["evals"];


    for(uint64_t s = 1; s < starkInfo.starkStruct.steps.size(); ++s) {
        zkinRecursive2["a_s" + to_string(s) + "_root"] = zkin1["s" + to_string(s) + "_root"];
        zkinRecursive2["a_s" + to_string(s) + "_vals"] = zkin1["s" + to_string(s) + "_vals"];
        zkinRecursive2["a_s" + to_string(s) + "_siblings"] = zkin1["s" + to_string(s) + "_siblings"];

        zkinRecursive2["b_s" + to_string(s) + "_root"] = zkin2["s" + to_string(s) + "_root"];
        zkinRecursive2["b_s" + to_string(s) + "_vals"] = zkin2["s" + to_string(s) + "_vals"];
        zkinRecursive2["b_s" + to_string(s) + "_siblings"] = zkin2["s" + to_string(s) + "_siblings"];
    }
    
    zkinRecursive2["a_finalPol"] = zkin1["finalPol"];
    zkinRecursive2["b_finalPol"] = zkin2["finalPol"];

    zkinRecursive2["a_sv_circuitType"] = zkin1["sv_circuitType"];
    zkinRecursive2["b_sv_circuitType"] = zkin2["sv_circuitType"];
    
    if(globalInfo["aggTypes"][airgroupId].size() > 0) {
        zkinRecursive2["aggregationTypes"] = zkin2["sv_aggregationTypes"];
        for(uint64_t a = 0; a < globalInfo["aggTypes"][airgroupId].size(); ++a) {
            assert(zkin2["sv_aggregationTypes"][a] == zkin1["sv_aggregationTypes"][a]);
        }

        zkinRecursive2["a_sv_airgroupvalues"] = zkin1["sv_airgroupvalues"];
        zkinRecursive2["b_sv_airgroupvalues"] = zkin2["sv_airgroupvalues"];
    }

    for(uint64_t j = 0; j < globalInfo["numChallenges"].size() + 1; ++j) {
        zkinRecursive2["a_sv_root" + to_string(j + 1)] = zkin1["sv_root" + to_string(j + 1)];
        zkinRecursive2["b_sv_root" + to_string(j + 1)] = zkin2["sv_root" + to_string(j + 1)];
    }

    zkinRecursive2["a_sv_evalsHash"] = zkin1["sv_evalsHash"];
    zkinRecursive2["b_sv_evalsHash"] = zkin2["sv_evalsHash"];

    for(uint64_t j = 0; j < globalInfo["stepsFRI"].size() - 1; ++j) {
        zkinRecursive2["a_sv_s" + to_string(j + 1) + "_root"] = zkin1["sv_s" + to_string(j + 1) + "_root"];
        zkinRecursive2["b_sv_s" + to_string(j + 1) + "_root"] = zkin2["sv_s" + to_string(j + 1) + "_root"];
    }

    zkinRecursive2["a_sv_finalPolHash"] = zkin1["sv_finalPolHash"];
    zkinRecursive2["b_sv_finalPolHash"] = zkin2["sv_finalPolHash"];

    return zkinRecursive2;
}