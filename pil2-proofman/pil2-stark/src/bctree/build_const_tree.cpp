#include "build_const_tree.hpp"
#include <string>
#include <nlohmann/json.hpp>
#include <algorithm>
#include <sys/mman.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <fstream>
#include <filesystem>
#include <cstdint>

using namespace std;
using json = nlohmann::json;

Goldilocks fr;

void buildConstTree(const string constFile, const string starkInfoFile, const string constTreeFile, const string verKeyFile)
{
    TimerStart(BUILD_CONST_TREE);

    json starkInfoJson;
    file2json(starkInfoFile, starkInfoJson);

    uint64_t nBits = starkInfoJson["starkStruct"]["nBits"];
    uint64_t nBitsExt = starkInfoJson["starkStruct"]["nBitsExt"];
    uint64_t N = 1 << nBits;
    uint64_t NExtended = 1 << nBitsExt;
    uint64_t nPols = starkInfoJson["nConstants"];
    std::string verificationHashType = starkInfoJson["starkStruct"]["verificationHashType"];

    uintmax_t constPolsSize = nPols * sizeof(Goldilocks::Element) * N;
    
    TimerStart(LOADING_CONST_POLS);
    Goldilocks::Element *pConstPols = (Goldilocks::Element *)loadFileParallel(constFile, constPolsSize);
    Goldilocks::Element *pConstPolsExt = new Goldilocks::Element[NExtended * nPols];
    TimerStopAndLog(LOADING_CONST_POLS);

    TimerStart(EXTEND_CONST_POLS);
    NTT_Goldilocks ntt(N);
    ntt.extendPol(pConstPolsExt, pConstPols, NExtended, N, nPols);
    TimerStopAndLog(EXTEND_CONST_POLS);

    if (verificationHashType == "GL") {
        TimerStart(MERKELIZE_CONST_TREE);
        Goldilocks::Element root[4];
        MerkleTreeGL mt(2, true, NExtended, nPols);
        Goldilocks::Element *buffNodes = new Goldilocks::Element[mt.numNodes];
        mt.setSource(pConstPolsExt);
        mt.setNodes(buffNodes);
        mt.merkelize();
        mt.getRoot(root);
        TimerStopAndLog(MERKELIZE_CONST_TREE);

        TimerStart(GENERATING_FILES);

        if (verKeyFile != "") {
            json value;
            value[0] = Goldilocks::toU64(root[0]);
            value[1] = Goldilocks::toU64(root[1]);
            value[2] = Goldilocks::toU64(root[2]);
            value[3] = Goldilocks::toU64(root[3]);
            json2file(value, verKeyFile);
        }

        // ConstTree
        if(constTreeFile != "") {
           mt.writeFile(constTreeFile);
        }

        TimerStopAndLog(GENERATING_FILES);

    } else if(verificationHashType == "BN128"){
        TimerStart(MERKELIZE_CONST_TREE);
        RawFr::Element rootC;
        uint64_t merkleTreeArity = starkInfoJson["starkStruct"].contains("merkleTreeArity") ? starkInfoJson["starkStruct"]["merkleTreeArity"].get<uint64_t>() : 16;
        bool merkleTreeCustom = starkInfoJson["starkStruct"].contains("merkleTreeCustom") ? starkInfoJson["starkStruct"]["merkleTreeCustom"].get<bool>() : false;

        MerkleTreeBN128 mt(merkleTreeArity, merkleTreeCustom, NExtended, nPols);
        RawFr::Element *buffNodes = new RawFr::Element[mt.numNodes];
        mt.setSource(pConstPolsExt);
        mt.setNodes(buffNodes);
        mt.merkelize();
        mt.getRoot(&rootC);
        TimerStopAndLog(MERKELIZE_CONST_TREE);

        if (verKeyFile != "") {
            json value;
            RawFr rawfr;
            value = rawfr.toString(rootC);
            json2file(value, verKeyFile);
        }

        TimerStart(GENERATING_FILES);

        // ConstTree
        if(constTreeFile != "") {
            mt.writeFile(constTreeFile);
        }
        TimerStopAndLog(GENERATING_FILES);
    } else {
        cerr << "Invalid Hash Type: " << verificationHashType << endl;
        exit(-1);
    }

    delete[] pConstPolsExt;
    TimerStopAndLog(BUILD_CONST_TREE);
}