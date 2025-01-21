

#include "fri.hpp"
#include "timer.hpp"
#include "zklog.hpp"

template <typename ElementType>
void FRI<ElementType>::fold(uint64_t step, Goldilocks::Element* pol, Goldilocks::Element *challenge, uint64_t nBitsExt, uint64_t prevBits, uint64_t currentBits) {

    uint64_t polBits = step == 0 ? nBitsExt : prevBits;

    Goldilocks::Element polShiftInv = Goldilocks::inv(Goldilocks::shift());
    
    if(step > 0) {
        for (uint64_t j = 0; j < nBitsExt - prevBits; j++)
        {
            polShiftInv = polShiftInv * polShiftInv;
        }
    }

    uint64_t pol2N = 1 << currentBits;
    uint64_t nX = (1 << polBits) / pol2N;

    Goldilocks::Element wi = Goldilocks::inv(Goldilocks::w(polBits));

    uint64_t nn = ((1 << polBits) / nX);
    u_int64_t maxth = omp_get_max_threads();
    if (maxth > nn) maxth = nn;
#pragma omp parallel num_threads(maxth)
    {
        u_int64_t nth = omp_get_num_threads();
        u_int64_t thid = omp_get_thread_num();
        u_int64_t chunk = nn / nth;
        u_int64_t res = nn - nth * chunk;

        // Evaluate bounds of the loop for the thread
        uint64_t init = chunk * thid;
        uint64_t end;
        if (thid < res) {
            init += thid;
            end = init + chunk + 1;
        } else {
            init += res;
            end = init + chunk;
        }
        //  Evaluate the starting point for the sinv
        Goldilocks::Element aux = wi;
        Goldilocks::Element sinv_ = polShiftInv;
        for (uint64_t i = 0; i < chunk - 1; ++i) aux = aux * wi;
        for (u_int64_t i = 0; i < thid; ++i) sinv_ = sinv_ * aux;   
        u_int64_t ncor = res;
        if (thid < res) ncor = thid;
        for (u_int64_t j = 0; j < ncor; ++j) sinv_ = sinv_ * wi;
        for (uint64_t g = init; g < end; g++)
        {
            if (step != 0)
            {
                Goldilocks::Element ppar[nX * FIELD_EXTENSION];
                Goldilocks::Element ppar_c[nX * FIELD_EXTENSION];

                #pragma omp parallel for
                for (uint64_t i = 0; i < nX; i++)
                {
                    std::memcpy(&ppar[i * FIELD_EXTENSION], &pol[((i * pol2N) + g) * FIELD_EXTENSION], FIELD_EXTENSION * sizeof(Goldilocks::Element));
                }
                NTT_Goldilocks ntt(nX, 1);

                ntt.INTT(ppar_c, ppar, nX, FIELD_EXTENSION);
                polMulAxi(ppar_c, nX, sinv_); // Multiplies coefs by 1, shiftInv, shiftInv^2, shiftInv^3, ......
                evalPol(pol, g, nX, ppar_c, challenge);
                sinv_ = sinv_ * wi;
            }
        }
    }
}

template <typename ElementType>
void FRI<ElementType>::merkelize(uint64_t step, FRIProof<ElementType> &proof, Goldilocks::Element* pol, MerkleTreeType* treeFRI, uint64_t currentBits, uint64_t nextBits) {
    // Re-org in groups
    getTransposed(treeFRI->source, pol, 1 << currentBits, nextBits);

    treeFRI->merkelize();
    treeFRI->getRoot(&proof.proof.fri.treesFRI[step].root[0]);
}

template <typename ElementType>
void FRI<ElementType>::proveQueries(uint64_t* friQueries, uint64_t nQueries, FRIProof<ElementType> &fproof, MerkleTreeType **trees, uint64_t nTrees) {
    uint64_t maxBuffSize = 0;
    for(uint64_t i = 0; i < nTrees; ++i) {
        uint64_t buffSize = trees[i]->getMerkleTreeWidth() + trees[i]->getMerkleProofSize();
        if(buffSize > maxBuffSize) {
            maxBuffSize = buffSize;
        }
    }

    ElementType *buff = new ElementType[maxBuffSize];
    for (uint64_t i = 0; i < nQueries; i++) {
        fproof.proof.fri.trees.polQueries[i] = queryPol(trees, nTrees, friQueries[i], buff);
    }

    delete[] buff;

    return;
}

template <typename ElementType>
void FRI<ElementType>::proveFRIQueries(uint64_t* friQueries, uint64_t nQueries, uint64_t step, uint64_t currentBits, FRIProof<ElementType> &fproof, MerkleTreeType *treeFRI) {
    ElementType *buff = new ElementType[treeFRI->getMerkleTreeWidth() + treeFRI->getMerkleProofSize()];
    for (uint64_t i = 0; i < nQueries; i++) {
        fproof.proof.fri.treesFRI[step - 1].polQueries[i] = queryPol(treeFRI,  friQueries[i] % (1 << currentBits), buff);
    }

    delete[] buff;
    return;
}

