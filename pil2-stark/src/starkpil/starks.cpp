#include "definitions.hpp"
#include "starks.hpp"
#include "zklog.hpp"
#include "exit_process.hpp"

template <typename ElementType>
void Starks<ElementType>::extendAndMerkelizeCustomCommit(uint64_t commitId, uint64_t step, Goldilocks::Element *buffer, Goldilocks::Element *bufferExt, FRIProof<ElementType> &proof, Goldilocks::Element *pBuffHelper, string bufferFile)
{   
    uint64_t N = 1 << setupCtx.starkInfo.starkStruct.nBits;
    uint64_t NExtended = 1 << setupCtx.starkInfo.starkStruct.nBitsExt;

    std::string section = setupCtx.starkInfo.customCommits[commitId].name + to_string(step);
    uint64_t nCols = setupCtx.starkInfo.mapSectionsN[section];
    Goldilocks::Element *pBuff = buffer;
    Goldilocks::Element *pBuffExtended = bufferExt;
    ElementType *pBuffNodes = (ElementType *)(&bufferExt[NExtended * nCols]);

    NTT_Goldilocks ntt(N);
    if(pBuffHelper != nullptr) {
        ntt.extendPol(pBuffExtended, pBuff, NExtended, N, nCols, pBuffHelper);
    } else {
        ntt.extendPol(pBuffExtended, pBuff, NExtended, N, nCols);
    }
    
    uint64_t pos = setupCtx.starkInfo.nStages + 2 + commitId;
    treesGL[pos]->setSource(pBuffExtended);
    treesGL[pos]->setNodes(pBuffNodes);
    treesGL[pos]->merkelize();
    treesGL[pos]->getRoot(&proof.proof.roots[pos - 1][0]);

    if(bufferFile != "") {
        ofstream fw(bufferFile.c_str(), std::fstream::out | std::fstream::binary);
        writeFileParallel(bufferFile, pBuff, N * nCols * sizeof(Goldilocks::Element), 0);
        writeFileParallel(bufferFile, pBuffExtended, NExtended * nCols * sizeof(Goldilocks::Element), N * nCols * sizeof(Goldilocks::Element));
        writeFileParallel(bufferFile, treesGL[pos]->nodes, treesGL[pos]->numNodes * sizeof(Goldilocks::Element), (NExtended + N) * nCols * sizeof(Goldilocks::Element));
        fw.close();
    }
}

template <typename ElementType>
void Starks<ElementType>::loadCustomCommit(uint64_t commitId, uint64_t step, Goldilocks::Element *buffer, Goldilocks::Element *bufferExt, FRIProof<ElementType> &proof, string bufferFile)
{   
    uint64_t N = 1 << setupCtx.starkInfo.starkStruct.nBits;
    uint64_t NExtended = 1 << setupCtx.starkInfo.starkStruct.nBitsExt;

    std::string section = setupCtx.starkInfo.customCommits[commitId].name + to_string(step);
    uint64_t nCols = setupCtx.starkInfo.mapSectionsN[section];
    Goldilocks::Element *pBuff = buffer;
    Goldilocks::Element *pBuffExtended = bufferExt;
    ElementType *pBuffNodes = (ElementType *)(&bufferExt[NExtended * nCols]);

    uint64_t pos = setupCtx.starkInfo.nStages + 2 + commitId;

    Goldilocks::Element* tmpBuff = (Goldilocks::Element *)loadFileParallel(bufferFile, ((N + NExtended) * nCols + treesGL[pos]->getNumNodes(NExtended)) * sizeof(Goldilocks::Element));
    memcpy(pBuff, &tmpBuff[0], N * nCols * sizeof(Goldilocks::Element));
    memcpy(pBuffExtended, &tmpBuff[N * nCols], NExtended * nCols * sizeof(Goldilocks::Element));
    ElementType *tmpBuffNodes = (ElementType *)(&tmpBuff[N * nCols + NExtended * nCols]);
    memcpy(pBuffNodes, tmpBuffNodes, treesGL[pos]->numNodes * sizeof(ElementType));

    treesGL[pos]->setSource(pBuffExtended);
    treesGL[pos]->setNodes(pBuffNodes);
    treesGL[pos]->getRoot(&proof.proof.roots[pos - 1][0]);

}

