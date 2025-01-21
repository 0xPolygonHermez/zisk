#ifndef PROOF
#define PROOF

#include "goldilocks_base_field.hpp"
#include "poseidon_goldilocks.hpp"
#include "stark_info.hpp"
#include "fr.hpp"
#include <vector>
#include "nlohmann/json.hpp"

using json = nlohmann::json;

template <typename ElementType>
std::string toString(const ElementType& element);

template<>
inline std::string toString(const Goldilocks::Element& element) {
    return Goldilocks::toString(element);
}

template<>
inline std::string toString(const RawFr::Element& element) {
    return RawFr::field.toString(element, 10);
}

template <typename ElementType>
class MerkleProof
{
public:
    std::vector<std::vector<Goldilocks::Element>> v;
    std::vector<std::vector<ElementType>> mp;

    MerkleProof(uint64_t nLinears, uint64_t elementsTree, uint64_t numSiblings, void *pointer) : v(nLinears, std::vector<Goldilocks::Element>(1, Goldilocks::zero())), mp(elementsTree, std::vector<ElementType>(numSiblings))
    {
        for (uint64_t i = 0; i < nLinears; i++)
        {
            std::memcpy(&v[i][0], &((Goldilocks::Element *)pointer)[i], sizeof(Goldilocks::Element));
        }
        ElementType *mpCursor = (ElementType *)&((Goldilocks::Element *)pointer)[nLinears];
        for (uint64_t j = 0; j < elementsTree; j++)
        {
            std::memcpy(&mp[j][0], &mpCursor[j * numSiblings], numSiblings * sizeof(ElementType));
        }
    }
};

template <typename ElementType>
class ProofTree
{
public:
    std::vector<ElementType> root;
    std::vector<std::vector<MerkleProof<ElementType>>> polQueries;

    uint64_t nFieldElements;

    ProofTree(uint64_t nFieldElements_, uint64_t nQueries) : root(nFieldElements_), polQueries(nQueries), nFieldElements(nFieldElements_) {}

    void setRoot(ElementType *_root)
    {
        std::memcpy(&root[0], &_root[0], nFieldElements * sizeof(ElementType));
    };
};

template <typename ElementType>
class Fri
{
public:
    ProofTree<ElementType> trees;
    std::vector<ProofTree<ElementType>> treesFRI;
    std::vector<std::vector<Goldilocks::Element>> pol;
   

    Fri(StarkInfo &starkInfo) :  trees((starkInfo.starkStruct.verificationHashType == "GL") ? HASH_SIZE : 1, starkInfo.starkStruct.nQueries),
                                 treesFRI(),
                                 pol(1 << starkInfo.starkStruct.steps[starkInfo.starkStruct.steps.size() - 1].nBits, std::vector<Goldilocks::Element>(FIELD_EXTENSION, Goldilocks::zero())) {
        uint64_t nQueries = starkInfo.starkStruct.nQueries;
        uint64_t nFieldElements = (starkInfo.starkStruct.verificationHashType == "GL") ? HASH_SIZE : 1;
       
        for (size_t i = 0; i < starkInfo.starkStruct.steps.size() - 1; i++)
        {
            treesFRI.emplace_back(nFieldElements, nQueries);
        }
    }

    void setPol(Goldilocks::Element *pPol, uint64_t degree)
    {
        for (uint64_t i = 0; i < degree; i++)
        {
            std::memcpy(&pol[i][0], &pPol[i * FIELD_EXTENSION], FIELD_EXTENSION * sizeof(Goldilocks::Element));
        }
    }
};

template <typename ElementType>
class Proofs
{
public:
    StarkInfo &starkInfo;
    uint64_t nStages;
    uint64_t nCustomCommits;
    uint64_t nFieldElements;
    uint64_t airId;
    uint64_t airgroupId;
    ElementType **roots;
    Fri<ElementType> fri;
    std::vector<std::vector<Goldilocks::Element>> evals;
    std::vector<std::vector<Goldilocks::Element>> airgroupValues;
    std::vector<std::vector<Goldilocks::Element>> airValues;
    std::vector<std::string> customCommits;
    Proofs(StarkInfo &starkInfo_) :
        starkInfo(starkInfo_),
        fri(starkInfo_),
        evals(starkInfo_.evMap.size(), std::vector<Goldilocks::Element>(FIELD_EXTENSION, Goldilocks::zero())),
        airgroupValues(starkInfo_.airgroupValuesMap.size(), std::vector<Goldilocks::Element>(FIELD_EXTENSION, Goldilocks::zero())),
        airValues(starkInfo_.airValuesMap.size(), std::vector<Goldilocks::Element>(FIELD_EXTENSION, Goldilocks::zero())),
        customCommits(starkInfo_.customCommits.size())
        {
            nStages = starkInfo_.nStages + 1;
            nCustomCommits = starkInfo_.customCommits.size();
            roots = new ElementType*[nStages + nCustomCommits];
            nFieldElements = starkInfo_.starkStruct.verificationHashType == "GL" ? HASH_SIZE : 1;
            airId = starkInfo_.airId;
            airgroupId = starkInfo_.airgroupId;
            for(uint64_t i = 0; i < nStages + nCustomCommits; i++)
            {
                roots[i] = new ElementType[nFieldElements];
            }
            for(uint64_t i = 0; i < nCustomCommits; ++i) {
                customCommits[i] = starkInfo.customCommits[i].name;    
            }
        };

