#ifndef MERKLEHASH_GOLDILOCKS
#define MERKLEHASH_GOLDILOCKS

#include <cassert>
#include <math.h> /* floor */
#include "goldilocks_base_field.hpp"
#include "goldilocks_cubic_extension.hpp"
#include "poseidon_goldilocks.hpp"

#define MERKLEHASHGOLDILOCKS_HEADER_SIZE 2
#define MERKLEHASHGOLDILOCKS_ARITY 2
class MerklehashGoldilocks
{
public:
    inline static void root(Goldilocks::Element *root, Goldilocks::Element *tree, uint64_t numElementsTree)
    {
        std::memcpy(root, &tree[numElementsTree - HASH_SIZE], HASH_SIZE * sizeof(Goldilocks::Element));
    }

    static void root(Goldilocks::Element (&root)[HASH_SIZE], Goldilocks::Element *tree, uint64_t numElementsTree)
    {
        std::memcpy(root, &tree[numElementsTree - HASH_SIZE], HASH_SIZE * sizeof(Goldilocks::Element));
    }

    static inline uint64_t getTreeNumElements(uint64_t degree)
    {
        return degree * HASH_SIZE + (degree - 1) * HASH_SIZE;
    };
};

#endif