template <typename ElementType>
void FRI<ElementType>::setFinalPol(FRIProof<ElementType> &fproof, Goldilocks::Element* buffer, uint64_t nBits) {
    fproof.proof.fri.setPol(buffer, (1 << nBits));
    return;
}

template <typename ElementType>
vector<MerkleProof<ElementType>> FRI<ElementType>::queryPol(MerkleTreeType *trees[], uint64_t nTrees, uint64_t idx, ElementType* buff)
{
    vector<MerkleProof<ElementType>> vMkProof;
    for (uint i = 0; i < nTrees; i++)
    {
        trees[i]->getGroupProof(&buff[0], idx);

        MerkleProof<ElementType> mkProof(trees[i]->getMerkleTreeWidth(), trees[i]->getMerkleProofLength(), trees[i]->getNumSiblings(), &buff[0]);
        vMkProof.push_back(mkProof);
    }
    return vMkProof;
}


template <typename ElementType>
vector<MerkleProof<ElementType>> FRI<ElementType>::queryPol(MerkleTreeType *tree, uint64_t idx, ElementType *buff)
{
    vector<MerkleProof<ElementType>> vMkProof;

    tree->getGroupProof(&buff[0], idx);

    MerkleProof<ElementType> mkProof(tree->getMerkleTreeWidth(), tree->getMerkleProofLength(), tree->getNumSiblings(), &buff[0]);
    vMkProof.push_back(mkProof);

    return vMkProof;
}

template <typename ElementType>
void FRI<ElementType>::polMulAxi(Goldilocks::Element *pol, uint64_t degree, Goldilocks::Element acc)
{
    Goldilocks::Element r = Goldilocks::one();
    for (uint64_t i = 0; i < degree; i++)
    {   
        Goldilocks3::mul((Goldilocks3::Element &)(pol[i * FIELD_EXTENSION]), (Goldilocks3::Element &)(pol[i * FIELD_EXTENSION]), r);
        r = r * acc;
    }
}

template <typename ElementType>
void FRI<ElementType>::evalPol(Goldilocks::Element* res, uint64_t res_idx, uint64_t degree, Goldilocks::Element* p, Goldilocks::Element *x)
{
    if (degree == 0)
    {
        res[res_idx * FIELD_EXTENSION] = Goldilocks::zero();
        res[res_idx * FIELD_EXTENSION + 1] = Goldilocks::zero();
        res[res_idx * FIELD_EXTENSION + 2] = Goldilocks::zero();
        return;
    }

    std::memcpy(&res[res_idx * FIELD_EXTENSION], &p[(degree - 1) * FIELD_EXTENSION], FIELD_EXTENSION * sizeof(Goldilocks::Element));
    for (int64_t i = degree - 2; i >= 0; i--)
    {
        Goldilocks3::Element aux;
        Goldilocks3::mul(aux, (Goldilocks3::Element &)(res[res_idx * FIELD_EXTENSION]), (Goldilocks3::Element &)x[0]);
        Goldilocks3::add((Goldilocks3::Element &)(res[res_idx * FIELD_EXTENSION]), aux, (Goldilocks3::Element &)p[i * FIELD_EXTENSION]);
    }
}

template <typename ElementType>
void FRI<ElementType>::getTransposed(Goldilocks::Element *aux, Goldilocks::Element* pol, uint64_t degree, uint64_t trasposeBits)
{
    uint64_t w = (1 << trasposeBits);
    uint64_t h = degree / w;

#pragma omp parallel for
    for (uint64_t i = 0; i < w; i++)
    {
        for (uint64_t j = 0; j < h; j++)
        {

            uint64_t fi = j * w + i;
            uint64_t di = i * h + j;

            std::memcpy(&aux[di * FIELD_EXTENSION], &pol[fi * FIELD_EXTENSION], FIELD_EXTENSION * sizeof(Goldilocks::Element));
        }
    }
}


template <typename ElementType>
void FRI<ElementType>::verify_fold(Goldilocks::Element* value, uint64_t step, uint64_t nBitsExt, uint64_t currentBits, uint64_t prevBits, Goldilocks::Element *challenge, uint64_t idx, std::vector<Goldilocks::Element> &v) {
    Goldilocks::Element shift = Goldilocks::shift();
    
    for (uint64_t j = 0; j < nBitsExt - prevBits; j++) {
        shift = shift * shift;
    }
    
    uint64_t nX = (1 << prevBits) / (1 << currentBits);

    Goldilocks::Element ppar_c[nX * FIELD_EXTENSION];
    
    uint64_t c = 0;
    for(uint64_t i = 0; i < v.size(); ++i) {
        ppar_c[c++] = v[i];
    }

    assert(c == nX * FIELD_EXTENSION);

    NTT_Goldilocks ntt(nX, 1);

    ntt.INTT(ppar_c, ppar_c, nX, FIELD_EXTENSION);

    Goldilocks::Element sinv = Goldilocks::inv(Goldilocks::mul(shift, Goldilocks::exp(Goldilocks::w(prevBits), idx)));
    
    Goldilocks::Element aux[3];
    Goldilocks3::mul((Goldilocks3::Element &)aux[0], (Goldilocks3::Element &)challenge[0], sinv);
    evalPol(value, 0, nX, ppar_c, aux);
}