#include "definitions.hpp"
#include "starks.hpp"
#include "zklog.hpp"
#include "exit_process.hpp"

template <typename ElementType>
void Starks<ElementType>::extendAndMerkelize(uint64_t step, Goldilocks::Element *buffer, FRIProof<ElementType> &proof, Goldilocks::Element *pBuffHelper)
{   
    uint64_t N = 1 << setupCtx.starkInfo.starkStruct.nBits;
    uint64_t NExtended = 1 << setupCtx.starkInfo.starkStruct.nBitsExt;

    std::string section = "cm" + to_string(step);  
    uint64_t nCols = setupCtx.starkInfo.mapSectionsN["cm" + to_string(step)];
    
    Goldilocks::Element *pBuff = &buffer[setupCtx.starkInfo.mapOffsets[make_pair(section, false)]];
    Goldilocks::Element *pBuffExtended = &buffer[setupCtx.starkInfo.mapOffsets[make_pair(section, true)]];

    NTT_Goldilocks ntt(N);
    if(pBuffHelper != nullptr) {
        ntt.extendPol(pBuffExtended, pBuff, NExtended, N, nCols, pBuffHelper);
    } else {
        ntt.extendPol(pBuffExtended, pBuff, NExtended, N, nCols);
    }
    
    treesGL[step - 1]->setSource(pBuffExtended);
    treesGL[step - 1]->merkelize();
    treesGL[step - 1]->getRoot(&proof.proof.roots[step - 1][0]);
}

template <typename ElementType>
void Starks<ElementType>::commitStage(uint64_t step, Goldilocks::Element *buffer, FRIProof<ElementType> &proof, Goldilocks::Element* pBuffHelper)
{  

    if (step <= setupCtx.starkInfo.nStages)
    {
        extendAndMerkelize(step, buffer, proof, pBuffHelper);
    }
    else
    {
        computeQ(step, buffer, proof, pBuffHelper);
    }
}

template <typename ElementType>
void Starks<ElementType>::computeQ(uint64_t step, Goldilocks::Element *buffer, FRIProof<ElementType> &proof, Goldilocks::Element* pBuffHelper)
{
    uint64_t N = 1 << setupCtx.starkInfo.starkStruct.nBits;
    uint64_t NExtended = 1 << setupCtx.starkInfo.starkStruct.nBitsExt;

    std::string section = "cm" + to_string(setupCtx.starkInfo.nStages + 1);
    uint64_t nCols = setupCtx.starkInfo.mapSectionsN["cm" + to_string(setupCtx.starkInfo.nStages + 1)];
    Goldilocks::Element *cmQ = &buffer[setupCtx.starkInfo.mapOffsets[make_pair(section, true)]];


    NTT_Goldilocks nttExtended(NExtended);

    if(pBuffHelper != nullptr) {
        nttExtended.INTT(&buffer[setupCtx.starkInfo.mapOffsets[std::make_pair("q", true)]], &buffer[setupCtx.starkInfo.mapOffsets[std::make_pair("q", true)]], NExtended, setupCtx.starkInfo.qDim, pBuffHelper);
    } else {
        nttExtended.INTT(&buffer[setupCtx.starkInfo.mapOffsets[std::make_pair("q", true)]], &buffer[setupCtx.starkInfo.mapOffsets[std::make_pair("q", true)]], NExtended, setupCtx.starkInfo.qDim);
    }

    for (uint64_t p = 0; p < setupCtx.starkInfo.qDeg; p++)
    {   
        #pragma omp parallel for
        for(uint64_t i = 0; i < N; i++)
        { 
            Goldilocks3::mul((Goldilocks3::Element &)cmQ[(i * setupCtx.starkInfo.qDeg + p) * FIELD_EXTENSION], (Goldilocks3::Element &)buffer[setupCtx.starkInfo.mapOffsets[std::make_pair("q", true)] + (p * N + i) * FIELD_EXTENSION], setupCtx.constPols.S[p]);
        }
    }

    memset(&cmQ[N * setupCtx.starkInfo.qDeg * setupCtx.starkInfo.qDim], 0, (NExtended - N) * setupCtx.starkInfo.qDeg * setupCtx.starkInfo.qDim * sizeof(Goldilocks::Element));

    if(pBuffHelper != nullptr) {
        nttExtended.NTT(cmQ, cmQ, NExtended, nCols, pBuffHelper);
    } else {
        nttExtended.NTT(cmQ, cmQ, NExtended, nCols);
    }

    treesGL[step - 1]->setSource(&buffer[setupCtx.starkInfo.mapOffsets[std::make_pair("cm" + to_string(step), true)]]);
    treesGL[step - 1]->merkelize();
    treesGL[step - 1]->getRoot(&proof.proof.roots[step - 1][0]);
    
}

