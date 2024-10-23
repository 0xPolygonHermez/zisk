#ifndef CONST_POLS_STARKS_HPP
#define CONST_POLS_STARKS_HPP

#include <cstdint>
#include "goldilocks_base_field.hpp"
#include "zkassert.hpp"
#include "stark_info.hpp"
#include "zklog.hpp"
#include "utils.hpp"
#include "timer.hpp"
#include "ntt_goldilocks.hpp"
#include "merkleTreeBN128.hpp"
#include "merkleTreeGL.hpp"


class ConstPols 
{
public:
    Goldilocks::Element *pConstPolsAddress = nullptr;
    Goldilocks::Element *pConstPolsAddressExtended;
    Goldilocks::Element *pConstTreeAddress = nullptr;
    Goldilocks::Element *zi = nullptr;
    Goldilocks::Element *S = nullptr;
    Goldilocks::Element *x = nullptr;
    Goldilocks::Element *x_n = nullptr; // Needed for PIL1 compatibility
    Goldilocks::Element *x_2ns = nullptr; // Needed for PIL1 compatibility

    ConstPols(StarkInfo& starkInfo, std::string constPolsFile, bool calculateTree = true) {

        loadConstPols(starkInfo, constPolsFile);

        if(calculateTree) {
           calculateConstTree(starkInfo);
        }

        computeZerofier(starkInfo);

        computeX(starkInfo);

        computeConnectionsX(starkInfo); // Needed for PIL1 compatibility
    }

    ConstPols(StarkInfo& starkInfo, std::string constPolsFile, std::string constTreeFile) {

        loadConstPols(starkInfo, constPolsFile);
            
        loadConstTree(starkInfo, constTreeFile);

        computeZerofier(starkInfo);

        computeX(starkInfo);

        computeConnectionsX(starkInfo); // Needed for PIL1 compatibility
    }

