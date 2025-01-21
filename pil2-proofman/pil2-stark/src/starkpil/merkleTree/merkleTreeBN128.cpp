
#include "merkleTreeBN128.hpp"
#include <algorithm> // std::max
#include <cassert>

MerkleTreeBN128::MerkleTreeBN128(uint64_t _arity, bool _custom, uint64_t _height, uint64_t _width) : height(_height), width(_width)
{

    numNodes = getNumNodes(height);
    arity = _arity;
    custom = _custom;
}

MerkleTreeBN128::MerkleTreeBN128(uint64_t _arity, bool _custom, Goldilocks::Element *tree)
{
    width = Goldilocks::toU64(tree[0]);
    height = Goldilocks::toU64(tree[1]);
    source = &tree[2];
    arity = _arity;
    custom = _custom;
    numNodes = getNumNodes(height);
    
    nodes = (RawFr::Element *)&source[width * height];
}

uint64_t MerkleTreeBN128::getNumSiblings() 
{
    return arity * nFieldElements;
}

uint64_t MerkleTreeBN128::getMerkleTreeWidth()
{
    return width;
}

uint64_t MerkleTreeBN128::getMerkleProofLength()
{
    return ceil((double)log(height) / log(arity));
}


uint64_t MerkleTreeBN128::getMerkleProofSize()
{
    return getMerkleProofLength() * arity * sizeof(RawFr::Element);
}

uint64_t MerkleTreeBN128::getNumNodes(uint64_t n)
{   
    uint n_tmp = n;
    uint64_t nextN = floor(((double)(n_tmp - 1) / arity) + 1);
    uint64_t acc = nextN * arity;
    while (n_tmp > 1)
    {
        // FIll with zeros if n nodes in the leve is not even
        n_tmp = nextN;
        nextN = floor((n_tmp - 1) / arity) + 1;
        if (n_tmp > 1)
        {
            acc += nextN * arity;
        }
        else
        {
            acc += 1;
        }
    }

    return acc;
}


void MerkleTreeBN128::getRoot(RawFr::Element *root)
{
    std::memcpy(root, &nodes[numNodes - 1], sizeof(RawFr::Element));
}


void MerkleTreeBN128::setSource(Goldilocks::Element *_source)
{
    source = _source;
}

void MerkleTreeBN128::setNodes(RawFr::Element *_nodes)
{
    nodes = _nodes;
}

Goldilocks::Element MerkleTreeBN128::getElement(uint64_t idx, uint64_t subIdx)
{
    assert((idx > 0) || (idx < width));
    return source[width * idx + subIdx];
}

void MerkleTreeBN128::getGroupProof(RawFr::Element *proof, uint64_t idx)
{
    assert(idx < height);

    Goldilocks::Element v[width];
    for (uint64_t i = 0; i < width; i++)
    {
        v[i] = getElement(idx, i);
    }
    std::memcpy(proof, &v[0], width * sizeof(Goldilocks::Element));
    void *proofCursor = (uint8_t *)proof + width * sizeof(Goldilocks::Element);

    genMerkleProof((RawFr::Element *)proofCursor, idx, 0, height);
}

void MerkleTreeBN128::genMerkleProof(RawFr::Element *proof, uint64_t idx, uint64_t offset, uint64_t n)
{
    if (n <= 1) return;

    uint64_t nBitsArity = std::ceil(std::log2(arity));

    uint64_t nextIdx = idx >> nBitsArity;
    uint64_t si = idx ^ (idx & (arity - 1));

    std::memcpy(proof, &nodes[offset + si], arity * sizeof(RawFr::Element));
    uint64_t nextN = (std::floor((n - 1) / arity) + 1);
    genMerkleProof(&proof[arity], nextIdx, offset + nextN * arity, nextN);
}

