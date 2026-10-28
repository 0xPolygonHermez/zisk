#ifndef MUL_ADD_HPP
#define MUL_ADD_HPP

#include <stdint.h>

// Computes t = a * b + c, for a, b and c of L u64 LE limbs and t of 2*L limbs. The result always
// fits: (2^n - 1)^2 + (2^n - 1) < 2^(2n). t must not overlap a, b or c.
template <int L>
inline void mul_add (uint64_t * t, const uint64_t * a, const uint64_t * b, const uint64_t * c)
{
    for (int i = 0; i < L; i++) t[i] = c[i];
    for (int i = L; i < 2 * L; i++) t[i] = 0;

    // Schoolbook multiplication, one row per limb of a, accumulating into t
    for (int i = 0; i < L; i++)
    {
        uint64_t carry = 0;
        for (int j = 0; j < L; j++)
        {
            unsigned __int128 p = (unsigned __int128)a[i] * b[j] + t[i + j] + carry;
            t[i + j] = (uint64_t)p;
            carry = (uint64_t)(p >> 64);
        }
        // Propagate the row carry; it can only ripple while the limbs above are all ones
        for (int k = i + L; carry != 0 && k < 2 * L; k++)
        {
            unsigned __int128 s = (unsigned __int128)t[k] + carry;
            t[k] = (uint64_t)s;
            carry = (uint64_t)(s >> 64);
        }
    }
}

#endif