    // For verification only
    ConstPols(StarkInfo& starkInfo, Goldilocks::Element* z, Goldilocks::Element* constVals) {        
        pConstPolsAddress = (Goldilocks::Element *)malloc(starkInfo.nConstants * starkInfo.starkStruct.nQueries * sizeof(Goldilocks::Element));
        for(uint64_t i = 0; i < starkInfo.nConstants * starkInfo.starkStruct.nQueries; ++i) {
            pConstPolsAddress[i] = constVals[i];
        }


        zi = new Goldilocks::Element[starkInfo.boundaries.size() * FIELD_EXTENSION];

        Goldilocks::Element one[3] = {Goldilocks::one(), Goldilocks::zero(), Goldilocks::zero()};

        Goldilocks::Element xN[3] = {Goldilocks::one(), Goldilocks::zero(), Goldilocks::zero()};
        for(uint64_t i = 0; i < uint64_t(1 << starkInfo.starkStruct.nBits); ++i) {
            Goldilocks3::mul((Goldilocks3::Element *)xN, (Goldilocks3::Element *)xN, (Goldilocks3::Element *)z);
        }

        Goldilocks::Element zN[3] = { xN[0] - Goldilocks::one(), xN[1], xN[2]};
        Goldilocks::Element zNInv[3];
        Goldilocks3::inv((Goldilocks3::Element *)zNInv, (Goldilocks3::Element *)zN);
        std::memcpy(&zi[0], zNInv, FIELD_EXTENSION * sizeof(Goldilocks::Element));

        for(uint64_t i = 1; i < starkInfo.boundaries.size(); ++i) {
            Boundary boundary = starkInfo.boundaries[i];
            if(boundary.name == "firstRow") {
                Goldilocks::Element zi_[3];
                Goldilocks3::sub((Goldilocks3::Element &)zi_[0], (Goldilocks3::Element &)z[0], (Goldilocks3::Element &)one[0]);
                Goldilocks3::inv((Goldilocks3::Element *)zi_, (Goldilocks3::Element *)zi_);
                Goldilocks3::mul((Goldilocks3::Element *)zi_, (Goldilocks3::Element *)zi_, (Goldilocks3::Element *)zN);
                std::memcpy(&zi[i*FIELD_EXTENSION], zi_, FIELD_EXTENSION * sizeof(Goldilocks::Element));
            } else if(boundary.name == "lastRow") {
                Goldilocks::Element root = Goldilocks::one();
                for(uint64_t i = 0; i < uint64_t(1 << starkInfo.starkStruct.nBits) - 1; ++i) {
                    root = root * Goldilocks::w(starkInfo.starkStruct.nBits);
                }
                Goldilocks::Element zi_[3];
                Goldilocks3::sub((Goldilocks3::Element &)zi_[0], (Goldilocks3::Element &)z[0], (Goldilocks3::Element &)root);
                Goldilocks3::inv((Goldilocks3::Element *)zi_, (Goldilocks3::Element *)zi_);
                Goldilocks3::mul((Goldilocks3::Element *)zi_, (Goldilocks3::Element *)zi_, (Goldilocks3::Element *)zN);
                std::memcpy(&zi[i*FIELD_EXTENSION], zi_, FIELD_EXTENSION * sizeof(Goldilocks::Element));
            } else if(boundary.name == "everyRow") {
                uint64_t nRoots = boundary.offsetMin + boundary.offsetMax;
                Goldilocks::Element roots[nRoots];
                Goldilocks::Element zi_[3] = { Goldilocks::one(), Goldilocks::zero(), Goldilocks::zero()};
                for(uint64_t i = 0; i < boundary.offsetMin; ++i) {
                    roots[i] = Goldilocks::one();
                    for(uint64_t j = 0; j < i; ++j) {
                        roots[i] = roots[i] * Goldilocks::w(starkInfo.starkStruct.nBits);
                    }
                    Goldilocks::Element aux[3];
                    Goldilocks3::sub((Goldilocks3::Element &)aux[0], (Goldilocks3::Element &)z[0], (Goldilocks3::Element &)roots[i]);
                    Goldilocks3::mul((Goldilocks3::Element *)zi_, (Goldilocks3::Element *)zi_, (Goldilocks3::Element *)aux);
                }

                for(uint64_t i = 0; i < boundary.offsetMax; ++i) {
                    roots[i + boundary.offsetMin] = Goldilocks::one();
                    for(uint64_t j = 0; j < (uint64_t(1 << starkInfo.starkStruct.nBits) - i - 1); ++j) {
                        roots[i + boundary.offsetMin] = roots[i + boundary.offsetMin] * Goldilocks::w(starkInfo.starkStruct.nBits);
                    }
                    Goldilocks::Element aux[3];
                    Goldilocks3::sub((Goldilocks3::Element &)aux[0], (Goldilocks3::Element &)z[0], (Goldilocks3::Element &)roots[i + boundary.offsetMin]);
                    Goldilocks3::mul((Goldilocks3::Element *)zi_, (Goldilocks3::Element *)zi_, (Goldilocks3::Element *)aux);
                }

                std::memcpy(&zi[i*FIELD_EXTENSION], zi_, FIELD_EXTENSION * sizeof(Goldilocks::Element));
            }
        }

        x_n = new Goldilocks::Element[FIELD_EXTENSION];
        x_n[0] = z[0];
        x_n[1] = z[1];
        x_n[2] = z[2];
    };

    void calculateConstTree(StarkInfo& starkInfo) {
        pConstTreeAddress = (Goldilocks::Element *)malloc(getConstTreeSize(starkInfo));
        if(pConstTreeAddress == NULL)
        {
            zklog.error("Starks::Starks() failed to allocate pConstTreeAddress");
            exitProcess();
        }
        pConstPolsAddressExtended = &pConstTreeAddress[2];
    
        uint64_t merkleTreeArity = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? starkInfo.starkStruct.merkleTreeArity : 2;
        uint64_t merkleTreeCustom = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? starkInfo.starkStruct.merkleTreeCustom : true;
        uint64_t N = 1 << starkInfo.starkStruct.nBits;
        uint64_t NExtended = 1 << starkInfo.starkStruct.nBitsExt;
        NTT_Goldilocks ntt(N);
        ntt.extendPol((Goldilocks::Element *)pConstPolsAddressExtended, (Goldilocks::Element *)pConstPolsAddress, NExtended, N, starkInfo.nConstants);
        MerkleTreeGL mt(merkleTreeArity, merkleTreeCustom, NExtended, starkInfo.nConstants, (Goldilocks::Element *)pConstPolsAddressExtended);
        mt.merkelize();

        pConstTreeAddress[0] = Goldilocks::fromU64(starkInfo.nConstants);  
        pConstTreeAddress[1] = Goldilocks::fromU64(NExtended);
        memcpy(&pConstTreeAddress[2 + starkInfo.nConstants * NExtended], mt.nodes, mt.numNodes * sizeof(Goldilocks::Element));
    }