template <typename ElementType>
void Starks<ElementType>::computeLEv(Goldilocks::Element *xiChallenge, Goldilocks::Element *LEv) {
    uint64_t N = 1 << setupCtx.starkInfo.starkStruct.nBits;
        
    Goldilocks::Element xis[setupCtx.starkInfo.openingPoints.size() * FIELD_EXTENSION];
    Goldilocks::Element xisShifted[setupCtx.starkInfo.openingPoints.size() * FIELD_EXTENSION];

    Goldilocks::Element shift_inv = Goldilocks::inv(Goldilocks::shift());
    for (uint64_t i = 0; i < setupCtx.starkInfo.openingPoints.size(); ++i)
    {
        Goldilocks::Element w = Goldilocks::one();
        uint64_t openingAbs = setupCtx.starkInfo.openingPoints[i] < 0 ? -setupCtx.starkInfo.openingPoints[i] : setupCtx.starkInfo.openingPoints[i];
        for (uint64_t j = 0; j < openingAbs; ++j)
        {
            w = w * Goldilocks::w(setupCtx.starkInfo.starkStruct.nBits);
        }

        if (setupCtx.starkInfo.openingPoints[i] < 0)
        {
            w = Goldilocks::inv(w);
        }

        Goldilocks3::mul((Goldilocks3::Element &)(xis[i * FIELD_EXTENSION]), (Goldilocks3::Element &)xiChallenge[0], w);
        Goldilocks3::mul((Goldilocks3::Element &)(xisShifted[i * FIELD_EXTENSION]), (Goldilocks3::Element &)(xis[i * FIELD_EXTENSION]), shift_inv);

        Goldilocks3::one((Goldilocks3::Element &)LEv[i * FIELD_EXTENSION]);
    }


#pragma omp parallel for
    for (uint64_t i = 0; i < setupCtx.starkInfo.openingPoints.size(); ++i)
    {
        for (uint64_t k = 1; k < N; k++)
        {
            Goldilocks3::mul((Goldilocks3::Element &)(LEv[(k*setupCtx.starkInfo.openingPoints.size() + i)*FIELD_EXTENSION]), (Goldilocks3::Element &)(LEv[((k-1)*setupCtx.starkInfo.openingPoints.size() + i)*FIELD_EXTENSION]), (Goldilocks3::Element &)(xisShifted[i * FIELD_EXTENSION]));
        }
    }
    
    NTT_Goldilocks ntt(N);

    ntt.INTT(&LEv[0], &LEv[0], N, FIELD_EXTENSION * setupCtx.starkInfo.openingPoints.size());
}


template <typename ElementType>
void Starks<ElementType>::computeEvals(Goldilocks::Element *buffer, Goldilocks::Element *LEv, Goldilocks::Element *evals, FRIProof<ElementType> &proof)
{
    evmap(buffer, evals, LEv);
    proof.proof.setEvals(evals);
}

template <typename ElementType>
void Starks<ElementType>::calculateXDivXSub(Goldilocks::Element *xiChallenge, Goldilocks::Element *xDivXSub)
{
    uint64_t NExtended = 1 << setupCtx.starkInfo.starkStruct.nBitsExt;

    Goldilocks::Element xis[setupCtx.starkInfo.openingPoints.size() * FIELD_EXTENSION];
    for (uint64_t i = 0; i < setupCtx.starkInfo.openingPoints.size(); ++i)
    {
        Goldilocks::Element w = Goldilocks::one();
        uint64_t openingAbs = setupCtx.starkInfo.openingPoints[i] < 0 ? -setupCtx.starkInfo.openingPoints[i] : setupCtx.starkInfo.openingPoints[i];
        for (uint64_t j = 0; j < openingAbs; ++j)
        {
            w = w * Goldilocks::w(setupCtx.starkInfo.starkStruct.nBits);
        }

        if (setupCtx.starkInfo.openingPoints[i] < 0)
        {
            w = Goldilocks::inv(w);
        }

        Goldilocks3::mul((Goldilocks3::Element &)(xis[i * FIELD_EXTENSION]), (Goldilocks3::Element &)xiChallenge[0], w);
    }

    for (uint64_t i = 0; i < setupCtx.starkInfo.openingPoints.size(); ++i)
    {
#pragma omp parallel for
        for (uint64_t k = 0; k < NExtended; k++)
        {
            Goldilocks3::sub((Goldilocks3::Element &)(xDivXSub[(k + i * NExtended) * FIELD_EXTENSION]), setupCtx.constPols.x[k], (Goldilocks3::Element &)(xis[i * FIELD_EXTENSION]));
        }
    }

    Polinomial xDivXSubXi_(xDivXSub, NExtended * setupCtx.starkInfo.openingPoints.size(), FIELD_EXTENSION, FIELD_EXTENSION);
    Polinomial::batchInverseParallel(xDivXSubXi_, xDivXSubXi_);

    for (uint64_t i = 0; i < setupCtx.starkInfo.openingPoints.size(); ++i)
    {
#pragma omp parallel for
        for (uint64_t k = 0; k < NExtended; k++)
        {
            Goldilocks3::mul((Goldilocks3::Element &)(xDivXSub[(k + i * NExtended) * FIELD_EXTENSION]), (Goldilocks3::Element &)(xDivXSub[(k + i * NExtended) * FIELD_EXTENSION]), setupCtx.constPols.x[k]);
        }
    }
}

