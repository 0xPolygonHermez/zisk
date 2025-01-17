#ifndef FRI_HPP
#define FRI_HPP

#include "proof_stark.hpp"
#include <cassert>
#include <vector>
#include "ntt_goldilocks.hpp"
#include "merklehash_goldilocks.hpp"
#include "merkleTreeGL.hpp"
#include "merkleTreeBN128.hpp"

template <typename ElementType>
class FRI
{
public:
    using MerkleTreeType = std::conditional_t<std::is_same<ElementType, Goldilocks::Element>::value, MerkleTreeGL, MerkleTreeBN128>;

    static void fold(uint64_t step, Goldilocks::Element *pol, Goldilocks::Element *challenge, uint64_t nBitsExt, uint64_t prevBits, uint64_t currentBits);
    static void merkelize(uint64_t step, FRIProof<ElementType> &proof, Goldilocks::Element* pol, MerkleTreeType* treeFRI, uint64_t currentBits, uint64_t nextBits);
    static void proveQueries(uint64_t* friQueries, uint64_t nQueries, FRIProof<ElementType> &fproof, MerkleTreeType **trees, uint64_t nTrees);
    static void proveFRIQueries(uint64_t* friQueries, uint64_t nQueries, uint64_t step, uint64_t currentBits, FRIProof<ElementType> &fproof, MerkleTreeType *treeFRI);
    static void setFinalPol(FRIProof<ElementType> &fproof, Goldilocks::Element* buffer, uint64_t nBits);
    static void verify_fold(Goldilocks::Element* value, uint64_t step, uint64_t nBitsExt, uint64_t currentBits, uint64_t prevBits, Goldilocks::Element *challenge, uint64_t idx, std::vector<Goldilocks::Element> &v);
private:
    static vector<MerkleProof<ElementType>> queryPol(MerkleTreeType *trees[], uint64_t nTrees, uint64_t idx, ElementType* buff);
    static vector<MerkleProof<ElementType>> queryPol(MerkleTreeType *tree, uint64_t idx, ElementType* buff);
    static void polMulAxi(Goldilocks::Element *pol, uint64_t degree, Goldilocks::Element acc);
    static void evalPol(Goldilocks::Element* res, uint64_t res_idx, uint64_t degree, Goldilocks::Element* p, Goldilocks::Element *x);
    static void getTransposed(Goldilocks::Element *aux, Goldilocks::Element* pol, uint64_t degree, uint64_t trasposeBits);
};

template class FRI<RawFr::Element>;
template class FRI<Goldilocks::Element>;

#endif