#ifndef PROOF
#define PROOF

#include "goldilocks_base_field.hpp"
#include "poseidon_goldilocks.hpp"
#include "stark_info.hpp"
#include "fr.hpp"
#include <vector>
#include "nlohmann/json.hpp"

using ordered_json = nlohmann::ordered_json;

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

    ordered_json merkleProof2json()
    {
        ordered_json j = ordered_json::array();
        ordered_json json_v = ordered_json::array();
        for (uint i = 0; i < v.size(); i++)
        {
            if (v[i].size() > 1)
            {
                ordered_json element = ordered_json::array();
                for (uint j = 0; j < v[i].size(); j++)
                {
                    element.push_back(Goldilocks::toString(v[i][j]));
                }
                json_v.push_back(element);
            }
            else
            {
                json_v.push_back(Goldilocks::toString(v[i][0]));
            }
        }
        j.push_back(json_v);

        ordered_json json_mp = ordered_json::array();
        for (uint i = 0; i < mp.size(); i++)
        {
            ordered_json element = ordered_json::array();
            for (uint j = 0; j < mp[i].size(); j++)
            {
                element.push_back(toString(mp[i][j]));
            }
            json_mp.push_back(element);
        }
        j.push_back(json_mp);
        return j;
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

    ordered_json ProofTree2json(bool friQueries = true)
    {
        ordered_json j_ProofTree2json = ordered_json::object();

        if(friQueries) {
             ordered_json json_root = ordered_json::array();
            if(root.size() == 1) {
                j_ProofTree2json["root"] = toString(root[0]);
            } else {
                for (uint i = 0; i < root.size(); i++)
                {
                    json_root.push_back(toString(root[i]));
                }
                j_ProofTree2json["root"] = json_root;
            }
        }

        ordered_json json_polQueries = ordered_json::array();
        for (uint i = 0; i < polQueries.size(); i++)
        {
            ordered_json element = ordered_json::array();
            if (polQueries[i].size() != 1)
            {
                for (uint j = 0; j < polQueries[i].size(); j++)
                {
                    element.push_back(polQueries[i][j].merkleProof2json());
                }
                json_polQueries.push_back(element);
            }
            else
            {
                json_polQueries.push_back(polQueries[i][0].merkleProof2json());
            }
        }

        j_ProofTree2json["polQueries"] = json_polQueries;

        return j_ProofTree2json;
    }
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
    ordered_json QueriesP2json()
    {
        return trees.ProofTree2json(false);
    }

    ordered_json FriQueriesP2json()
    {
        ordered_json j = ordered_json::array();

        for (uint i = 0; i < treesFRI.size(); i++)
        {
            j.push_back((treesFRI[i].ProofTree2json()));
        }

        ordered_json json_pol = ordered_json::array();
        for (uint i = 0; i < pol.size(); i++)
        {
            ordered_json element = ordered_json::array();
            for (uint j = 0; j < pol[i].size(); j++)
            {
                element.push_back(Goldilocks::toString(pol[i][j]));
            }
            json_pol.push_back(element);
        }
        j.push_back(json_pol);
        return j;
    }
};

template <typename ElementType>
class Proofs
{
public:
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
    Proofs(StarkInfo &starkInfo) :
        fri(starkInfo),
        evals(starkInfo.evMap.size(), std::vector<Goldilocks::Element>(FIELD_EXTENSION, Goldilocks::zero())),
        airgroupValues(starkInfo.airgroupValuesMap.size(), std::vector<Goldilocks::Element>(FIELD_EXTENSION, Goldilocks::zero())),
        airValues(starkInfo.airValuesMap.size(), std::vector<Goldilocks::Element>(FIELD_EXTENSION, Goldilocks::zero())),
        customCommits(starkInfo.customCommits.size())
        {
            nStages = starkInfo.nStages + 1;
            nCustomCommits = starkInfo.customCommits.size();
            roots = new ElementType*[nStages + nCustomCommits];
            nFieldElements = starkInfo.starkStruct.verificationHashType == "GL" ? HASH_SIZE : 1;
            airId = starkInfo.airId;
            airgroupId = starkInfo.airgroupId;
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
        for (uint64_t i = 0; i < airgroupValues.size(); i++)
        {
            std::memcpy(&airgroupValues[i][0], &_airgroupValues[i * FIELD_EXTENSION], FIELD_EXTENSION * sizeof(Goldilocks::Element));
        }
    }

    void setAirValues(Goldilocks::Element *_airValues) {
        for (uint64_t i = 0; i < airValues.size(); i++)
        {
            std::memcpy(&airValues[i][0], &_airValues[i * FIELD_EXTENSION], FIELD_EXTENSION * sizeof(Goldilocks::Element));
        }
    }

    ordered_json proof2json()
    {
        ordered_json j = ordered_json::object();

        j["airId"] = airId;
        j["airgroupId"] = airgroupId;
        
        for(uint64_t i = 0; i < nStages; i++) {
            ordered_json json_root = ordered_json::array();
            if(nFieldElements == 1) {
                j["root" + to_string(i + 1)] = toString(roots[i][0]);
            } else {
                for (uint k = 0; k < nFieldElements; k++)
                {
                    json_root.push_back(toString(roots[i][k]));
                }
                j["root" + to_string(i + 1)] = json_root;
            }
        }

        ordered_json json_evals = ordered_json::array();
        for (uint i = 0; i < evals.size(); i++)
        {
            ordered_json element = ordered_json::array();
            for (uint j = 0; j < evals[i].size(); j++)
            {
                element.push_back(Goldilocks::toString(evals[i][j]));
            }
            json_evals.push_back(element);
        }
        j["evals"] = json_evals;

        ordered_json json_airgroupValues = ordered_json::array();
        for (uint i = 0; i < airgroupValues.size(); i++)
        {
            ordered_json element = ordered_json::array();
            for (uint j = 0; j < airgroupValues[i].size(); j++)
            {
                element.push_back(Goldilocks::toString(airgroupValues[i][j]));
            }
            json_airgroupValues.push_back(element);
        }

        j["airgroupValues"] = json_airgroupValues;

        ordered_json json_airValues = ordered_json::array();
        for (uint i = 0; i < airValues.size(); i++)
        {
            ordered_json element = ordered_json::array();
            for (uint j = 0; j < airValues[i].size(); j++)
            {
                element.push_back(Goldilocks::toString(airValues[i][j]));
            }
            json_airValues.push_back(element);
        }

        j["airValues"] = json_airValues;
        
        j["queries"] = fri.QueriesP2json();

        j["fri"] = fri.FriQueriesP2json();

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

    FRIProof(StarkInfo &starkInfo) : proof(starkInfo), publics(starkInfo.nPublics) {
        airId = starkInfo.airId;
        airgroupId = starkInfo.airgroupId;
    };
};

#endif