template <typename ElementType>
void Starks<ElementType>::evmap(Goldilocks::Element *buffer, Goldilocks::Element *evals, Goldilocks::Element *LEv)
{
    uint64_t extendBits = setupCtx.starkInfo.starkStruct.nBitsExt - setupCtx.starkInfo.starkStruct.nBits;
    u_int64_t size_eval = setupCtx.starkInfo.evMap.size();

    uint64_t N = 1 << setupCtx.starkInfo.starkStruct.nBits;

    int num_threads = omp_get_max_threads();
    int size_thread = size_eval * FIELD_EXTENSION;
    Goldilocks::Element *evals_acc = &buffer[setupCtx.starkInfo.mapOffsets[std::make_pair("evals", true)]];
    memset(&evals_acc[0], 0, num_threads * size_thread * sizeof(Goldilocks::Element));
    
    Polinomial *ordPols = new Polinomial[size_eval];

    for (uint64_t i = 0; i < size_eval; i++)
    {
        EvMap ev = setupCtx.starkInfo.evMap[i];
        bool committed = ev.type == EvMap::eType::cm ? true : false;
        Goldilocks::Element *pols = committed ? buffer : setupCtx.constPols.pConstPolsAddressExtended;
        setupCtx.starkInfo.getPolynomial(ordPols[i], pols, committed, ev.id, true);
    }

#pragma omp parallel
    {
        int thread_idx = omp_get_thread_num();
        Goldilocks::Element *evals_acc_thread = &evals_acc[thread_idx * size_thread];
#pragma omp for
        for (uint64_t k = 0; k < N; k++)
        {
            Goldilocks3::Element LEv_[setupCtx.starkInfo.openingPoints.size()];
            for(uint64_t o = 0; o < setupCtx.starkInfo.openingPoints.size(); o++) {
                uint64_t pos = (o + k*setupCtx.starkInfo.openingPoints.size()) * FIELD_EXTENSION;
                LEv_[o][0] = LEv[pos];
                LEv_[o][1] = LEv[pos + 1];
                LEv_[o][2] = LEv[pos + 2];
            }
            uint64_t row = (k << extendBits);
            for (uint64_t i = 0; i < size_eval; i++)
            {
                EvMap ev = setupCtx.starkInfo.evMap[i];
                Goldilocks3::Element res;
                if (ordPols[i].dim() == 1) {
                    Goldilocks3::mul(res, LEv_[ev.openingPos], *ordPols[i][row]);
                } else {
                    Goldilocks3::mul(res, LEv_[ev.openingPos], (Goldilocks3::Element &)(*ordPols[i][row]));
                }
                Goldilocks3::add((Goldilocks3::Element &)(evals_acc_thread[i * FIELD_EXTENSION]), (Goldilocks3::Element &)(evals_acc_thread[i * FIELD_EXTENSION]), res);
            }
        }
#pragma omp for
        for (uint64_t i = 0; i < size_eval; ++i)
        {
            Goldilocks3::Element sum = { Goldilocks::zero(), Goldilocks::zero(), Goldilocks::zero() };
            for (int k = 0; k < num_threads; ++k)
            {
                Goldilocks3::add(sum, sum, (Goldilocks3::Element &)(evals_acc[k * size_thread + i * FIELD_EXTENSION]));
            }
            std::memcpy((Goldilocks3::Element &)(evals[i * FIELD_EXTENSION]), sum, FIELD_EXTENSION * sizeof(Goldilocks::Element));
        }
    }
    delete[] ordPols;
}

