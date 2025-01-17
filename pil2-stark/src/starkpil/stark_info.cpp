#include "stark_info.hpp"
#include "utils.hpp"
#include "timer.hpp"
#include "zklog.hpp"
#include "exit_process.hpp"

StarkInfo::StarkInfo(string file, bool verify_)
{
    // Load contents from json file
    json starkInfoJson;
    file2json(file, starkInfoJson);
    load(starkInfoJson, verify_);
}

void StarkInfo::load(json j, bool verify_)
{   
    starkStruct.nBits = j["starkStruct"]["nBits"];
    starkStruct.nBitsExt = j["starkStruct"]["nBitsExt"];
    starkStruct.nQueries = j["starkStruct"]["nQueries"];
    starkStruct.verificationHashType = j["starkStruct"]["verificationHashType"];
    if(starkStruct.verificationHashType == "BN128") {
        if(j["starkStruct"].contains("merkleTreeArity")) {
            starkStruct.merkleTreeArity = j["starkStruct"]["merkleTreeArity"];
        } else {
            starkStruct.merkleTreeArity = 16;
        }
        if(j["starkStruct"].contains("merkleTreeCustom")) {
            starkStruct.merkleTreeCustom = j["starkStruct"]["merkleTreeCustom"];
        } else {
            starkStruct.merkleTreeCustom = false;
        }
    } else {
        starkStruct.merkleTreeArity = 2;
        starkStruct.merkleTreeCustom = true;
    }
    if(j["starkStruct"].contains("hashCommits")) {
        starkStruct.hashCommits = j["starkStruct"]["hashCommits"];
    } else {
        starkStruct.hashCommits = false;
    }

    for (uint64_t i = 0; i < j["starkStruct"]["steps"].size(); i++)
    {
        StepStruct step;
        step.nBits = j["starkStruct"]["steps"][i]["nBits"];
        starkStruct.steps.push_back(step);
    }

    airId = j["airId"];
    airgroupId = j["airgroupId"];

    nPublics = j["nPublics"];
    nConstants = j["nConstants"];

    nStages = j["nStages"];

    qDeg = j["qDeg"];
    qDim = j["qDim"];

    friExpId = j["friExpId"];
    cExpId = j["cExpId"];


    for(uint64_t i = 0; i < j["customCommits"].size(); i++) {
        CustomCommits c;
        c.name = j["customCommits"][i]["name"];
        for(uint64_t k = 0; k < j["customCommits"][i]["publicValues"].size(); k++) {
            c.publicValues.push_back(j["customCommits"][i]["publicValues"][k]["idx"]);
        }
        for(uint64_t k = 0; k < j["customCommits"][i]["stageWidths"].size(); k++) {
            c.stageWidths.push_back(j["customCommits"][i]["stageWidths"][k]);
        }
        customCommits.push_back(c);
    }

    for(uint64_t i = 0; i < j["openingPoints"].size(); i++) {
        openingPoints.push_back(j["openingPoints"][i]);
    }

    for(uint64_t i = 0; i < j["boundaries"].size(); i++) {
        Boundary b;
        b.name = j["boundaries"][i]["name"];
        if(b.name == string("everyFrame")) {
            b.offsetMin = j["boundaries"][i]["offsetMin"];
            b.offsetMax = j["boundaries"][i]["offsetMax"];
        }
        boundaries.push_back(b);
    }

    for (uint64_t i = 0; i < j["challengesMap"].size(); i++) 
    {
        PolMap map;
        map.stage = j["challengesMap"][i]["stage"];
        map.name = j["challengesMap"][i]["name"];
        map.dim = j["challengesMap"][i]["dim"];
        map.stageId = j["challengesMap"][i]["stageId"];
        challengesMap.push_back(map);
    }

    for (uint64_t i = 0; i < j["publicsMap"].size(); i++) 
    {
        PolMap map;
        map.name = j["publicsMap"][i]["name"];
        if(j["publicsMap"][i].contains("lengths")) {
            for (uint64_t l = 0; l < j["publicsMap"][i]["lengths"].size(); l++) {
                map.lengths.push_back(j["publicsMap"][i]["lengths"][l]);
            } 
        }
        publicsMap.push_back(map);
    }

    for (uint64_t i = 0; i < j["airgroupValuesMap"].size(); i++) 
    {
        PolMap map;
        map.name = j["airgroupValuesMap"][i]["name"];
        map.stage = j["airgroupValuesMap"][i]["stage"];
        airgroupValuesMap.push_back(map);
    }

    for (uint64_t i = 0; i < j["airValuesMap"].size(); i++) 
    {
        PolMap map;
        map.name = j["airValuesMap"][i]["name"];
        map.stage = j["airValuesMap"][i]["stage"];
        airValuesMap.push_back(map);
    }

    for (uint64_t i = 0; i < j["proofValuesMap"].size(); i++) 
    {
        PolMap map;
        map.name = j["proofValuesMap"][i]["name"];
        map.stage = j["proofValuesMap"][i]["stage"];
        proofValuesMap.push_back(map);
    }

    for (uint64_t i = 0; i < j["cmPolsMap"].size(); i++) 
    {
        PolMap map;
        map.stage = j["cmPolsMap"][i]["stage"];
        map.name = j["cmPolsMap"][i]["name"];
        map.dim = j["cmPolsMap"][i]["dim"];
        map.imPol = j["cmPolsMap"][i].contains("imPol") ? true : false;
        map.stagePos = j["cmPolsMap"][i]["stagePos"];
        map.stageId = j["cmPolsMap"][i]["stageId"];
        if(j["cmPolsMap"][i].contains("expId")) {
            map.expId = j["cmPolsMap"][i]["expId"];
        }
        if(j["cmPolsMap"][i].contains("lengths")) {
            for (uint64_t k = 0; k < j["cmPolsMap"][i]["lengths"].size(); k++) {
                map.lengths.push_back(j["cmPolsMap"][i]["lengths"][k]);
            } 
        }
        map.polsMapId = j["cmPolsMap"][i]["polsMapId"];
        cmPolsMap.push_back(map);
    }

    for (uint64_t i = 0; i < j["customCommitsMap"].size(); i++) 
    {
        vector<PolMap> custPolsMap(j["customCommitsMap"][i].size());
        for(uint64_t k = 0; k < j["customCommitsMap"][i].size(); ++k) {
            PolMap map;
            map.stage = j["customCommitsMap"][i][k]["stage"];
            map.name = j["customCommitsMap"][i][k]["name"];
            map.dim = j["customCommitsMap"][i][k]["dim"];
            map.stagePos = j["customCommitsMap"][i][k]["stagePos"];
            map.stageId = j["customCommitsMap"][i][k]["stageId"];
            map.commitId = i;
            if(j["customCommitsMap"][i][k].contains("expId")) {
                map.expId = j["customCommitsMap"][i][k]["expId"];
            }
            if(j["customCommitsMap"][i].contains("lengths")) {
                for (uint64_t l = 0; l < j["customCommitsMap"][i][k]["lengths"].size(); l++) {
                    map.lengths.push_back(j["customCommitsMap"][i][k]["lengths"][l]);
                } 
            }
            map.polsMapId = j["customCommitsMap"][i][k]["polsMapId"];
            custPolsMap[k] = map;
        }
        customCommitsMap.push_back(custPolsMap);
    }


    for (uint64_t i = 0; i < j["constPolsMap"].size(); i++) 
    {
        PolMap map;
        map.stage = j["constPolsMap"][i]["stage"];
        map.name = j["constPolsMap"][i]["name"];
        map.dim = j["constPolsMap"][i]["dim"];
        map.imPol = false;
        map.stagePos = j["constPolsMap"][i]["stageId"];
        map.stageId = j["constPolsMap"][i]["stageId"];
        if(j["constPolsMap"][i].contains("lengths")) {
            for (uint64_t k = 0; k < j["constPolsMap"][i]["lengths"].size(); k++) {
                map.lengths.push_back(j["constPolsMap"][i]["lengths"][k]);
            } 
        }
        map.polsMapId = j["constPolsMap"][i]["polsMapId"];
        constPolsMap.push_back(map);
    }

    for (uint64_t i = 0; i < j["evMap"].size(); i++)
    {
        EvMap map;
        map.setType(j["evMap"][i]["type"]);
        if(j["evMap"][i]["type"] == "custom") {
            map.commitId = j["evMap"][i]["commitId"];
        }
        map.id = j["evMap"][i]["id"];
        map.prime = j["evMap"][i]["prime"];
        if(j["evMap"][i].contains("openingPos")) {
            map.openingPos = j["evMap"][i]["openingPos"];
        } else {
            int64_t prime = map.prime;
            auto openingPoint = std::find_if(openingPoints.begin(), openingPoints.end(), [prime](int p) { return p == prime; });
            if(openingPoint == openingPoints.end()) {
                zklog.error("Opening point not found");
                exitProcess();
                exit(-1);
            }
            map.openingPos = std::distance(openingPoints.begin(), openingPoint);
        }
        evMap.push_back(map);
    }

    for (auto it = j["mapSectionsN"].begin(); it != j["mapSectionsN"].end(); it++)  
    {
        mapSectionsN[it.key()] = it.value();
    }

    if(verify_) {
        verify = verify_;
        mapTotalN = 0;
        mapOffsets[std::make_pair("const", false)] = 0;
        for(uint64_t stage = 1; stage <= nStages + 1; ++stage) {
            mapOffsets[std::make_pair("cm" + to_string(stage), false)] = mapTotalN;
            mapTotalN += mapSectionsN["cm" + to_string(stage)] * starkStruct.nQueries;
        }
        // Set offsets for custom commits
        for(uint64_t i = 0; i < customCommits.size(); ++i) {
            mapOffsets[std::make_pair(customCommits[i].name + "0", false)] = 0;
            mapOffsets[std::make_pair(customCommits[i].name + "0", true)] = 0;
        }
    } else {
        setMapOffsets();
    }
}

