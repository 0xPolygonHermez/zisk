#ifndef SETUP_CTX_HPP
#define SETUP_CTX_HPP

#include "stark_info.hpp"
#include "const_pols.hpp"
#include "expressions_bin.hpp"

class ProverHelpers {
    public: 
    Goldilocks::Element *zi = nullptr;
    Goldilocks::Element *S = nullptr;
    Goldilocks::Element *x = nullptr;
    Goldilocks::Element *x_n = nullptr; // Needed for PIL1 compatibility
    Goldilocks::Element *x_2ns = nullptr; // Needed for PIL1 compatibility

    ProverHelpers(StarkInfo &starkInfo) {
        computeZerofier(starkInfo);

        computeX(starkInfo);

        computeConnectionsX(starkInfo); // Needed for PIL1 compatibility
    }

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

    ~ProverHelpers() {
        if(zi != nullptr) delete zi;
        if(S != nullptr) delete S;
        if(x != nullptr) delete x;
        if(x_n != nullptr) delete x_n;
        if(x_2ns != nullptr) delete x_2ns;   
    };
};

class SetupCtx {
public:

    StarkInfo &starkInfo;
    ExpressionsBin &expressionsBin;
    ProverHelpers &proverHelpers; 
    
    SetupCtx(StarkInfo &_starkInfo, ExpressionsBin& _expressionsBin, ProverHelpers& _proverHelpers) : starkInfo(_starkInfo), expressionsBin(_expressionsBin), proverHelpers(_proverHelpers)  {};
};

#endif