template <typename ElementType>
void Starks<ElementType>::setConstTree(ConstPols &constPols)
{
    setupCtx.constPols = constPols;
    treesGL[setupCtx.starkInfo.nStages + 1] = new MerkleTreeType(setupCtx.starkInfo.starkStruct.merkleTreeArity, setupCtx.starkInfo.starkStruct.merkleTreeCustom, (Goldilocks::Element *)setupCtx.constPols.pConstTreeAddress);
}

template <typename ElementType>
void Starks<ElementType>::getChallenge(TranscriptType &transcript, Goldilocks::Element &challenge)
{
    transcript.getField((uint64_t *)&challenge);
}

template <typename ElementType>
void Starks<ElementType>::calculateHash(ElementType* hash, Goldilocks::Element* buffer, uint64_t nElements) {
    TranscriptType transcriptHash(setupCtx.starkInfo.starkStruct.merkleTreeArity, setupCtx.starkInfo.starkStruct.merkleTreeCustom);
    transcriptHash.put(buffer, nElements);
    transcriptHash.getState(hash);
};

template <typename ElementType>
void Starks<ElementType>::addTranscriptGL(TranscriptType &transcript, Goldilocks::Element *buffer, uint64_t nElements)
{
    transcript.put(buffer, nElements);
};

template <typename ElementType>
void Starks<ElementType>::addTranscript(TranscriptType &transcript, ElementType *buffer, uint64_t nElements)
{
    transcript.put(buffer, nElements);
};

template <typename ElementType>
void Starks<ElementType>::ffi_treesGL_get_root(uint64_t index, ElementType *dst)
{
    treesGL[index]->getRoot(dst);
}

template <typename ElementType>
void Starks<ElementType>::calculateImPolsExpressions(uint64_t step, StepsParams &params) {
    std::vector<Dest> dests;
    for(uint64_t i = 0; i < setupCtx.starkInfo.cmPolsMap.size(); i++) {
        if(setupCtx.starkInfo.cmPolsMap[i].imPol && setupCtx.starkInfo.cmPolsMap[i].stage == step) {
            Dest destStruct(&params.pols[setupCtx.starkInfo.mapOffsets[std::make_pair("cm" + to_string(step), false)] + setupCtx.starkInfo.cmPolsMap[i].stagePos], setupCtx.starkInfo.mapSectionsN["cm" + to_string(step)]);
            destStruct.addParams(setupCtx.expressionsBin.expressionsInfo[setupCtx.starkInfo.cmPolsMap[i].expId], false);
            
            dests.push_back(destStruct);
        }
    }

    if(dests.size() == 0) return;

#ifdef __AVX512__
    ExpressionsAvx512 expressionsCtx(setupCtx);
#elif defined(__AVX2__)
    ExpressionsAvx expressionsCtx(setupCtx);
#else
    ExpressionsPack expressionsCtx(setupCtx);
#endif

    expressionsCtx.calculateExpressions(params, setupCtx.expressionsBin.expressionsBinArgsExpressions, dests, uint64_t(1 << setupCtx.starkInfo.starkStruct.nBits));
}

template <typename ElementType>
void Starks<ElementType>::calculateQuotientPolynomial(StepsParams &params) {
#ifdef __AVX512__
    ExpressionsAvx512 expressionsCtx(setupCtx);
#elif defined(__AVX2__)
    ExpressionsAvx expressionsCtx(setupCtx);
#else
    ExpressionsPack expressionsCtx(setupCtx);
#endif
    if(setupCtx.constPols.pConstTreeAddress == nullptr) {
        zklog.error("Const tree is not set");
        exitProcess();
        exit(-1);
    }
    expressionsCtx.calculateExpression(params, &params.pols[setupCtx.starkInfo.mapOffsets[std::make_pair("q", true)]], setupCtx.starkInfo.cExpId);
}

template <typename ElementType>
void Starks<ElementType>::calculateFRIPolynomial(StepsParams &params) {
#ifdef __AVX512__
    ExpressionsAvx512 expressionsCtx(setupCtx);
#elif defined(__AVX2__)
    ExpressionsAvx expressionsCtx(setupCtx);
#else
    ExpressionsPack expressionsCtx(setupCtx);
#endif
    expressionsCtx.calculateExpression(params, &params.pols[setupCtx.starkInfo.mapOffsets[std::make_pair("f", true)]], setupCtx.starkInfo.friExpId);
}