void StarkInfo::setMapOffsets() {
    uint64_t N = (1 << starkStruct.nBits);
    uint64_t NExtended = (1 << starkStruct.nBitsExt);

    // Set offsets for constants
    mapOffsets[std::make_pair("const", false)] = 0;
    mapOffsets[std::make_pair("const", true)] = 0;
    mapOffsets[std::make_pair("cm1", false)] = 0;

    // Set offsets for custom commits
    for(uint64_t i = 0; i < customCommits.size(); ++i) {
        mapOffsets[std::make_pair(customCommits[i].name + "0", false)] = 0;
        mapOffsets[std::make_pair(customCommits[i].name + "0", true)] = 0;
    }

    mapTotalN = 0;

    for(uint64_t stage = nStages; stage >= 2; stage--) {
        mapOffsets[std::make_pair("cm" + to_string(stage), false)] = mapTotalN;
        mapTotalN += N * mapSectionsN["cm" + to_string(stage)];
    }

    mapOffsets[std::make_pair("cm" + to_string(nStages), true)] = mapOffsets[std::make_pair("cm" + to_string(nStages), false)];
    mapTotalN = mapOffsets[std::make_pair("cm" + to_string(nStages), false)] + NExtended * mapSectionsN["cm" + to_string(nStages)];

    // Set offsets for all stages in the extended field (cm1, cm2, ..., cmN)
    for(uint64_t stage = 1; stage <= nStages + 1; stage++) {
        if(stage == nStages) continue;
        mapOffsets[std::make_pair("cm" + to_string(stage), true)] = mapTotalN;
        mapTotalN += NExtended * mapSectionsN["cm" + to_string(stage)];
    }

    // This is never used, just set to avoid invalid read
    mapOffsets[std::make_pair("cm" + to_string(nStages + 1), false)] = 0;

    mapOffsets[std::make_pair("f", true)] = mapTotalN;
    mapTotalN += NExtended * FIELD_EXTENSION;

    mapOffsets[std::make_pair("q", true)] = mapOffsets[std::make_pair("f", true)];

    mapOffsets[std::make_pair("evals", true)] = mapTotalN;
    mapTotalN += evMap.size() * omp_get_max_threads() * FIELD_EXTENSION;

    // Merkle tree nodes sizes
    for (uint64_t i = 0; i < nStages + 1; i++) {
        uint64_t numNodes = getNumNodesMT(1 << starkStruct.nBitsExt);
        mapOffsets[std::make_pair("mt" + to_string(i + 1), true)] = mapTotalN;
        mapTotalN += numNodes;
    }
    
    for(uint64_t step = 0; step < starkStruct.steps.size() - 1; ++step) {
        uint64_t height = 1 << starkStruct.steps[step + 1].nBits;
        uint64_t width = ((1 << starkStruct.steps[step].nBits) / height) * FIELD_EXTENSION;
        uint64_t numNodes = getNumNodesMT(height);
        mapOffsets[std::make_pair("fri_" + to_string(step + 1), true)] = mapTotalN;
        mapTotalN += height * width;
        mapOffsets[std::make_pair("mt_fri_" + to_string(step + 1), true)] = mapTotalN;
        mapTotalN += numNodes;
    }
}

