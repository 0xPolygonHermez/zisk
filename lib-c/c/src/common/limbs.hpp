#ifndef LIMBS_HPP
#define LIMBS_HPP

#include <stdint.h>

// Helpers on unsigned integers of L u64 LE limbs

// Returns a < b
template <int L>
inline bool limbs_less_than (const uint64_t * a, const uint64_t * b)
{
    for (int i = L - 1; i >= 0; i--)
    {
        if (a[i] != b[i]) return a[i] < b[i];
    }
    return false;
}

// a -= b, ignoring the final borrow
template <int L>
inline void limbs_sub (uint64_t * a, const uint64_t * b)
{
    uint64_t borrow = 0;
    for (int i = 0; i < L; i++)
    {
        unsigned __int128 d = (unsigned __int128)a[i] - b[i] - borrow;
        a[i] = (uint64_t)d;
        borrow = (uint64_t)(d >> 64) & 1;
    }
}

// Reduces a to [0, m) by repeated subtraction; meant for values that are already canonical or
// just a few multiples of m above it, where it costs one comparison in the common case
template <int L>
inline void limbs_reduce (uint64_t * a, const uint64_t * m)
{
    while (!limbs_less_than<L>(a, m)) limbs_sub<L>(a, m);
}

#endif
