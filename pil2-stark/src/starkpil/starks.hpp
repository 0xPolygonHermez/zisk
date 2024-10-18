#ifndef STARKS_HPP
#define STARKS_HPP

#include <algorithm>
#include <cmath>
#include "utils.hpp"
#include "timer.hpp"
#include "const_pols.hpp"
#include "proof_stark.hpp"
#include "fri.hpp"
#include "transcriptGL.hpp"
#include "steps.hpp"
#include "zklog.hpp"
#include "merkleTreeBN128.hpp"
#include "transcriptBN128.hpp"
#include "exit_process.hpp"
#include "expressions_bin.hpp"
#include "expressions_avx.hpp"
#include "expressions_avx512.hpp"
#include "expressions_pack.hpp"


template <typename ElementType>
class Starks
{
public:
    SetupCtx& setupCtx;    
    using TranscriptType = std::conditional_t<std::is_same<ElementType, Goldilocks::Element>::value, TranscriptGL, TranscriptBN128>;
    using MerkleTreeType = std::conditional_t<std::is_same<ElementType, Goldilocks::Element>::value, MerkleTreeGL, MerkleTreeBN128>;

    MerkleTreeType **treesGL;
    MerkleTreeType **treesFRI;

public:
    Starks(SetupCtx& setupCtx_) : setupCtx(setupCtx_)                                                   
    {
        treesGL = new MerkleTreeType*[setupCtx.starkInfo.nStages + 2];
        treesGL[setupCtx.starkInfo.nStages + 1] = new MerkleTreeType(setupCtx.starkInfo.starkStruct.merkleTreeArity, setupCtx.starkInfo.starkStruct.merkleTreeCustom, (Goldilocks::Element *)setupCtx.constPols.pConstTreeAddress);
        for (uint64_t i = 0; i < setupCtx.starkInfo.nStages + 1; i++)
        {
            std::string section = "cm" + to_string(i + 1);
            uint64_t nCols = setupCtx.starkInfo.mapSectionsN[section];
            treesGL[i] = new MerkleTreeType(setupCtx.starkInfo.starkStruct.merkleTreeArity, setupCtx.starkInfo.starkStruct.merkleTreeCustom, 1 << setupCtx.starkInfo.starkStruct.nBitsExt, nCols, NULL, false);
        }

        treesFRI = new MerkleTreeType*[setupCtx.starkInfo.starkStruct.steps.size() - 1];
        for(uint64_t step = 0; step < setupCtx.starkInfo.starkStruct.steps.size() - 1; ++step) {
            uint64_t nGroups = 1 << setupCtx.starkInfo.starkStruct.steps[step + 1].nBits;
            uint64_t groupSize = (1 << setupCtx.starkInfo.starkStruct.steps[step].nBits) / nGroups;

            treesFRI[step] = new MerkleTreeType(setupCtx.starkInfo.starkStruct.merkleTreeArity, setupCtx.starkInfo.starkStruct.merkleTreeCustom, nGroups, groupSize * FIELD_EXTENSION, NULL);
        }
    };
    ~Starks()
    {
        for (uint i = 0; i < setupCtx.starkInfo.nStages + 2; i++)
        {
            delete treesGL[i];
        }
        delete[] treesGL;

        for (uint64_t i = 0; i < setupCtx.starkInfo.starkStruct.steps.size() - 1; i++)
        {
            delete treesFRI[i];
        }
        delete[] treesFRI;
        
    };
    
    void extendAndMerkelize(uint64_t step, Goldilocks::Element *buffer, FRIProof<ElementType> &proof, Goldilocks::Element* pBuffHelper = nullptr);

    void commitStage(uint64_t step, Goldilocks::Element *buffer, FRIProof<ElementType> &proof, Goldilocks::Element* pBuffHelper = nullptr);
    void computeQ(uint64_t step, Goldilocks::Element *buffer, FRIProof<ElementType> &proof, Goldilocks::Element* pBuffHelper = nullptr);
    
    void calculateImPolsExpressions(uint64_t step, Goldilocks::Element *buffer, Goldilocks::Element *publicInputs, Goldilocks::Element *challenges, Goldilocks::Element *subproofValues, Goldilocks::Element *evals);
    void calculateQuotientPolynomial(Goldilocks::Element *buffer, Goldilocks::Element *publicInputs, Goldilocks::Element *challenges, Goldilocks::Element *subproofValues, Goldilocks::Element *evals);
    void calculateFRIPolynomial(Goldilocks::Element *buffer, Goldilocks::Element *publicInputs, Goldilocks::Element *challenges, Goldilocks::Element *subproofValues, Goldilocks::Element *evals, Goldilocks::Element *xDivXSub);

    void computeLEv(Goldilocks::Element *xiChallenge, Goldilocks::Element *LEv);
    void computeEvals(Goldilocks::Element *buffer, Goldilocks::Element *LEv, Goldilocks::Element *evals, FRIProof<ElementType> &proof);

    void calculateXDivXSub(Goldilocks::Element *xiChallenge, Goldilocks::Element *xDivXSub);

    void calculateHash(ElementType* hash, Goldilocks::Element* buffer, uint64_t nElements);

    void addTranscriptGL(TranscriptType &transcript, Goldilocks::Element* buffer, uint64_t nElements);
    void addTranscript(TranscriptType &transcript, ElementType* buffer, uint64_t nElements);
    void getChallenge(TranscriptType &transcript, Goldilocks::Element& challenge);


    // Following function are created to be used by the ffi interface
    void ffi_treesGL_get_root(uint64_t index, ElementType *dst);

    void evmap(Goldilocks::Element *buffer, Goldilocks::Element *evals, Goldilocks::Element *LEv);
};

template class Starks<Goldilocks::Element>;
template class Starks<RawFr::Element>;

#endif // STARKS_H