void MerkleTreeBN128::linearHash(RawFr::Element* result, Goldilocks::Element* values)
{
    if (width > 4)
    {
        uint64_t widthRawFrElements = ceil((double)width / FIELD_EXTENSION);
        RawFr::Element buff[widthRawFrElements]; 

        uint64_t nElementsGL = (width > FIELD_EXTENSION + 1) ? ceil((double)width / FIELD_EXTENSION) : 0;
        for (uint64_t j = 0; j < nElementsGL; j++)
        {
            uint64_t pending = width - j * FIELD_EXTENSION;
            uint64_t batch;
            (pending >= FIELD_EXTENSION) ? batch = FIELD_EXTENSION : batch = pending;
            for (uint64_t k = 0; k < batch; k++)
            {
                buff[j].v[k] = Goldilocks::toU64(values[j * FIELD_EXTENSION + k]);
            }
            RawFr::field.toMontgomery(buff[j], buff[j]);
        }

        uint pending = nElementsGL;
        Poseidon_opt p;
        std::vector<RawFr::Element> elements(arity + 1);
        while (pending > 0)
        {
            std::memset(&elements[0], 0, (arity + 1) * sizeof(RawFr::Element));
            if (pending >= arity)
            {
                std::memcpy(&elements[1], &buff[nElementsGL - pending], arity * sizeof(RawFr::Element));
                std::memcpy(&elements[0], &result[0], sizeof(RawFr::Element));
                p.hash(elements, &result[0]);
                pending = pending - arity;
            }
            else if(custom) 
            {
                std::memcpy(&elements[1], &buff[nElementsGL - pending], pending * sizeof(RawFr::Element));
                std::memcpy(&elements[0], &result[0], sizeof(RawFr::Element));
                p.hash(elements, &result[0]);
                pending = 0;
            }
            else
            {
                std::vector<RawFr::Element> elements_last(pending + 1);
                std::memcpy(&elements_last[1], &buff[nElementsGL - pending], pending * sizeof(RawFr::Element));
                std::memcpy(&elements_last[0], &result[0], sizeof(RawFr::Element));
                p.hash(elements_last, &result[0]);
                pending = 0;
            }
        } 
    } else {
        for (uint64_t k = 0; k < width; k++)
        {
            result[0].v[k] = Goldilocks::toU64(values[k]);
        }
        RawFr::field.toMontgomery(result[0], result[0]);
    }
}

/*
 * LinearHash BN128
 */
void MerkleTreeBN128::linearHash()
{
    if (width > 4)
    {
        uint64_t widthRawFrElements = ceil((double)width / FIELD_EXTENSION);
        RawFr::Element *buff = (RawFr::Element *)calloc(height * widthRawFrElements, sizeof(RawFr::Element));

    uint64_t nElementsGL = (width > FIELD_EXTENSION + 1) ? ceil((double)width / FIELD_EXTENSION) : 0;
#pragma omp parallel for
        for (uint64_t i = 0; i < height; i++)
        {
            for (uint64_t j = 0; j < nElementsGL; j++)
            {
                uint64_t pending = width - j * FIELD_EXTENSION;
                uint64_t batch;
                (pending >= FIELD_EXTENSION) ? batch = FIELD_EXTENSION : batch = pending;
                for (uint64_t k = 0; k < batch; k++)
                {
                    buff[i * nElementsGL + j].v[k] = Goldilocks::toU64(source[i * width + j * FIELD_EXTENSION + k]);
                }
                RawFr::field.toMontgomery(buff[i * nElementsGL + j], buff[i * nElementsGL + j]);
            }
        }

#pragma omp parallel for
        for (uint64_t i = 0; i < height; i++)
        {
            uint pending = nElementsGL;
            Poseidon_opt p;
            std::vector<RawFr::Element> elements(arity + 1);
            while (pending > 0)
            {
                std::memset(&elements[0], 0, (arity + 1) * sizeof(RawFr::Element));
                if (pending >= arity)
                {
                    std::memcpy(&elements[1], &buff[i * nElementsGL + nElementsGL - pending], arity * sizeof(RawFr::Element));
                    std::memcpy(&elements[0], &nodes[i], sizeof(RawFr::Element));
                    p.hash(elements, &nodes[i]);
                    pending = pending - arity;
                }
                else if(custom) 
                {
                    std::memcpy(&elements[1], &buff[i * nElementsGL + nElementsGL - pending], pending * sizeof(RawFr::Element));
                    std::memcpy(&elements[0], &nodes[i], sizeof(RawFr::Element));
                    p.hash(elements, &nodes[i]);
                    pending = 0;
                }
                else
                {
                    std::vector<RawFr::Element> elements_last(pending + 1);
                    std::memcpy(&elements_last[1], &buff[i * nElementsGL + nElementsGL - pending], pending * sizeof(RawFr::Element));
                    std::memcpy(&elements_last[0], &nodes[i], sizeof(RawFr::Element));
                    p.hash(elements_last, &nodes[i]);
                    pending = 0;
                }
            }
        }
        free(buff);
    }
    else
    {
#pragma omp parallel for
        for (uint64_t i = 0; i < height; i++)
        {
            for (uint64_t k = 0; k < width; k++)
            {
                nodes[i].v[k] = Goldilocks::toU64(source[i * width + k]);
            }
            RawFr::field.toMontgomery(nodes[i], nodes[i]);
        }
    }
}

