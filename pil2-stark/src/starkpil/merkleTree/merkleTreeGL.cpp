#include "merkleTreeGL.hpp"
#include <cassert>
#include <algorithm> // std::max


MerkleTreeGL::MerkleTreeGL(uint64_t _arity, bool _custom, uint64_t _height, uint64_t _width, Goldilocks::Element *_source, bool allocate) : height(_height), width(_width), source(_source)
{

    if (source == NULL && allocate)
    {
        source = (Goldilocks::Element *)calloc(height * width, sizeof(Goldilocks::Element));
        isSourceAllocated = true;
    }
    arity = _arity;
    custom = _custom;
    numNodes = getNumNodes(height);
    nodes = (Goldilocks::Element *)calloc(numNodes, sizeof(Goldilocks::Element));
    isNodesAllocated = true;
};

MerkleTreeGL::MerkleTreeGL(uint64_t _arity, bool _custom, Goldilocks::Element *tree)
{
    width = Goldilocks::toU64(tree[0]);
    height = Goldilocks::toU64(tree[1]);
    source = &tree[2];
    arity = _arity;
    custom = _custom;
    numNodes = getNumNodes(height);
    nodes = &tree[2 + height * width];
    isNodesAllocated = false;
    isSourceAllocated = false;
};

MerkleTreeGL::~MerkleTreeGL()
{
    if (isSourceAllocated)
    {
        free(source);
    }
    if (isNodesAllocated)
    {
        free(nodes);
    }
}

uint64_t MerkleTreeGL::getNumSiblings() 
{
    return (arity - 1) * nFieldElements;
}

uint64_t MerkleTreeGL::getMerkleTreeWidth() 
{
    return width;
}

uint64_t MerkleTreeGL::getMerkleProofLength() {
    if(height > 1) {
        return (uint64_t)ceil(log10(height) / log10(arity));
    } 
    return 0;
}

uint64_t MerkleTreeGL::getMerkleProofSize() {
    return getMerkleProofLength() * nFieldElements;
}

uint64_t MerkleTreeGL::getNumNodes(uint64_t height)
{
    return height * nFieldElements + (height - 1) * nFieldElements;
}

void MerkleTreeGL::getRoot(Goldilocks::Element *root)
{
    std::memcpy(root, &nodes[numNodes - nFieldElements], nFieldElements * sizeof(Goldilocks::Element));
}

void MerkleTreeGL::copySource(Goldilocks::Element *_source)
{
    std::memcpy(source, _source, height * width * sizeof(Goldilocks::Element));
}

void MerkleTreeGL::setSource(Goldilocks::Element *_source)
{
    source = _source;
}

Goldilocks::Element MerkleTreeGL::getElement(uint64_t idx, uint64_t subIdx)
{
    assert((idx > 0) || (idx < width));
    return source[idx * width + subIdx];
};

void MerkleTreeGL::getGroupProof(Goldilocks::Element *proof, uint64_t idx) {
    assert(idx < height);

    for (uint64_t i = 0; i < width; i++)
    {
        proof[i] = getElement(idx, i);
    }

    genMerkleProof(&proof[width], idx, 0, height * nFieldElements);
}

void MerkleTreeGL::genMerkleProof(Goldilocks::Element *proof, uint64_t idx, uint64_t offset, uint64_t n)
{
    if (n <= nFieldElements) return;
    
    uint64_t nextIdx = idx >> 1;
    uint64_t si = (idx ^ 1) * nFieldElements;

    std::memcpy(proof, &nodes[offset + si], nFieldElements * sizeof(Goldilocks::Element));

    uint64_t nextN = (std::floor((n - 1) / 8) + 1) * nFieldElements;
    genMerkleProof(&proof[nFieldElements], nextIdx, offset + nextN * 2, nextN);
}

bool MerkleTreeGL::verifyGroupProof(Goldilocks::Element* root, std::vector<std::vector<Goldilocks::Element>> &mp, uint64_t idx, std::vector<std::vector<Goldilocks::Element>> &v) {
    Goldilocks::Element value[4];
    for(uint64_t i = 0; i < nFieldElements; ++i) {
        value[i] = Goldilocks::zero();
    }


    std::vector<Goldilocks::Element> linearValues;

    for(uint64_t i = 0; i < v.size(); ++i) {
        for(uint64_t j = 0; j < v[i].size(); ++j) {
            linearValues.push_back(v[i][j]);
        }
    }

    PoseidonGoldilocks::linear_hash_seq(value, linearValues.data(), linearValues.size());

    calculateRootFromProof(value, mp, idx, 0);
    for(uint64_t i = 0; i < 4; ++i) {
        if(Goldilocks::toU64(value[i]) != Goldilocks::toU64(root[i])) {
            return false;
        }
    }

    return true;
}

void MerkleTreeGL::calculateRootFromProof(Goldilocks::Element (&value)[4], std::vector<std::vector<Goldilocks::Element>> &mp, uint64_t idx, uint64_t offset) {
    if(offset == mp.size()) return;

    uint64_t currIdx = idx & 1;
    uint64_t nextIdx = idx / 2;

    Goldilocks::Element inputs[12];

    if(currIdx == 0) {
        std::memcpy(&inputs[0], value, nFieldElements * sizeof(Goldilocks::Element));
        std::memcpy(&inputs[4], mp[offset].data(), nFieldElements * sizeof(Goldilocks::Element));
    } else {
        std::memcpy(&inputs[0], mp[offset].data(), nFieldElements * sizeof(Goldilocks::Element));
        std::memcpy(&inputs[4], value, nFieldElements * sizeof(Goldilocks::Element));
    }
    
    for(uint64_t i = 8; i < 12; ++i) {
        inputs[i] = Goldilocks::zero();
    }

    PoseidonGoldilocks::hash_seq(value, inputs);

    calculateRootFromProof(value, mp, nextIdx, offset + 1);
}


void MerkleTreeGL::merkelize()
{
#ifdef __AVX512__
    PoseidonGoldilocks::merkletree_avx512(nodes, source, width, height);
#elif defined(__AVX2__)
    PoseidonGoldilocks::merkletree_avx(nodes, source, width, height);
#else
    PoseidonGoldilocks::merkletree_seq(nodes, source, width, height);
#endif
}

void MerkleTreeGL::writeFile(std::string constTreeFile)
{
    ofstream fw(constTreeFile.c_str(), std::fstream::out | std::fstream::binary);
    fw.write((const char *)&(width), sizeof(uint64_t));
    fw.write((const char *)&(height), sizeof(uint64_t)); 
    // fw.write((const char *)source, width * height * sizeof(Goldilocks::Element));
    // fw.write((const char *)nodes, numNodes * sizeof(Goldilocks::Element));
    // fw.close();

    uint64_t sourceOffset = sizeof(uint64_t) * 2;
    uint64_t nodesOffset = sourceOffset + width * height * sizeof(Goldilocks::Element);
    fw.close();
    writeFileParallel(constTreeFile, source, width * height * sizeof(Goldilocks::Element), sourceOffset);
    writeFileParallel(constTreeFile, nodes, numNodes * sizeof(Goldilocks::Element), nodesOffset);
}