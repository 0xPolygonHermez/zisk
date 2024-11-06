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

class ConstTree {
public:
    ConstTree () {};

    uint64_t getNumNodes(StarkInfo& starkInfo) {
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

        return acc;
    }

    uint64_t getConstTreeSizeBytesBN128B(StarkInfo& starkInfo)
    {   
        uint64_t NExtended = 1 << starkInfo.starkStruct.nBitsExt;
        uint64_t acc = getNumNodes(starkInfo);
        return 16 + (NExtended * starkInfo.nConstants) * sizeof(Goldilocks::Element) + acc * sizeof(RawFr::Element);
    }

    uint64_t getConstTreeSizeGL(StarkInfo& starkInfo)
    {   
        uint64_t NExtended = 1 << starkInfo.starkStruct.nBitsExt;
        uint64_t acc = getNumNodes(starkInfo);
        return 2 + (NExtended * starkInfo.nConstants) + acc * HASH_SIZE;
    }

    Goldilocks::Element* calculateConstTree(StarkInfo& starkInfo, Goldilocks::Element *pConstPolsAddress, Goldilocks::Element *treeAddress, std::string constTreeFile) {
        uint64_t merkleTreeArity = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? starkInfo.starkStruct.merkleTreeArity : 2;
        uint64_t merkleTreeCustom = starkInfo.starkStruct.verificationHashType == std::string("BN128") ? starkInfo.starkStruct.merkleTreeCustom : true;
        uint64_t N = 1 << starkInfo.starkStruct.nBits;
        uint64_t NExtended = 1 << starkInfo.starkStruct.nBitsExt;
        NTT_Goldilocks ntt(N);
        ntt.extendPol((Goldilocks::Element *)&treeAddress[2], (Goldilocks::Element *)pConstPolsAddress, NExtended, N, starkInfo.nConstants);
        MerkleTreeGL mt(merkleTreeArity, merkleTreeCustom, NExtended, starkInfo.nConstants, (Goldilocks::Element *)&treeAddress[2]);
        mt.merkelize();

        treeAddress[0] = Goldilocks::fromU64(starkInfo.nConstants);  
        treeAddress[1] = Goldilocks::fromU64(NExtended);
        memcpy(&treeAddress[2 + starkInfo.nConstants * NExtended], mt.nodes, mt.numNodes * sizeof(Goldilocks::Element));

        if(constTreeFile != "") {
            TimerStart(WRITING_TREE_FILE);
            mt.writeFile(constTreeFile);
            TimerStopAndLog(WRITING_TREE_FILE);
        }
        return treeAddress;
    }

    void loadConstTree(Goldilocks::Element *constTreePols, std::string constTreeFile, uint64_t constTreeSize) {
        loadFileParallel((void*)constTreePols, constTreeFile, constTreeSize);
    }

    void loadConstPols(Goldilocks::Element *constPols, std::string constPolsFile, uint64_t constPolsSize) {
        loadFileParallel((void *)constPols, constPolsFile, constPolsSize);
    }
};

#endif