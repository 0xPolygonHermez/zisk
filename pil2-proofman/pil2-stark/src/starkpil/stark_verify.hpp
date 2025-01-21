#include "expressions_ctx.hpp"
#include "stark_info.hpp"
#include "merkleTreeGL.hpp"
#include "merkleTreeBN128.hpp"

template <typename ElementType>
ElementType fromString(const std::string& element);

template<>
inline Goldilocks::Element fromString(const std::string& element) {
    return Goldilocks::fromString(element);
}

template<>
inline RawFr::Element fromString(const std::string& element) {
    RawFr::Element r;
    RawFr::field.fromString(r, element, 10);
    return r;
}

template <typename ElementType>
bool starkVerify(json jproof, StarkInfo& starkInfo, ExpressionsBin& expressionsBin, string verkeyFile, Goldilocks::Element *publics, Goldilocks::Element *proofValues, bool challengesVadcop, Goldilocks::Element* challenges_) {

    json verkeyJson;
    file2json(verkeyFile, verkeyJson);

    using TranscriptType = std::conditional_t<std::is_same<ElementType, Goldilocks::Element>::value, TranscriptGL, TranscriptBN128>;

    using MerkleTreeType = std::conditional_t<std::is_same<ElementType, Goldilocks::Element>::value, MerkleTreeGL, MerkleTreeBN128>;

    uint64_t nFieldElements = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? 1 : HASH_SIZE;

    ElementType verkey[nFieldElements];
    if(starkInfo.starkStruct.verificationHashType == "GL") {
        for(uint64_t i = 0; i < nFieldElements; i++) {
            verkey[i] = fromString<ElementType>(verkeyJson[i].dump());
        }
    } else {
        verkey[0] = fromString<ElementType>(verkeyJson);
    }

    uint64_t friQueries[starkInfo.starkStruct.nQueries];

    Goldilocks::Element evals[starkInfo.evMap.size()  * FIELD_EXTENSION];
    for(uint64_t i = 0; i < starkInfo.evMap.size(); ++i) {
        for(uint64_t j = 0; j < FIELD_EXTENSION; ++j) {
            evals[i*FIELD_EXTENSION + j] = Goldilocks::fromString(jproof["evals"][i][j]);
        }
    }

    Goldilocks::Element airgroupValues[starkInfo.airgroupValuesMap.size()  * FIELD_EXTENSION];
    for(uint64_t i = 0; i < starkInfo.airgroupValuesMap.size() ; ++i) {
        for(uint64_t j = 0; j < FIELD_EXTENSION; ++j) {
            airgroupValues[i*FIELD_EXTENSION + j] = Goldilocks::fromString(jproof["airgroupvalues"][i][j]);
        }
    }

    Goldilocks::Element airValues[starkInfo.airValuesMap.size()  * FIELD_EXTENSION];
    uint64_t c = 0;
    for(uint64_t i = 0; i < starkInfo.airValuesMap.size(); ++i) {
        if(starkInfo.airValuesMap[i].stage == 1) {
            airValues[c++] = Goldilocks::fromString(jproof["airvalues"][i][0]);
        } else {
            airValues[c++] = Goldilocks::fromString(jproof["airvalues"][i][0]);
            airValues[c++] = Goldilocks::fromString(jproof["airvalues"][i][1]);
            airValues[c++] = Goldilocks::fromString(jproof["airvalues"][i][2]);
        }
    }

    Goldilocks::Element challenges[(starkInfo.challengesMap.size() + starkInfo.starkStruct.steps.size() + 1) * FIELD_EXTENSION];

    if(!challengesVadcop) {
        uint64_t c = 0;
        TranscriptType transcript(starkInfo.starkStruct.merkleTreeArity, starkInfo.starkStruct.merkleTreeCustom);
        transcript.put(&verkey[0], nFieldElements);
        if(starkInfo.nPublics > 0) {
            if(!starkInfo.starkStruct.hashCommits) {
                transcript.put(&publics[0], starkInfo.nPublics);
            } else {
                ElementType hash[nFieldElements];
                TranscriptType transcriptHash(starkInfo.starkStruct.merkleTreeArity, starkInfo.starkStruct.merkleTreeCustom);
                transcriptHash.put(&publics[0], starkInfo.nPublics);
                transcriptHash.getState(hash);
                transcript.put(hash, nFieldElements);
            }
        }

        for(uint64_t s = 1; s <= starkInfo.nStages + 1; ++s) {
            uint64_t nChallenges = std::count_if(starkInfo.challengesMap.begin(), starkInfo.challengesMap.end(),[s](const PolMap& c) { return c.stage == s; });
            for(uint64_t i = 0; i < nChallenges; ++i) {
                transcript.getField((uint64_t *)&challenges[c*FIELD_EXTENSION]);
                c++;
            }
            ElementType root[nFieldElements];
            if(nFieldElements == 1) {
                root[0] = fromString<ElementType>(jproof["root" + to_string(s)]);
            } else {
                for(uint64_t i = 0; i < nFieldElements; ++i) {
                    root[i] = fromString<ElementType>(jproof["root" + to_string(s)][i]);
                }
            }
            transcript.put(&root[0], nFieldElements);
        }

        // Evals challenge
        transcript.getField((uint64_t *)&challenges[c*FIELD_EXTENSION]);
        c++;

        if(!starkInfo.starkStruct.hashCommits) {
            transcript.put(&evals[0], starkInfo.evMap.size()  * FIELD_EXTENSION);
        } else {
            ElementType hash[nFieldElements];
            TranscriptType transcriptHash(starkInfo.starkStruct.merkleTreeArity, starkInfo.starkStruct.merkleTreeCustom);
            transcriptHash.put(&evals[0], starkInfo.evMap.size()  * FIELD_EXTENSION);
            transcriptHash.getState(hash);
            transcript.put(hash, nFieldElements);
        }

        // FRI challenges
        transcript.getField((uint64_t *)&challenges[c*FIELD_EXTENSION]);
        c++;
        transcript.getField((uint64_t *)&challenges[c*FIELD_EXTENSION]);
        c++;

        for (uint64_t step=0; step<starkInfo.starkStruct.steps.size(); step++) {
            transcript.getField((uint64_t *)&challenges[c*FIELD_EXTENSION]);
            c++;
            if (step < starkInfo.starkStruct.steps.size() - 1) {
                ElementType root[nFieldElements];
                if(nFieldElements == 1) {
                    root[0] = fromString<ElementType>(jproof["s" + std::to_string(step + 1) + "_root"]);
                } else {
                    for(uint64_t i = 0; i < nFieldElements; ++i) {
                        root[i] = fromString<ElementType>(jproof["s" + std::to_string(step + 1) + "_root"][i]);
                    }
                }
                
                transcript.put(&root[0], nFieldElements);
            } else {
                uint64_t finalPolSize = (1<< starkInfo.starkStruct.steps[step].nBits);
                Goldilocks::Element finalPol[finalPolSize * FIELD_EXTENSION];
                for(uint64_t i = 0; i < finalPolSize; ++i) {
                    for(uint64_t j = 0; j < FIELD_EXTENSION; ++j) {
                        finalPol[i*FIELD_EXTENSION + j] = Goldilocks::fromString(jproof["finalPol"][i][j]);
                    }
                }

                if(!starkInfo.starkStruct.hashCommits) {
                    transcript.put(&finalPol[0],finalPolSize*FIELD_EXTENSION);
                } else {
                    ElementType hash[nFieldElements];
                    TranscriptType transcriptHash(starkInfo.starkStruct.merkleTreeArity, starkInfo.starkStruct.merkleTreeCustom);
                    transcriptHash.put(&finalPol[0], finalPolSize*FIELD_EXTENSION);
                    transcriptHash.getState(hash);
                    transcript.put(hash, nFieldElements);
                }
            }
        }
        transcript.getField((uint64_t *)&challenges[c*FIELD_EXTENSION]);
        c++;
        assert(c == (starkInfo.challengesMap.size() + starkInfo.starkStruct.steps.size() + 1));
        
    } else {
        std::memcpy(challenges, challenges_, ((starkInfo.challengesMap.size() + starkInfo.starkStruct.steps.size() + 1) * FIELD_EXTENSION) * sizeof(Goldilocks::Element));
    }

    Goldilocks::Element *challenge = &challenges[(starkInfo.challengesMap.size() + starkInfo.starkStruct.steps.size()) * FIELD_EXTENSION];

    TranscriptType transcriptPermutation(starkInfo.starkStruct.merkleTreeArity, starkInfo.starkStruct.merkleTreeCustom);
    transcriptPermutation.put(challenge, FIELD_EXTENSION);
    transcriptPermutation.getPermutations(friQueries, starkInfo.starkStruct.nQueries, starkInfo.starkStruct.steps[0].nBits);

    Goldilocks::Element constPolsVals[starkInfo.nConstants * starkInfo.starkStruct.nQueries];
#pragma omp parallel for
    for(uint64_t q = 0; q < starkInfo.starkStruct.nQueries; ++q) {
        for(uint64_t i = 0; i < starkInfo.nConstants; ++i) {
            constPolsVals[q*starkInfo.nConstants + i] = Goldilocks::fromString(jproof["s0_valsC"][q][i]);
        }
    }
    
    Goldilocks::Element xiChallenge[FIELD_EXTENSION];

    for (uint64_t i = 0; i < starkInfo.challengesMap.size(); i++)
    {
        if(starkInfo.challengesMap[i].stage == starkInfo.nStages + 2) {
            if(starkInfo.challengesMap[i].stageId == 0) {
                std::memcpy(&xiChallenge[0], &challenges[i*FIELD_EXTENSION], FIELD_EXTENSION * sizeof(Goldilocks::Element));
            }
        }
    }

    ProverHelpers proverHelpers(starkInfo, xiChallenge);

    SetupCtx setupCtx(starkInfo, expressionsBin, proverHelpers);

    Goldilocks::Element *xDivXSub = new Goldilocks::Element[starkInfo.openingPoints.size() * FIELD_EXTENSION * starkInfo.starkStruct.nQueries];
    for(uint64_t i = 0; i < starkInfo.starkStruct.nQueries; ++i) {
        uint64_t query = friQueries[i];
        Goldilocks::Element x = Goldilocks::shift() * Goldilocks::exp(Goldilocks::w(starkInfo.starkStruct.nBitsExt), query);
        for(uint64_t o = 0; o < starkInfo.openingPoints.size(); ++o) {
            Goldilocks::Element w = Goldilocks::one();

            for(uint64_t j = 0; j < uint64_t(std::abs(starkInfo.openingPoints[o])); ++j) {
                w = w * Goldilocks::w(starkInfo.starkStruct.nBits);
            }
            if(starkInfo.openingPoints[o] < 0) {
                w = Goldilocks::inv(w);
            }
            
            Goldilocks::Element x_ext[FIELD_EXTENSION] = { x, Goldilocks::zero(), Goldilocks::zero() };
            Goldilocks::Element aux[FIELD_EXTENSION];
            Goldilocks3::mul((Goldilocks3::Element &)aux[0], (Goldilocks3::Element &)xiChallenge[0], w);
            Goldilocks3::sub((Goldilocks3::Element &)aux[0], (Goldilocks3::Element &)x_ext[0], (Goldilocks3::Element &)aux[0]);
            Goldilocks3::inv((Goldilocks3::Element *)aux, (Goldilocks3::Element *)aux);
            Goldilocks3::mul((Goldilocks3::Element &)aux[0], (Goldilocks3::Element &)aux[0], (Goldilocks3::Element &)x_ext[0]);
            std::memcpy(&xDivXSub[(i + o*starkInfo.starkStruct.nQueries)*FIELD_EXTENSION], &aux[0], FIELD_EXTENSION * sizeof(Goldilocks::Element));
        }
    }

    Goldilocks::Element *trace = new Goldilocks::Element[starkInfo.mapSectionsN["cm1"]*starkInfo.starkStruct.nQueries];
    Goldilocks::Element *aux_trace = new Goldilocks::Element[starkInfo.mapTotalN];
#pragma omp parallel for
    for(uint64_t q = 0; q < starkInfo.starkStruct.nQueries; ++q) {
        for(uint64_t i = 0; i < starkInfo.cmPolsMap.size(); ++i) {
            uint64_t stage = starkInfo.cmPolsMap[i].stage;
            uint64_t stagePos = starkInfo.cmPolsMap[i].stagePos;
            uint64_t offset = starkInfo.mapOffsets[std::make_pair("cm" + to_string(stage), false)];
            uint64_t nPols = starkInfo.mapSectionsN["cm" + to_string(stage)];
            Goldilocks::Element *pols = stage == 1 ? trace : aux_trace;
            if(starkInfo.cmPolsMap[i].dim == 1) {
                pols[offset + q*nPols + stagePos] = Goldilocks::fromString(jproof["s0_vals" + to_string(stage)][q][stagePos]);
            } else {
                pols[offset + q*nPols + stagePos] = Goldilocks::fromString(jproof["s0_vals" + to_string(stage)][q][stagePos]);
                pols[offset + q*nPols + stagePos + 1] = Goldilocks::fromString(jproof["s0_vals" + to_string(stage)][q][stagePos + 1]);
                pols[offset + q*nPols + stagePos + 2] = Goldilocks::fromString(jproof["s0_vals" + to_string(stage)][q][stagePos + 2]);
            }
        }
    }
    
    Goldilocks::Element *custom_commits[starkInfo.customCommits.size()];
    for(uint64_t c = 0; c < starkInfo.customCommits.size(); ++c) {
        custom_commits[c] = new Goldilocks::Element[starkInfo.customCommitsMap[c].size() * starkInfo.starkStruct.nQueries];
    }

#pragma omp parallel for
    for(uint64_t q = 0; q < starkInfo.starkStruct.nQueries; ++q) {
        for(uint64_t c = 0; c < starkInfo.customCommits.size(); ++c) {
            for(uint64_t i = 0; i < starkInfo.customCommitsMap[c].size(); ++i) {
                uint64_t stagePos = starkInfo.customCommitsMap[c][i].stagePos;
                uint64_t offset = starkInfo.mapOffsets[std::make_pair(starkInfo.customCommits[c].name + "0", false)];
                uint64_t nPols = starkInfo.mapSectionsN[starkInfo.customCommits[c].name + "0"];
                Goldilocks::Element *pols = custom_commits[c];
                if(starkInfo.customCommitsMap[c][i].dim == 1) {
                    pols[offset + q*nPols + stagePos] = Goldilocks::fromString(jproof["s0_vals_" + starkInfo.customCommits[c].name + "_0"][q][stagePos]);
                } else {
                    pols[offset + q*nPols + stagePos] = Goldilocks::fromString(jproof["s0_vals_" + starkInfo.customCommits[c].name + "_0"][q][stagePos]);
                    pols[offset + q*nPols + stagePos + 1] = Goldilocks::fromString(jproof["s0_vals_" + starkInfo.customCommits[c].name + "_0"][q][stagePos + 1]);
                    pols[offset + q*nPols + stagePos + 2] = Goldilocks::fromString(jproof["s0_vals_" + starkInfo.customCommits[c].name + "_0"][q][stagePos + 2]);
                }
            }
        }   
    }

    StepsParams params = {
        trace : trace,
        aux_trace: aux_trace,
        publicInputs : publics,
        proofValues: proofValues,
        challenges : challenges,
        airgroupValues : airgroupValues,
        airValues : airValues,
        evals : evals,
        xDivXSub : xDivXSub,
        pConstPolsAddress: constPolsVals,
        pConstPolsExtendedTreeAddress: nullptr,
        pCustomCommits: {nullptr, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr, nullptr},
    };

    for (uint64_t i = 0; i < starkInfo.customCommits.size(); ++i) {
        params.pCustomCommits[i] = custom_commits[i];
    }

    bool isValid = true;

    zklog.trace("Verifying evaluations");
    ExpressionsPack expressionsPack(setupCtx, 1);
    
    Goldilocks::Element buff[FIELD_EXTENSION];
    Dest dest(buff);
    dest.addParams(setupCtx.expressionsBin.expressionsInfo[starkInfo.cExpId]);
    std::vector<Dest> dests = {dest};
    
    expressionsPack.calculateExpressions(params, setupCtx.expressionsBin.expressionsBinArgsExpressions, dests, 1, false);

    Goldilocks::Element xN[3] = {Goldilocks::one(), Goldilocks::zero(), Goldilocks::zero()};
    for(uint64_t i = 0; i < uint64_t(1 << starkInfo.starkStruct.nBits); ++i) {
        Goldilocks3::mul((Goldilocks3::Element *)xN, (Goldilocks3::Element *)xN, (Goldilocks3::Element *)xiChallenge);
    }

    Goldilocks::Element xAcc[3] = { Goldilocks::one(), Goldilocks::zero(), Goldilocks::zero() };
    Goldilocks::Element q[3] = { Goldilocks::zero(), Goldilocks::zero(), Goldilocks::zero() };
    uint64_t qStage = starkInfo.nStages + 1;
    uint64_t qIndex = std::find_if(starkInfo.cmPolsMap.begin(), starkInfo.cmPolsMap.end(), [qStage](const PolMap& p) {
        return p.stage == qStage && p.stageId == 0;
    }) - starkInfo.cmPolsMap.begin();

    for(uint64_t i = 0; i < starkInfo.qDeg; ++i) {
        uint64_t index = qIndex + i;
        uint64_t evId = std::find_if(starkInfo.evMap.begin(), starkInfo.evMap.end(), [index](const EvMap& e) {
           return e.type == EvMap::eType::cm && e.id == index;
        }) - starkInfo.evMap.begin();
        Goldilocks::Element aux[3];
        Goldilocks3::mul((Goldilocks3::Element &)aux[0], (Goldilocks3::Element &)xAcc[0], (Goldilocks3::Element &)evals[evId * FIELD_EXTENSION]);
        Goldilocks3::add((Goldilocks3::Element &)q, (Goldilocks3::Element &)q, (Goldilocks3::Element &)aux[0]);
        Goldilocks3::mul((Goldilocks3::Element &)xAcc[0], (Goldilocks3::Element &)xAcc[0], (Goldilocks3::Element &)xN);
    }

    Goldilocks::Element res[3] = { q[0] - buff[0], q[1] - buff[1], q[2] - buff[2]};
    if(!Goldilocks::isZero(res[0]) || !Goldilocks::isZero(res[1]) || !Goldilocks::isZero(res[2])) {
        zklog.error("Invalid evaluations");
        isValid = false;
    }

    zklog.trace("Verifying FRI queries consistency");
    Goldilocks::Element buffQueries[FIELD_EXTENSION*starkInfo.starkStruct.nQueries];
    Dest destQueries(buffQueries);
    destQueries.addParams(setupCtx.expressionsBin.expressionsInfo[starkInfo.friExpId]);
    std::vector<Dest> destsQueries = {destQueries};
    expressionsPack.calculateExpressions(params, setupCtx.expressionsBin.expressionsBinArgsExpressions, destsQueries, starkInfo.starkStruct.nQueries, false);
    bool isValidFRIConsistency = true;
#pragma omp parallel for
    for(uint64_t q = 0; q < starkInfo.starkStruct.nQueries; ++q) {
        uint64_t idx = friQueries[q] % (1 << starkInfo.starkStruct.steps[0].nBits);
        if(starkInfo.starkStruct.steps.size() > 1) {
            uint64_t nextNGroups = 1 << starkInfo.starkStruct.steps[1].nBits;
            uint64_t groupIdx = idx / nextNGroups;
            if(!Goldilocks::isZero(Goldilocks::fromString(jproof["s1_vals"][q][groupIdx * FIELD_EXTENSION]) - buffQueries[q*FIELD_EXTENSION]) 
                || !Goldilocks::isZero(Goldilocks::fromString(jproof["s1_vals"][q][groupIdx * FIELD_EXTENSION + 1]) - buffQueries[q*FIELD_EXTENSION + 1]) 
                || !Goldilocks::isZero(Goldilocks::fromString(jproof["s1_vals"][q][groupIdx * FIELD_EXTENSION + 2]) - buffQueries[q*FIELD_EXTENSION + 2])) {
                isValidFRIConsistency = false;
            }
        } else {
            if(!Goldilocks::isZero(Goldilocks::fromString(jproof["finalPol"][idx][0]) - buffQueries[q*FIELD_EXTENSION]) 
                || !Goldilocks::isZero(Goldilocks::fromString(jproof["finalPol"][idx][1]) - buffQueries[q*FIELD_EXTENSION + 1]) 
                || !Goldilocks::isZero(Goldilocks::fromString(jproof["finalPol"][idx][2]) - buffQueries[q*FIELD_EXTENSION + 2])) {
                isValidFRIConsistency = false;
            }
        }
    }
    if(!isValidFRIConsistency) {
        isValid = false;
        zklog.error("Verify FRI query consistency failed");
    }

    for(uint64_t s = 0; s < starkInfo.nStages + 1; ++s) {
        zklog.trace("Verifying stage " +  to_string(s + 1) + " Merkle tree");
        std::string section = "cm" + to_string(s + 1);
        uint64_t nCols = starkInfo.mapSectionsN[section];
        MerkleTreeType tree(starkInfo.starkStruct.merkleTreeArity, starkInfo.starkStruct.merkleTreeCustom, 1 << starkInfo.starkStruct.nBitsExt, nCols);
        ElementType root[nFieldElements];
        if(nFieldElements == 1) {
            root[0] = fromString<ElementType>(jproof["root" + to_string(s + 1)]);
        } else {
            for(uint64_t j = 0; j < nFieldElements; ++j) {
                root[j] = fromString<ElementType>(jproof["root" + to_string(s + 1)][j]);
            }
        }
       
        bool isValidStageMT = true;
    #pragma omp parallel for
        for(uint64_t q = 0; q < starkInfo.starkStruct.nQueries; ++q) {
            std::vector<Goldilocks::Element> values(nCols);
            for (uint64_t i = 0; i < nCols; ++i) {
                values[i] = Goldilocks::fromString(jproof["s0_vals" + to_string(s + 1)][q][i]);
            }

            uint64_t nSiblings = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? std::floor((starkInfo.starkStruct.steps[0].nBits - 1) / std::ceil(std::log2(starkInfo.starkStruct.merkleTreeArity))) + 1 : starkInfo.starkStruct.steps[0].nBits;
            uint64_t nSiblingsPerLevel = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? starkInfo.starkStruct.merkleTreeArity : nFieldElements;
            
            std::vector<std::vector<ElementType>> siblings(
                nSiblings, 
                std::vector<ElementType>(nSiblingsPerLevel)
            );

            for (uint64_t i = 0; i < nSiblings; ++i) {
                for (uint64_t j = 0; j < nSiblingsPerLevel; ++j) {
                    siblings[i][j] = fromString<ElementType>(jproof["s0_siblings" + to_string(s + 1)][q][i][j]);
                }
            }

            bool res = tree.verifyGroupProof(root, siblings, friQueries[q], values);
            if(!res) {
                isValidStageMT = false;
            }
        }
        if(!isValidStageMT) {
            zklog.error("Stage " + to_string(s + 1) + " Merkle Tree verification failed");
            isValid = false;
        }
    }

    zklog.trace("Verifying constant Merkle tree");
    MerkleTreeType treeC(starkInfo.starkStruct.merkleTreeArity, starkInfo.starkStruct.merkleTreeCustom, 1 << starkInfo.starkStruct.nBitsExt, starkInfo.nConstants);
    bool isValidConstantMT = true;
#pragma omp parallel for
    for(uint64_t q = 0; q < starkInfo.starkStruct.nQueries; ++q) {
        std::vector<Goldilocks::Element> values(starkInfo.nConstants);
        for (uint64_t i = 0; i < starkInfo.nConstants; ++i) {
            values[i] = Goldilocks::fromString(jproof["s0_valsC"][q][i]);
        }

        uint64_t nSiblings = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? std::floor((starkInfo.starkStruct.steps[0].nBits - 1) / std::ceil(std::log2(starkInfo.starkStruct.merkleTreeArity))) + 1 : starkInfo.starkStruct.steps[0].nBits;
        uint64_t nSiblingsPerLevel = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? starkInfo.starkStruct.merkleTreeArity : nFieldElements;
        
        std::vector<std::vector<ElementType>> siblings(
            nSiblings, 
            std::vector<ElementType>(nSiblingsPerLevel)
        );

        for (uint64_t i = 0; i < nSiblings; ++i) {
            for (uint64_t j = 0; j < nSiblingsPerLevel; ++j) {
                siblings[i][j] = fromString<ElementType>(jproof["s0_siblingsC"][q][i][j]);
            }
        }

        bool res = treeC.verifyGroupProof(verkey, siblings, friQueries[q], values);
        if(!res) {
            isValidConstantMT = false;
        }
    }
    if(!isValidConstantMT) {
        zklog.error("Constant Merkle Tree verification failed");
        isValid = false;
    }

    for(uint64_t c = 0; c < starkInfo.customCommits.size(); ++c) {
        zklog.trace("Verifying custom commit " + starkInfo.customCommits[c].name + " Merkle tree");
        std::string section = starkInfo.customCommits[c].name + "0";
        uint64_t nCols = starkInfo.mapSectionsN[section];
        MerkleTreeType tree(starkInfo.starkStruct.merkleTreeArity, starkInfo.starkStruct.merkleTreeCustom, 1 << starkInfo.starkStruct.nBitsExt, nCols);
        ElementType root[nFieldElements];
        for(uint64_t j = 0; j < nFieldElements; ++j) {
            root[j] = fromString<ElementType>(Goldilocks::toString(publics[starkInfo.customCommits[c].publicValues[j]]));
        }
        bool isValidCustomCommitsMT = true;
    #pragma omp parallel for
        for(uint64_t q = 0; q < starkInfo.starkStruct.nQueries; ++q) {
            std::vector<Goldilocks::Element> values(nCols);
            for (uint64_t i = 0; i < nCols; ++i) {
                values[i] = Goldilocks::fromString(jproof["s0_vals_" + starkInfo.customCommits[c].name + "_0"][q][i]);
            }

            uint64_t nSiblings = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? std::floor((starkInfo.starkStruct.steps[0].nBits - 1) / std::ceil(std::log2(starkInfo.starkStruct.merkleTreeArity))) + 1 : starkInfo.starkStruct.steps[0].nBits;
            uint64_t nSiblingsPerLevel = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? starkInfo.starkStruct.merkleTreeArity : nFieldElements;

            std::vector<std::vector<ElementType>> siblings(
                nSiblings, 
                std::vector<ElementType>(nSiblingsPerLevel)
            );

            for (uint64_t i = 0; i < nSiblings; ++i) {
                for (uint64_t j = 0; j < nSiblingsPerLevel; ++j) {
                    siblings[i][j] = fromString<ElementType>(jproof["s0_siblings_" + starkInfo.customCommits[c].name + "_0"][q][i][j]);
                }
            }
            bool res = tree.verifyGroupProof(root, siblings, friQueries[q], values);
            if(!res) {
                isValidCustomCommitsMT = false;
            }
        }
        if(!isValidCustomCommitsMT) {
            zklog.error("Custom Commit " + starkInfo.customCommits[c].name + " Merkle Tree verification failed");
            isValid = false;
        }
    }


    zklog.trace("Verifying FRI foldings Merkle Trees");
    for (uint64_t step=1; step< starkInfo.starkStruct.steps.size(); step++) {
        uint64_t nGroups = 1 << starkInfo.starkStruct.steps[step].nBits;
        uint64_t groupSize = (1 << starkInfo.starkStruct.steps[step - 1].nBits) / nGroups;
        MerkleTreeType treeFRI(starkInfo.starkStruct.merkleTreeArity, starkInfo.starkStruct.merkleTreeCustom, nGroups, groupSize * FIELD_EXTENSION);
        ElementType root[nFieldElements];
        if (nFieldElements == 1) {
            root[0] = fromString<ElementType>(jproof["s" + std::to_string(step) + "_root"]);
        } else {
            for(uint64_t j = 0; j < nFieldElements; ++j) {
                root[j] = fromString<ElementType>(jproof["s" + std::to_string(step) + "_root"][j]);
            }
        }

        bool isValidFoldingMT = true;
    #pragma omp parallel for
        for(uint64_t q = 0; q < starkInfo.starkStruct.nQueries; ++q) {
            uint64_t n_values = (1 << (starkInfo.starkStruct.steps[step-1].nBits - starkInfo.starkStruct.steps[step].nBits))*FIELD_EXTENSION;
            std::vector<Goldilocks::Element> values(n_values);
            for (uint64_t i = 0; i < n_values; ++i) {
                values[i] = Goldilocks::fromString(jproof["s" + std::to_string(step) + "_vals"][q][i]);
            }

            uint64_t nSiblings = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? std::floor((starkInfo.starkStruct.steps[step].nBits - 1) / std::ceil(std::log2(starkInfo.starkStruct.merkleTreeArity))) + 1 : starkInfo.starkStruct.steps[step].nBits;
            uint64_t nSiblingsPerLevel = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? starkInfo.starkStruct.merkleTreeArity : nFieldElements;

            std::vector<std::vector<ElementType>> siblings(
                nSiblings, 
                std::vector<ElementType>(nSiblingsPerLevel)
            );

            for (uint64_t i = 0; i < nSiblings; ++i) {
                for (uint64_t j = 0; j < nSiblingsPerLevel; ++j) {
                    siblings[i][j] = fromString<ElementType>(jproof["s" + std::to_string(step) + "_siblings"][q][i][j]);
                }
            }
            bool res = treeFRI.verifyGroupProof(root, siblings, friQueries[q], values);
            if(!res) {
                isValidFoldingMT = false;
            }
        }
        if(!isValidFoldingMT) {
            zklog.error("FRI folding Merkle Tree verification failed");
            isValid = false;
        }
    }

    zklog.trace("Verifying FRI foldings");
    for (uint64_t step=1; step < starkInfo.starkStruct.steps.size(); step++) {
        bool isValidFolding = true;
    #pragma omp parallel for
        for(uint64_t q = 0; q < starkInfo.starkStruct.nQueries; ++q) {
            uint64_t idx = friQueries[q] % (1 << starkInfo.starkStruct.steps[step].nBits);     
            Goldilocks::Element value[3];
            uint64_t n_values = (1 << (starkInfo.starkStruct.steps[step-1].nBits - starkInfo.starkStruct.steps[step].nBits))*FIELD_EXTENSION;
            std::vector<Goldilocks::Element> values(n_values);
            for (uint64_t i = 0; i < n_values; ++i) {
                values[i] = Goldilocks::fromString(jproof["s" + std::to_string(step) + "_vals"][q][i]);
            }
            FRI<ElementType>::verify_fold(
                value,
                step, 
                starkInfo.starkStruct.nBitsExt, 
                starkInfo.starkStruct.steps[step].nBits, 
                starkInfo.starkStruct.steps[step - 1].nBits,
                &challenges[(starkInfo.challengesMap.size() + step)*FIELD_EXTENSION],
                idx,
                values
            );
            if (step < starkInfo.starkStruct.steps.size() - 1) {
                uint64_t groupIdx = idx / (1 << starkInfo.starkStruct.steps[step + 1].nBits);
                for(uint64_t i = 0; i < FIELD_EXTENSION; ++i) {
                    if(!Goldilocks::isZero(value[i] - Goldilocks::fromString(jproof["s" + to_string(step + 1) + "_vals"][q][groupIdx * FIELD_EXTENSION + i]))) {
                        isValidFolding = false;
                    }
                }
            } else {
                for(uint64_t i = 0; i < FIELD_EXTENSION; ++i) {
                    if(!Goldilocks::isZero(value[i] - Goldilocks::fromString(jproof["finalPol"][idx][i]))) {
                        isValidFolding = false;
                    }
                }
            }
        }
        if(!isValidFolding) {
            zklog.error("FRI folding verification failed");
            isValid = false;
        }
    }
    
    delete xDivXSub;
    delete trace;
    delete aux_trace;

    for(uint64_t c = 0; c < starkInfo.customCommits.size(); ++c) {
        delete[] custom_commits[c];
    }

    return isValid;
}