    ~Proofs() {
        for (uint64_t i = 0; i < nStages + nCustomCommits; ++i) {
            delete[] roots[i];
        }
        delete[] roots;
    }

    void setEvals(Goldilocks::Element *_evals)
    {
        for (uint64_t i = 0; i < evals.size(); i++)
        {
            std::memcpy(&evals[i][0], &_evals[i * evals[i].size()], evals[i].size() * sizeof(Goldilocks::Element));
        }
    }

    void setAirgroupValues(Goldilocks::Element *_airgroupValues) {
        uint64_t p = 0;
        for (uint64_t i = 0; i < starkInfo.airgroupValuesMap.size(); i++)
        {
            if(starkInfo.airgroupValuesMap[i].stage == 1) {
                airgroupValues[i][0] = _airgroupValues[p++];
                airgroupValues[i][1] = Goldilocks::zero();
                airgroupValues[i][2] = Goldilocks::zero();
            } else {
                std::memcpy(&airgroupValues[i][0], &_airgroupValues[p], FIELD_EXTENSION * sizeof(Goldilocks::Element));
                p += 3;
            }
        }
    }

    void setAirValues(Goldilocks::Element *_airValues) {
        uint64_t p = 0;
        for (uint64_t i = 0; i < starkInfo.airValuesMap.size(); i++)
        {
            if(starkInfo.airValuesMap[i].stage == 1) {
                airValues[i][0] = _airValues[p++];
                airValues[i][1] = Goldilocks::zero();
                airValues[i][2] = Goldilocks::zero();
            } else {
                std::memcpy(&airValues[i][0], &_airValues[p], FIELD_EXTENSION * sizeof(Goldilocks::Element));
                p += 3;
            }
        }
    }