    void loadConstTree(StarkInfo& starkInfo, std::string constTreeFile) {
        uint64_t constTreeSizeBytes = getConstTreeSize(starkInfo);

        pConstTreeAddress = (Goldilocks::Element *)loadFileParallel(constTreeFile, constTreeSizeBytes);
        
        pConstPolsAddressExtended = &pConstTreeAddress[2];
    }

    void loadConstPols(StarkInfo& starkInfo, std::string constPolsFile) {
        // Allocate an area of memory, mapped to file, to read all the constant polynomials,
        // and create them using the allocated address

        uint64_t N = 1 << starkInfo.starkStruct.nBits;
        uint64_t constPolsSize = starkInfo.nConstants * sizeof(Goldilocks::Element) * N;
        
        pConstPolsAddress = (Goldilocks::Element *)loadFileParallel(constPolsFile, constPolsSize);
    }

    uint64_t getConstTreeSize(StarkInfo& starkInfo)
    {   
        uint64_t merkleTreeArity = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? starkInfo.starkStruct.merkleTreeArity : 2;
        uint64_t NExtended = 1 << starkInfo.starkStruct.nBitsExt;
        uint n_tmp = NExtended;
        uint64_t nextN = floor(((double)(n_tmp - 1) / merkleTreeArity) + 1);
        uint64_t acc = nextN * merkleTreeArity;
        while (n_tmp > 1)
        {
            // FIll with zeros if n nodes in the leve is not even
            n_tmp = nextN;
            nextN = floor((n_tmp - 1) / merkleTreeArity) + 1;
            if (n_tmp > 1)
            {
                acc += nextN * merkleTreeArity;
            }
            else
            {
                acc += 1;
            }
        }

        uint64_t elementSize = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? sizeof(RawFr::Element) : sizeof(Goldilocks::Element);
        uint64_t numElements = NExtended * starkInfo.nConstants * sizeof(Goldilocks::Element);
        uint64_t nFieldElements = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? 1 : HASH_SIZE;
        uint64_t total = numElements + acc * nFieldElements * elementSize;
        if(starkInfo.starkStruct.verificationHashType == std::string("BN128")) {
            total += 16; // HEADER
        } else {
            total += merkleTreeArity * elementSize;
        }
        return total; 
        
    };

    void computeZerofier(StarkInfo& starkInfo) {
        uint64_t N = 1 << starkInfo.starkStruct.nBits;
        uint64_t NExtended = 1 << starkInfo.starkStruct.nBitsExt;
        zi = new Goldilocks::Element[starkInfo.boundaries.size() * NExtended];

        for(uint64_t i = 0; i < starkInfo.boundaries.size(); ++i) {
            Boundary boundary = starkInfo.boundaries[i];
            if(boundary.name == "everyRow") {
                buildZHInv(starkInfo);
            } else if(boundary.name == "firstRow") {
                buildOneRowZerofierInv(starkInfo, i, 0);
            } else if(boundary.name == "lastRow") {
                buildOneRowZerofierInv(starkInfo, i, N);
            } else if(boundary.name == "everyRow") {
                buildFrameZerofierInv(starkInfo, i, boundary.offsetMin, boundary.offsetMax);
            }
        }
    }

    void computeConnectionsX(StarkInfo& starkInfo) {
        uint64_t N = 1 << starkInfo.starkStruct.nBits;
        uint64_t NExtended = 1 << starkInfo.starkStruct.nBitsExt;
        x_n = new Goldilocks::Element[N];
        Goldilocks::Element xx = Goldilocks::one();
        for (uint64_t i = 0; i < N; i++)
        {
            x_n[i] = xx;
            Goldilocks::mul(xx, xx, Goldilocks::w(starkInfo.starkStruct.nBits));
        }
        xx = Goldilocks::shift();
        x_2ns = new Goldilocks::Element[NExtended];
        for (uint64_t i = 0; i < NExtended; i++)
        {
            x_2ns[i] = xx;
            Goldilocks::mul(xx, xx, Goldilocks::w(starkInfo.starkStruct.nBitsExt));
        }
    }