void MerkleTreeBN128::calculateRootFromProof(RawFr::Element *value, std::vector<std::vector<RawFr::Element>> &mp, uint64_t idx, uint64_t offset) {
    if(offset == mp.size()) return;

    uint64_t nBitsArity = std::ceil(std::log2(arity));

    uint64_t currIdx = idx & (arity - 1);
    uint64_t nextIdx = idx >> nBitsArity;

    Poseidon_opt p;
    std::vector<RawFr::Element> elements(arity + 1);
    std::memset(&elements[0], 0, (arity + 1) * sizeof(RawFr::Element));

    for(uint64_t i = 0; i < arity; ++i) {
        std::memcpy(&elements[i], &mp[offset][i], sizeof(RawFr::Element));
    }

    std::memcpy(&elements[currIdx], &value[0], sizeof(RawFr::Element));
    p.hash(elements, &value[0]);

    calculateRootFromProof(value, mp, nextIdx, offset + 1);

}


bool MerkleTreeBN128::verifyGroupProof(RawFr::Element* root, std::vector<std::vector<RawFr::Element>> &mp, uint64_t idx, std::vector<Goldilocks::Element> &v) {
    RawFr::Element value[1];
    value[0] = RawFr::field.zero();

    linearHash(value, v.data());

    calculateRootFromProof(&value[0], mp, idx, 0);

    if (RawFr::field.eq(root[0], value[0])) {
        return false;
    }
    return true;
}


void MerkleTreeBN128::merkelize()
{

    linearHash();

    RawFr::Element *cursor = &nodes[0];
    uint64_t n256 = height;
    uint64_t nextN256 = floor((double)(n256 - 1) / arity) + 1;
    RawFr::Element *cursorNext = &nodes[nextN256 * arity];
    while (n256 > 1)
    {
        uint64_t batches = ceil((double)n256 / arity);
#pragma omp parallel for
        for (uint64_t i = 0; i < batches; i++)
        {
            Poseidon_opt p;
            vector<RawFr::Element> elements(arity + 1);
            std::memset(&elements[0], 0, (arity + 1) * sizeof(RawFr::Element));
            uint numHashes = (i == batches - 1) ? n256 - i*arity : arity;
            std::memcpy(&elements[1], &cursor[i * arity], numHashes * sizeof(RawFr::Element));
            p.hash(elements, &cursorNext[i]);
        }

        n256 = nextN256;
        nextN256 = floor((double)(n256 - 1) / arity) + 1;
        cursor = cursorNext;
        cursorNext = &cursor[nextN256 * arity];
    }
}

void MerkleTreeBN128::writeFile(std::string constTreeFile) {
    std::ofstream fw(constTreeFile.c_str(), std::fstream::out | std::fstream::binary);
    fw.write((const char *)&(width), sizeof(width));
    fw.write((const char *)&(height), sizeof(height));
    fw.write((const char *)source, width * height * sizeof(Goldilocks::Element));
    fw.write((const char *)nodes, numNodes * sizeof(RawFr::Element));
    fw.close();
}