    json proof2json()
    {
        json j = json::object();
        
        for(uint64_t i = 0; i < nStages; i++) {
            if(nFieldElements == 1) {
                j["root" + to_string(i + 1)] = toString(roots[i][0]);
            } else {
                j["root" + to_string(i + 1)] = json::array();
                for (uint k = 0; k < nFieldElements; k++)
                {
                    j["root" + to_string(i + 1)][k] = toString(roots[i][k]);
                }
            }
        }

        j["evals"] = json::array();
        for (uint i = 0; i < evals.size(); i++)
        {
            j["evals"][i] = json::array();
            for (uint k = 0; k < FIELD_EXTENSION; k++)
            {
                j["evals"][i][k] = Goldilocks::toString(evals[i][k]);
            }
        }

        if(airgroupValues.size() > 0) {
            j["airgroupvalues"] = json::array();
            for (uint i = 0; i < airgroupValues.size(); i++)
            {
                j["airgroupvalues"][i] = json::array();
                for (uint k = 0; k < FIELD_EXTENSION; k++)
                {
                    j["airgroupvalues"][i][k] = Goldilocks::toString(airgroupValues[i][k]);
                }
            }
        }

        if(airValues.size() > 0) {
            j["airvalues"] = json::array();
            for (uint i = 0; i < airValues.size(); i++)
            {
                j["airvalues"][i] = json::array();
                for (uint k = 0; k < airValues[i].size(); k++)
                {
                    j["airvalues"][i][k] = Goldilocks::toString(airValues[i][k]);
                }
            }
        }

        
        j["s0_valsC"] = json::array();
        j["s0_siblingsC"] = json::array();

        for(uint64_t i = 0; i < starkInfo.nStages + 1; ++i) {
            uint64_t stage = i + 1;
            j["s0_siblings" + to_string(stage)] = json::array();
            j["s0_vals" + to_string(stage)] = json::array();
        }

        for(uint64_t i = 0; i < starkInfo.customCommits.size(); ++i) {
            j["s0_siblings_" + starkInfo.customCommits[i].name + "_0"] = json::array();
            j["s0_vals_" + starkInfo.customCommits[i].name + "_0"] = json::array();
        }

        for (uint64_t i = 0; i < starkInfo.starkStruct.nQueries; i++) {
            uint64_t nSiblings = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? std::floor((starkInfo.starkStruct.steps[0].nBits - 1) / std::ceil(std::log2(starkInfo.starkStruct.merkleTreeArity))) + 1 : starkInfo.starkStruct.steps[0].nBits;
            uint64_t nSiblingsPerLevel = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? starkInfo.starkStruct.merkleTreeArity : nFieldElements;

            j["s0_valsC"][i] = json::array();
            j["s0_siblingsC"][i] = json::array();
            for(uint64_t l = 0; l < starkInfo.nConstants; l++) {
                j["s0_valsC"][i][l] = Goldilocks::toString(fri.trees.polQueries[i][starkInfo.nStages + 1].v[l][0]);
            }
            for(uint64_t l = 0; l < nSiblings; ++l) {
                for(uint64_t k = 0; k < nSiblingsPerLevel; ++k) {
                    j["s0_siblingsC"][i][l][k] = toString(fri.trees.polQueries[i][starkInfo.nStages + 1].mp[l][k]);
                }
            }

            for (uint64_t s = 0; s < nStages; ++s) {
                uint64_t stage = s + 1;
                j["s0_vals" + to_string(stage)][i] = json::array();
                for(uint64_t l = 0; l < starkInfo.mapSectionsN["cm" + to_string(stage)]; l++) {
                    j["s0_vals" + to_string(stage)][i][l] = Goldilocks::toString(fri.trees.polQueries[i][s].v[l][0]);
                }

                j["s0_siblings" + to_string(stage)][i] = json::array();
                for(uint64_t l = 0; l < nSiblings; ++l) {
                    for(uint64_t k = 0; k < nSiblingsPerLevel; ++k) {
                        j["s0_siblings" + to_string(stage)][i][l][k] = toString(fri.trees.polQueries[i][s].mp[l][k]);
                    }
                }
            }

            for(uint64_t c = 0; c < starkInfo.customCommits.size(); ++c) {
                j["s0_siblings_" + starkInfo.customCommits[c].name + "_0"][i] = json::array();
                j["s0_vals_" + starkInfo.customCommits[c].name + "_0"][i] = json::array();

                for(uint64_t l = 0; l < starkInfo.mapSectionsN[starkInfo.customCommits[c].name + "0"]; l++) {
                    j["s0_vals_" + starkInfo.customCommits[c].name + "_0"][i][l] = Goldilocks::toString(fri.trees.polQueries[i][starkInfo.nStages + 2 + c].v[l][0]);
                }
                for(uint64_t l = 0; l < nSiblings; ++l) {
                    for(uint64_t k = 0; k < nSiblingsPerLevel; ++k) {
                        j["s0_siblings_" + starkInfo.customCommits[c].name + "_0"][i][l][k] = toString(fri.trees.polQueries[i][starkInfo.nStages + 2 + c].mp[l][k]);
                    }
                }
            }
        }

        for(uint64_t step = 1; step < starkInfo.starkStruct.steps.size(); ++step) {
            j["s" + std::to_string(step) + "_root"] = json::array();
            for(uint64_t i = 0; i < nFieldElements; i++) {
                j["s" + std::to_string(step) + "_root"][i] = toString(fri.treesFRI[step - 1].root[i]);
            }
            j["s" + std::to_string(step) + "_vals"] = json::array();
            j["s" + std::to_string(step) + "_siblings"] = json::array();
        }

        for(uint64_t i = 0; i < starkInfo.starkStruct.nQueries; i++) {
            for(uint64_t step = 1; step < starkInfo.starkStruct.steps.size(); ++step) {
                j["s" + std::to_string(step) + "_vals"][i] = json::array();
                j["s" + std::to_string(step) + "_siblings"][i] = json::array();

                for(uint64_t l = 0; l < uint64_t(1 << (starkInfo.starkStruct.steps[step - 1].nBits - starkInfo.starkStruct.steps[step].nBits)) * FIELD_EXTENSION; l++) {
                    j["s" + std::to_string(step) + "_vals"][i][l] = Goldilocks::toString(fri.treesFRI[step - 1].polQueries[i][0].v[l][0]);
                }

                uint64_t nSiblings = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? std::floor((starkInfo.starkStruct.steps[step].nBits - 1) / std::ceil(std::log2(starkInfo.starkStruct.merkleTreeArity))) + 1 : starkInfo.starkStruct.steps[step].nBits;
                uint64_t nSiblingsPerLevel = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? starkInfo.starkStruct.merkleTreeArity : nFieldElements;

                for(uint64_t l = 0; l < nSiblings; ++l) {
                    for(uint64_t k = 0; k < nSiblingsPerLevel; ++k) {
                        j["s" + std::to_string(step) + "_siblings"][i][l][k] = toString(fri.treesFRI[step - 1].polQueries[i][0].mp[l][k]);
                    }
                }
            }
        }
        

        j["finalPol"] = json::array();
        for (uint64_t i = 0; i < uint64_t (1 << (starkInfo.starkStruct.steps[starkInfo.starkStruct.steps.size() - 1].nBits)); i++)
        {
            j["finalPol"][i] = json::array();
            for(uint64_t l = 0; l < FIELD_EXTENSION; l++) {
                j["finalPol"][i][l] = Goldilocks::toString(fri.pol[i][l]);
            }
        }
        return j;
    }
};

template <typename ElementType>
class FRIProof
{
public:
    Proofs<ElementType> proof;
    std::vector<ElementType> publics;
    
    uint64_t airId;
    uint64_t airgroupId;
    uint64_t instanceId;

    FRIProof(StarkInfo &starkInfo, uint64_t _instanceId) : proof(starkInfo), publics(starkInfo.nPublics) {
        airId = starkInfo.airId;
        airgroupId = starkInfo.airgroupId;
        instanceId = _instanceId;
    };
};

#endif