    void computeX(StarkInfo& starkInfo) {
        uint64_t N = 1 << starkInfo.starkStruct.nBits;
        uint64_t extendBits = starkInfo.starkStruct.nBitsExt - starkInfo.starkStruct.nBits;
        x = new Goldilocks::Element[N << extendBits];
        x[0] = Goldilocks::shift();
        for (uint64_t k = 1; k < (N << extendBits); k++)
        {
            x[k] = x[k - 1] * Goldilocks::w(starkInfo.starkStruct.nBits + extendBits);
        }

        S = new Goldilocks::Element[starkInfo.qDeg];
        Goldilocks::Element shiftIn = Goldilocks::exp(Goldilocks::inv(Goldilocks::shift()), N);
        S[0] = Goldilocks::one();
        for(uint64_t i = 1; i < starkInfo.qDeg; i++) {
            S[i] = Goldilocks::mul(S[i - 1], shiftIn);
        }
    }

    void buildZHInv(StarkInfo& starkInfo)
    {
        uint64_t NExtended = 1 << starkInfo.starkStruct.nBitsExt;
        uint64_t extendBits = starkInfo.starkStruct.nBitsExt - starkInfo.starkStruct.nBits;
        uint64_t extend = (1 << extendBits);
        
        Goldilocks::Element w = Goldilocks::one();
        Goldilocks::Element sn = Goldilocks::shift();

        for (uint64_t i = 0; i < starkInfo.starkStruct.nBits; i++) Goldilocks::square(sn, sn);

        for (uint64_t i=0; i<extend; i++) {
            Goldilocks::inv(zi[i], (sn * w) - Goldilocks::one());
            Goldilocks::mul(w, w, Goldilocks::w(extendBits));
        }

        #pragma omp parallel for
        for (uint64_t i=extend; i<NExtended; i++) {
            zi[i] = zi[i % extend];
        }
    };

    void buildOneRowZerofierInv(StarkInfo& starkInfo, uint64_t offset, uint64_t rowIndex)
    {
        uint64_t NExtended = 1 << starkInfo.starkStruct.nBitsExt;
        Goldilocks::Element root = Goldilocks::one();

        for(uint64_t i = 0; i < rowIndex; ++i) {
            root = root * Goldilocks::w(starkInfo.starkStruct.nBits);
        }

        Goldilocks::Element w = Goldilocks::one();
        Goldilocks::Element sn = Goldilocks::shift();

        for(uint64_t i = 0; i < NExtended; ++i) {
            Goldilocks::Element x = sn * w;
            Goldilocks::inv(zi[i + offset * NExtended], (x - root) * zi[i]);
            w = w * Goldilocks::w(starkInfo.starkStruct.nBitsExt);
        }
    }

    void buildFrameZerofierInv(StarkInfo& starkInfo, uint64_t offset, uint64_t offsetMin, uint64_t offsetMax)
    {
        uint64_t NExtended = 1 << starkInfo.starkStruct.nBitsExt;
        uint64_t N = 1 << starkInfo.starkStruct.nBits;
        uint64_t nRoots = offsetMin + offsetMax;
        Goldilocks::Element roots[nRoots];

        for(uint64_t i = 0; i < offsetMin; ++i) {
            roots[i] = Goldilocks::one();
            for(uint64_t j = 0; j < i; ++j) {
                roots[i] = roots[i] * Goldilocks::w(starkInfo.starkStruct.nBits);
            }
        }

        for(uint64_t i = 0; i < offsetMax; ++i) {
            roots[i + offsetMin] = Goldilocks::one();
            for(uint64_t j = 0; j < (N - i - 1); ++j) {
                roots[i + offsetMin] = roots[i + offsetMin] * Goldilocks::w(starkInfo.starkStruct.nBits);
            }
        }

        Goldilocks::Element w = Goldilocks::one();
        Goldilocks::Element sn = Goldilocks::shift();

        for(uint64_t i = 0; i < NExtended; ++i) {
            zi[i + offset*NExtended] = Goldilocks::one();
            Goldilocks::Element x = sn * w;
            for(uint64_t j = 0; j < nRoots; ++j) {
                zi[i + offset*NExtended] = zi[i + offset*NExtended] * (x - roots[j]);
            }
            w = w * Goldilocks::w(starkInfo.starkStruct.nBitsExt);
        }
    }

    ~ConstPols()
    {   
        if(pConstPolsAddress != nullptr) free(pConstPolsAddress);
        if(pConstTreeAddress != nullptr) free(pConstTreeAddress);
        if(zi != nullptr) delete zi;
        if(S != nullptr) delete S;
        if(x != nullptr) delete x;
        if(x_n != nullptr) delete x_n;
        if(x_2ns != nullptr) delete x_2ns;        
    }
};

#endif // CONST_POLS_STARKS_HPP