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

    ProverHelpers(StarkInfo &starkInfo, bool pil1) {
        uint64_t nBits = starkInfo.starkStruct.nBits;
        uint64_t nBitsExt = starkInfo.starkStruct.nBitsExt;
        uint64_t qDeg = starkInfo.qDeg;
        vector<Boundary> boundaries = starkInfo.boundaries;

        if(pil1) {
            #pragma omp parallel sections
            {
                #pragma omp section
                computeZerofier(nBits, nBitsExt, boundaries);

                #pragma omp section
                computeX(nBits, nBitsExt, qDeg);

                #pragma omp section
                computeConnectionsX(nBits, nBitsExt);
            }
        } else {
            #pragma omp parallel sections
            {
                #pragma omp section
                computeZerofier(nBits, nBitsExt, boundaries);

                #pragma omp section
                computeX(nBits, nBitsExt, qDeg);
            }
            x_n = new Goldilocks::Element[1 << nBits];
            x_2ns = new Goldilocks::Element[1 << nBitsExt];
        }
    }

    ProverHelpers(StarkInfo& starkInfo, Goldilocks::Element* z) { 
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

    void computeZerofier(uint64_t nBits, uint64_t nBitsExt, vector<Boundary> boundaries) {
        uint64_t N = 1 << nBits;
        uint64_t NExtended = 1 << nBitsExt;
        zi = new Goldilocks::Element[boundaries.size() * NExtended];

        for(uint64_t i = 0; i < boundaries.size(); ++i) {
            Boundary boundary = boundaries[i];
            if(boundary.name == "everyRow") {
                buildZHInv(nBits, nBitsExt);
            } else if(boundary.name == "firstRow") {
                buildOneRowZerofierInv(nBits, nBitsExt, i, 0);
            } else if(boundary.name == "lastRow") {
                buildOneRowZerofierInv(nBits, nBitsExt, i, N);
            } else if(boundary.name == "everyFrame") {
                buildFrameZerofierInv(nBits, nBitsExt, i, boundary.offsetMin, boundary.offsetMax);
            }
        }
    }

    void computeConnectionsX(uint64_t nBits, uint64_t nBitsExt) {
        uint64_t N = 1 << nBits;
        uint64_t NExtended = 1 << nBitsExt;
        x_n = new Goldilocks::Element[N];
        x_2ns = new Goldilocks::Element[NExtended];

        Goldilocks::Element xx = Goldilocks::one();
        for (uint64_t i = 0; i < N; i++)
        {
            x_n[i] = xx;
            xx = xx * Goldilocks::w(nBits);
        }

        Goldilocks::Element xx_shift = Goldilocks::shift();
        for (uint64_t i = 0; i < NExtended; i++)
        {
            x_2ns[i] = xx_shift;
            xx_shift = xx_shift * Goldilocks::w(nBitsExt);
        }
    }

    void computeX(uint64_t nBits, uint64_t nBitsExt, uint64_t qDeg) {
        uint64_t N = 1 << nBits;
        uint64_t extendBits = nBitsExt - nBits;
        x = new Goldilocks::Element[N << extendBits];
        x[0] = Goldilocks::shift();
        for (uint64_t k = 1; k < (N << extendBits); k++)
        {
            x[k] = x[k - 1] * Goldilocks::w(nBitsExt);
        }

        S = new Goldilocks::Element[qDeg];
        Goldilocks::Element shiftIn = Goldilocks::exp(Goldilocks::inv(Goldilocks::shift()), N);
        S[0] = Goldilocks::one();
        for(uint64_t i = 1; i < qDeg; i++) {
            S[i] = Goldilocks::mul(S[i - 1], shiftIn);
        }
    }

    void buildZHInv(uint64_t nBits, uint64_t nBitsExt)
    {
        uint64_t NExtended = 1 << nBitsExt;
        uint64_t extendBits = nBitsExt - nBits;
        uint64_t extend = (1 << extendBits);
        
        Goldilocks::Element w = Goldilocks::one();
        Goldilocks::Element sn = Goldilocks::shift();

        for (uint64_t i = 0; i < nBits; i++) Goldilocks::square(sn, sn);

        for (uint64_t i=0; i<extend; i++) {
            Goldilocks::inv(zi[i], (sn * w) - Goldilocks::one());
            Goldilocks::mul(w, w, Goldilocks::w(extendBits));
        }

        #pragma omp parallel for
        for (uint64_t i=extend; i<NExtended; i++) {
            zi[i] = zi[i % extend];
        }
    };

    void buildOneRowZerofierInv(uint64_t nBits, uint64_t nBitsExt, uint64_t offset, uint64_t rowIndex)
    {
        uint64_t NExtended = 1 << nBitsExt;
        Goldilocks::Element root = Goldilocks::one();

        for(uint64_t i = 0; i < rowIndex; ++i) {
            root = root * Goldilocks::w(nBits);
        }

        Goldilocks::Element w = Goldilocks::one();
        Goldilocks::Element sn = Goldilocks::shift();

        for(uint64_t i = 0; i < NExtended; ++i) {
            Goldilocks::Element x = sn * w;
            Goldilocks::inv(zi[i + offset * NExtended], (x - root) * zi[i]);
            w = w * Goldilocks::w(nBitsExt);
        }
    }

    void buildFrameZerofierInv(uint64_t nBits, uint64_t nBitsExt, uint64_t offset, uint64_t offsetMin, uint64_t offsetMax)
    {
        uint64_t NExtended = 1 << nBitsExt;
        uint64_t N = 1 << nBits;
        uint64_t nRoots = offsetMin + offsetMax;
        Goldilocks::Element roots[nRoots];

        for(uint64_t i = 0; i < offsetMin; ++i) {
            roots[i] = Goldilocks::one();
            for(uint64_t j = 0; j < i; ++j) {
                roots[i] = roots[i] * Goldilocks::w(nBits);
            }
        }

        for(uint64_t i = 0; i < offsetMax; ++i) {
            roots[i + offsetMin] = Goldilocks::one();
            for(uint64_t j = 0; j < (N - i - 1); ++j) {
                roots[i + offsetMin] = roots[i + offsetMin] * Goldilocks::w(nBits);
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
            w = w * Goldilocks::w(nBitsExt);
        }
    }

    ~ProverHelpers() {
        if(zi != nullptr) delete[] zi;
        if(S != nullptr) delete[] S;
        if(x != nullptr) delete[] x;
        if(x_n != nullptr) delete[] x_n;
        if(x_2ns != nullptr) delete[] x_2ns;   
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