template <typename ElementType>
void Starks<ElementType>::extendAndMerkelize(uint64_t step, Goldilocks::Element *trace, Goldilocks::Element *aux_trace, FRIProof<ElementType> &proof, Goldilocks::Element *pBuffHelper)
{   
    uint64_t N = 1 << setupCtx.starkInfo.starkStruct.nBits;
    uint64_t NExtended = 1 << setupCtx.starkInfo.starkStruct.nBitsExt;

    std::string section = "cm" + to_string(step);  
    uint64_t nCols = setupCtx.starkInfo.mapSectionsN["cm" + to_string(step)];
    
    Goldilocks::Element *pBuff = step == 1 ? trace : &aux_trace[setupCtx.starkInfo.mapOffsets[make_pair(section, false)]];
    Goldilocks::Element *pBuffExtended = &aux_trace[setupCtx.starkInfo.mapOffsets[make_pair(section, true)]];
    ElementType *pBuffNodes = (ElementType *)(&aux_trace[setupCtx.starkInfo.mapOffsets[make_pair("mt" + to_string(step), true)]]);

    NTT_Goldilocks ntt(N);
    if(pBuffHelper != nullptr) {
        ntt.extendPol(pBuffExtended, pBuff, NExtended, N, nCols, pBuffHelper);
    } else {
        ntt.extendPol(pBuffExtended, pBuff, NExtended, N, nCols);
    }
    
    treesGL[step - 1]->setSource(pBuffExtended);
    treesGL[step - 1]->setNodes(pBuffNodes);
    treesGL[step - 1]->merkelize();
    treesGL[step - 1]->getRoot(&proof.proof.roots[step - 1][0]);
}

template <typename ElementType>
void Starks<ElementType>::commitStage(uint64_t step, Goldilocks::Element *trace, Goldilocks::Element *aux_trace, FRIProof<ElementType> &proof, Goldilocks::Element* pBuffHelper)
{  

    if (step <= setupCtx.starkInfo.nStages)
    {
        extendAndMerkelize(step, trace, aux_trace, proof, pBuffHelper);
    }
    else
    {
        computeQ(step, aux_trace, proof, pBuffHelper);
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
            Goldilocks3::mul((Goldilocks3::Element &)cmQ[(i * setupCtx.starkInfo.qDeg + p) * FIELD_EXTENSION], (Goldilocks3::Element &)buffer[setupCtx.starkInfo.mapOffsets[std::make_pair("q", true)] + (p * N + i) * FIELD_EXTENSION], setupCtx.proverHelpers.S[p]);
        }
    }

    memset(&cmQ[N * setupCtx.starkInfo.qDeg * setupCtx.starkInfo.qDim], 0, (NExtended - N) * setupCtx.starkInfo.qDeg * setupCtx.starkInfo.qDim * sizeof(Goldilocks::Element));

    if(pBuffHelper != nullptr) {
        nttExtended.NTT(cmQ, cmQ, NExtended, nCols, pBuffHelper);
    } else {
        nttExtended.NTT(cmQ, cmQ, NExtended, nCols);
    }

    treesGL[step - 1]->setSource(&buffer[setupCtx.starkInfo.mapOffsets[std::make_pair("cm" + to_string(step), true)]]);
    treesGL[step - 1]->setNodes((ElementType *)(&buffer[setupCtx.starkInfo.mapOffsets[std::make_pair("mt" + to_string(step), true)]]));
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
void Starks<ElementType>::computeEvals(StepsParams &params, Goldilocks::Element *LEv, FRIProof<ElementType> &proof)
{
    evmap(params, LEv);
    proof.proof.setEvals(params.evals);
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
            Goldilocks3::sub((Goldilocks3::Element &)(xDivXSub[(k + i * NExtended) * FIELD_EXTENSION]), setupCtx.proverHelpers.x[k], (Goldilocks3::Element &)(xis[i * FIELD_EXTENSION]));
        }
    }

    Polinomial xDivXSubXi_(xDivXSub, NExtended * setupCtx.starkInfo.openingPoints.size(), FIELD_EXTENSION, FIELD_EXTENSION);
    Polinomial::batchInverseParallel(xDivXSubXi_, xDivXSubXi_);

    for (uint64_t i = 0; i < setupCtx.starkInfo.openingPoints.size(); ++i)
    {
#pragma omp parallel for
        for (uint64_t k = 0; k < NExtended; k++)
        {
            Goldilocks3::mul((Goldilocks3::Element &)(xDivXSub[(k + i * NExtended) * FIELD_EXTENSION]), (Goldilocks3::Element &)(xDivXSub[(k + i * NExtended) * FIELD_EXTENSION]), setupCtx.proverHelpers.x[k]);
        }
    }
}