void StarkInfo::addMemoryRecursive() {
    uint64_t NExtended = (1 << starkStruct.nBitsExt);
    mapOffsets[std::make_pair("xDivXSubXi", true)] = mapTotalN;
    mapOffsets[std::make_pair("LEv", true)] = mapTotalN;
    mapTotalN += openingPoints.size() * NExtended * FIELD_EXTENSION;
}

void StarkInfo::getPolynomial(Polinomial &pol, Goldilocks::Element *pAddress, string type, PolMap& polInfo, bool domainExtended) {
    uint64_t deg = domainExtended ? 1 << starkStruct.nBitsExt : 1 << starkStruct.nBits;
    uint64_t dim = polInfo.dim;
    std::string stage = type == "cm" ? "cm" + to_string(polInfo.stage) : type == "custom" ? customCommits[polInfo.commitId].name + "0" : "const";
    uint64_t nCols = mapSectionsN[stage];
    uint64_t offset = mapOffsets[std::make_pair(stage, domainExtended)];
    offset += polInfo.stagePos;
    pol = Polinomial(&pAddress[offset], deg, dim, nCols);
}

uint64_t StarkInfo::getNumNodesMT(uint64_t height) {
    if(starkStruct.verificationHashType == "BN128") {
        uint n_tmp = height;
        uint64_t nextN = floor(((double)(n_tmp - 1) / starkStruct.merkleTreeArity) + 1);
        uint64_t acc = nextN * starkStruct.merkleTreeArity;
        while (n_tmp > 1)
        {
            // FIll with zeros if n nodes in the leve is not even
            n_tmp = nextN;
            nextN = floor((n_tmp - 1) / starkStruct.merkleTreeArity) + 1;
            if (n_tmp > 1)
            {
                acc += nextN * starkStruct.merkleTreeArity;
            }
            else
            {
                acc += 1;
            }
        }
        return acc * sizeof(RawFr::Element) / sizeof(Goldilocks::Element);
    } else {
        return height * HASH_SIZE + (height - 1) * HASH_SIZE;
    }
}

opType string2opType(const string s) 
{
    if(s == "const") 
        return const_;
    if(s == "cm")
        return cm;
    if(s == "tmp")
        return tmp;
    if(s == "public")
        return public_;
    if(s == "airgroupvalue")
        return airgroupvalue;
    if(s == "challenge")
        return challenge;
    if(s == "number")
        return number;
    if(s == "string") 
        return string_;
    if(s == "airvalue") 
        return airvalue;
    if(s == "custom") 
        return custom;
    if(s == "proofvalue") 
        return proofvalue;
    zklog.error("string2opType() found invalid string=" + s);
    exitProcess();
    exit(-1);
}