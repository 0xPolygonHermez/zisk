#ifndef MERKLETREEBN128
#define MERKLETREEBN128

#include <math.h>
#include "fr.hpp"
#include "goldilocks_base_field.hpp"
#include "goldilocks_cubic_extension.hpp"
#include "poseidon_opt.hpp"
#include "zklog.hpp"

class MerkleTreeBN128
{
private:
    void linearHash();
    void linearHash(RawFr::Element* result, Goldilocks::Element* values);

    Goldilocks::Element getElement(uint64_t idx, uint64_t subIdx);
    void genMerkleProof(RawFr::Element *proof, uint64_t idx, uint64_t offset, uint64_t n);
    void calculateRootFromProof(RawFr::Element *value, std::vector<std::vector<RawFr::Element>> &mp, uint64_t idx, uint64_t offset);

public:
    MerkleTreeBN128(){};
    MerkleTreeBN128(uint64_t arity, bool custom, Goldilocks::Element *tree);
    MerkleTreeBN128(uint64_t arity, bool custom, uint64_t _height, uint64_t _width);
    ~MerkleTreeBN128(){};

    uint64_t numNodes;
    uint64_t height;
    uint64_t width;

    Goldilocks::Element *source;
    RawFr::Element *nodes;

    bool isSourceAllocated = false;
    bool isNodesAllocated = false;

    uint64_t arity;
    bool custom;
    uint64_t nFieldElements = 1;

    uint64_t getNumSiblings();
    uint64_t getMerkleTreeWidth();
    uint64_t getMerkleProofSize();
    uint64_t getMerkleProofLength();

    uint64_t getNumNodes(uint64_t height);
    void getRoot(RawFr::Element *root);
    void setSource(Goldilocks::Element *source);
    void setNodes(RawFr::Element *nodes);

    void getGroupProof(RawFr::Element *proof, uint64_t idx);
    
    void merkelize();

    bool verifyGroupProof(RawFr::Element* root, std::vector<std::vector<RawFr::Element>> &mp, uint64_t idx, std::vector<Goldilocks::Element> &v);

    void writeFile(std::string constTreeFile);
};
#endif