template <typename ElementType>
void Starks<ElementType>::evmap(StepsParams& params, Goldilocks::Element *LEv)
{
    uint64_t extendBits = setupCtx.starkInfo.starkStruct.nBitsExt - setupCtx.starkInfo.starkStruct.nBits;
    u_int64_t size_eval = setupCtx.starkInfo.evMap.size();

    uint64_t N = 1 << setupCtx.starkInfo.starkStruct.nBits;

    int num_threads = omp_get_max_threads();
    int size_thread = size_eval * FIELD_EXTENSION;
    Goldilocks::Element *evals_acc = &params.aux_trace[setupCtx.starkInfo.mapOffsets[std::make_pair("evals", true)]];
    memset(&evals_acc[0], 0, num_threads * size_thread * sizeof(Goldilocks::Element));
    
    Polinomial *ordPols = new Polinomial[size_eval];

    for (uint64_t i = 0; i < size_eval; i++)
    {
        EvMap ev = setupCtx.starkInfo.evMap[i];
        string type = ev.type == EvMap::eType::cm ? "cm" : ev.type == EvMap::eType::custom ? "custom" : "fixed";
        Goldilocks::Element *pAddress = type == "cm" ? params.aux_trace : type == "custom" ? params.pCustomCommitsExtended[ev.commitId] : &params.pConstPolsExtendedTreeAddress[2];
        PolMap polInfo = type == "cm" ? setupCtx.starkInfo.cmPolsMap[ev.id] : type == "custom" ? setupCtx.starkInfo.customCommitsMap[ev.commitId][ev.id] : setupCtx.starkInfo.constPolsMap[ev.id];
        setupCtx.starkInfo.getPolynomial(ordPols[i], pAddress, type, polInfo, true);
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
            std::memcpy((Goldilocks3::Element &)(params.evals[i * FIELD_EXTENSION]), sum, FIELD_EXTENSION * sizeof(Goldilocks::Element));
        }
    }
    delete[] ordPols;
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
void Starks<ElementType>::ffi_treesGL_set_root(uint64_t index, FRIProof<ElementType> &proof)
{
    treesGL[index]->getRoot(&proof.proof.roots[index][0]);
}

template <typename ElementType>
void Starks<ElementType>::calculateImPolsExpressions(uint64_t step, StepsParams &params) {
    std::vector<Dest> dests;
    for(uint64_t i = 0; i < setupCtx.starkInfo.cmPolsMap.size(); i++) {
        if(setupCtx.starkInfo.cmPolsMap[i].imPol && setupCtx.starkInfo.cmPolsMap[i].stage == step) {
            Goldilocks::Element* pAddress = setupCtx.starkInfo.cmPolsMap[i].stage == 1 ? params.trace : params.aux_trace;
            Dest destStruct(&pAddress[setupCtx.starkInfo.mapOffsets[std::make_pair("cm" + to_string(step), false)] + setupCtx.starkInfo.cmPolsMap[i].stagePos], setupCtx.starkInfo.mapSectionsN["cm" + to_string(step)]);
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

    expressionsCtx.calculateExpressions(params, setupCtx.expressionsBin.expressionsBinArgsExpressions, dests, uint64_t(1 << setupCtx.starkInfo.starkStruct.nBits), false);
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
    expressionsCtx.calculateExpression(params, &params.aux_trace[setupCtx.starkInfo.mapOffsets[std::make_pair("q", true)]], setupCtx.starkInfo.cExpId);
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
    expressionsCtx.calculateExpression(params, &params.aux_trace[setupCtx.starkInfo.mapOffsets[std::make_pair("f", true)]], setupCtx.starkInfo.friExpId);

    for(uint64_t step = 0; step < setupCtx.starkInfo.starkStruct.steps.size() - 1; ++step) { 
        Goldilocks::Element *src = &params.aux_trace[setupCtx.starkInfo.mapOffsets[std::make_pair("fri_" + to_string(step + 1), true)]];
        treesFRI[step]->setSource(src);

        ElementType *nodes = (ElementType *)(&params.aux_trace[setupCtx.starkInfo.mapOffsets[std::make_pair("mt_fri_" + to_string(step + 1), true)]]);
        treesFRI[step]->setNodes(nodes);
    }
}