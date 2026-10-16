#include "arith256.hpp"
#include "../common/utils.hpp"
#include "../common/mul_add.hpp"

int Arith256 (
    const uint64_t * _a,  // 4 x 64 bits
    const uint64_t * _b,  // 4 x 64 bits
    const uint64_t * _c,  // 4 x 64 bits
          uint64_t * _dl, // 4 x 64 bits
          uint64_t * _dh  // 4 x 64 bits
)
{
    // d = a * b + c, computed in a temporary since the outputs may overlap the inputs
    uint64_t d[8];
    mul_add<4>(d, _a, _b, _c);

    // Decompose d = dl + dh<<256
    memcpy(_dl, &d[0], 4 * sizeof(uint64_t));
    memcpy(_dh, &d[4], 4 * sizeof(uint64_t));

    return 0;
}

/*******************/
/* ARITH256MODFAST */
/*******************/

// Montgomery arithmetic modulo a known odd 256-bit modulus m, with R = 2^256
struct Arith256Modulus
{
    uint64_t m[4];
    uint64_t minv; // -m^-1 mod 2^64
    uint64_t r2[4]; // R^2 mod m
};

static Arith256Modulus arith256_modulus (const char * hex)
{
    Arith256Modulus M;
    mpz_class m(hex, 16), r2;
    memset(M.m, 0, sizeof(M.m));
    mpz_export(M.m, NULL, -1, 8, -1, 0, m.get_mpz_t());
    // Newton iteration: every step doubles the number of correct low bits of m^-1
    uint64_t inv = M.m[0];
    for (int i = 0; i < 5; i++) inv *= 2 - M.m[0] * inv;
    M.minv = 0 - inv;
    r2 = (mpz_class(1) << 512) % m;
    memset(M.r2, 0, sizeof(M.r2));
    mpz_export(M.r2, NULL, -1, 8, -1, 0, r2.get_mpz_t());
    return M;
}

// The moduli seen in practice: secp256k1 base and scalar fields, BN254 base and scalar fields,
// and the Starknet field
static const Arith256Modulus arith256_moduli[] =
{
    arith256_modulus("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
    arith256_modulus("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141"),
    arith256_modulus("30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
    arith256_modulus("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001"),
    arith256_modulus("0800000000000011000000000000000000000000000000000000000000000001"),
};

// r = a * b * R^-1 mod m, for a, b < m (CIOS); r may overlap a or b
static inline void arith256_mont_mul (uint64_t * r, const uint64_t * a, const uint64_t * b, const Arith256Modulus &M)
{
    uint64_t t[6] = {0, 0, 0, 0, 0, 0};
    for (int i = 0; i < 4; i++)
    {
        // t += a[i] * b
        unsigned __int128 c = 0;
        for (int j = 0; j < 4; j++)
        {
            c += (unsigned __int128)t[j] + (unsigned __int128)a[i] * b[j];
            t[j] = (uint64_t)c;
            c >>= 64;
        }
        c += t[4];
        t[4] = (uint64_t)c;
        t[5] = (uint64_t)(c >> 64);

        // t = (t + q * m) / 2^64, with q chosen so that the low limb becomes zero
        uint64_t q = t[0] * M.minv;
        c = (unsigned __int128)t[0] + (unsigned __int128)q * M.m[0];
        c >>= 64;
        for (int j = 1; j < 4; j++)
        {
            c += (unsigned __int128)t[j] + (unsigned __int128)q * M.m[j];
            t[j - 1] = (uint64_t)c;
            c >>= 64;
        }
        c += t[4];
        t[3] = (uint64_t)c;
        t[4] = t[5] + (uint64_t)(c >> 64);
    }

    // t < 2m: subtract m once if needed
    if ((t[4] != 0) || !limbs_less_than<4>(t, M.m)) limbs_sub<4>(t, M.m);
    memcpy(r, t, 4 * sizeof(uint64_t));
}

int Arith256ModFast (
    const uint64_t * _a,      // 4 x 64 bits
    const uint64_t * _b,      // 4 x 64 bits
    const uint64_t * _c,      // 4 x 64 bits
    const uint64_t * _module, // 4 x 64 bits
          uint64_t * _d       // 4 x 64 bits
)
{
    const Arith256Modulus * M = NULL;
    for (size_t i = 0; i < sizeof(arith256_moduli) / sizeof(arith256_moduli[0]); i++)
    {
        if (memcmp(_module, arith256_moduli[i].m, 4 * sizeof(uint64_t)) == 0)
        {
            M = &arith256_moduli[i];
            break;
        }
    }
    if ((M == NULL) ||
        !limbs_less_than<4>(_a, M->m) || !limbs_less_than<4>(_b, M->m) || !limbs_less_than<4>(_c, M->m))
    {
        return -1;
    }

    // a * b = mont_mul(a, mont_mul(b, R^2)), since mont_mul(b, R^2) = b * R
    uint64_t br[4], d[4];
    arith256_mont_mul(br, _b, M->r2, *M);
    arith256_mont_mul(d, _a, br, *M);

    // d = d + c mod m, with d, c < m
    unsigned __int128 carry = 0;
    for (int i = 0; i < 4; i++)
    {
        carry += (unsigned __int128)d[i] + _c[i];
        d[i] = (uint64_t)carry;
        carry >>= 64;
    }
    if ((carry != 0) || !limbs_less_than<4>(d, M->m)) limbs_sub<4>(d, M->m);

    memcpy(_d, d, 4 * sizeof(uint64_t));
    return 0;
}

int Arith256Mod (
    const uint64_t * _a,      // 4 x 64 bits
    const uint64_t * _b,      // 4 x 64 bits
    const uint64_t * _c,      // 4 x 64 bits
    const uint64_t * _module, // 4 x 64 bits
          uint64_t * _d       // 4 x 64 bits
)
{
    // Fast path for the usual moduli
    if (Arith256ModFast(_a, _b, _c, _module, _d) == 0) return 0;

    // Convert input parameters to scalars
    mpz_class a, b, c, module;
    array2scalar(_a, a);
    array2scalar(_b, b);
    array2scalar(_c, c);
    array2scalar(_module, module);

    // Calculate the result as a scalar
    mpz_class d;
    d = ((a * b) + c) % module;

    // Convert scalar to output parameter
    scalar2array(d, _d);

    return 0;
}

int FastArith256(
    const uint64_t * _a,  // 4 x 64 bits (a)
    const uint64_t * _b,  // 4 x 64 bits (b)  
    const uint64_t * _c,  // 4 x 64 bits (c)
          uint64_t * _dl, // 4 x 64 bits (low result)
          uint64_t * _dh  // 4 x 64 bits (high result)
)
{
    // We will use schoolbook multiplication algorithm with 64-bit limbs
    // a = a[3]<<192 + a[2]<<128 + a[1]<<64 + a[0]
    // b = b[3]<<192 + b[2]<<128 + b[1]<<64 + b[0]
    // a*b = sum of all cross products a[i]*b[j] << (64*(i+j))
    
    uint64_t temp[8] = {0}; // temporary 512-bit result (8 limbs)
    
    asm volatile (
        // Initialize temp[8] = {0}
        "xor    %%rax, %%rax\n\t"
        "mov    %%rax, 0(%0)\n\t"   // temp[0] = 0
        "mov    %%rax, 8(%0)\n\t"   // temp[1] = 0
        "mov    %%rax, 16(%0)\n\t"  // temp[2] = 0
        "mov    %%rax, 24(%0)\n\t"  // temp[3] = 0
        "mov    %%rax, 32(%0)\n\t"  // temp[4] = 0
        "mov    %%rax, 40(%0)\n\t"  // temp[5] = 0
        "mov    %%rax, 48(%0)\n\t"  // temp[6] = 0
        "mov    %%rax, 56(%0)\n\t"  // temp[7] = 0
        
        // Multiplication a[0] * b[j] for j=0,1,2,3
        "mov    0(%1), %%rax\n\t"   // rax = a[0]
        
        "mulq   0(%2)\n\t"          // rdx:rax = a[0] * b[0]
        "add    %%rax, 0(%0)\n\t"   // temp[0] += rax
        "adc    %%rdx, 8(%0)\n\t"   // temp[1] += rdx + carry
        "adcq   $0, 16(%0)\n\t"     // temp[2] += carry
        
        "mov    0(%1), %%rax\n\t"   // rax = a[0]
        "mulq   8(%2)\n\t"          // rdx:rax = a[0] * b[1]
        "add    %%rax, 8(%0)\n\t"   // temp[1] += rax
        "adc    %%rdx, 16(%0)\n\t"  // temp[2] += rdx + carry
        "adcq   $0, 24(%0)\n\t"     // temp[3] += carry
        
        "mov    0(%1), %%rax\n\t"   // rax = a[0]
        "mulq   16(%2)\n\t"         // rdx:rax = a[0] * b[2]
        "add    %%rax, 16(%0)\n\t"  // temp[2] += rax
        "adc    %%rdx, 24(%0)\n\t"  // temp[3] += rdx + carry
        "adcq   $0, 32(%0)\n\t"     // temp[4] += carry
        
        "mov    0(%1), %%rax\n\t"   // rax = a[0]
        "mulq   24(%2)\n\t"         // rdx:rax = a[0] * b[3]
        "add    %%rax, 24(%0)\n\t"  // temp[3] += rax
        "adc    %%rdx, 32(%0)\n\t"  // temp[4] += rdx + carry
        "adcq   $0, 40(%0)\n\t"     // temp[5] += carry
        
        // Multiplication a[1] * b[j] for j=0,1,2,3
        "mov    8(%1), %%rax\n\t"   // rax = a[1]
        
        "mulq   0(%2)\n\t"          // rdx:rax = a[1] * b[0]
        "add    %%rax, 8(%0)\n\t"   // temp[1] += rax
        "adc    %%rdx, 16(%0)\n\t"  // temp[2] += rdx + carry
        "adcq   $0, 24(%0)\n\t"     // temp[3] += carry
        "adcq   $0, 32(%0)\n\t"     // temp[4] += carry
        
        "mov    8(%1), %%rax\n\t"   // rax = a[1]
        "mulq   8(%2)\n\t"          // rdx:rax = a[1] * b[1]
        "add    %%rax, 16(%0)\n\t"  // temp[2] += rax
        "adc    %%rdx, 24(%0)\n\t"  // temp[3] += rdx + carry
        "adcq   $0, 32(%0)\n\t"     // temp[4] += carry
        "adcq   $0, 40(%0)\n\t"     // temp[5] += carry
        
        "mov    8(%1), %%rax\n\t"   // rax = a[1]
        "mulq   16(%2)\n\t"         // rdx:rax = a[1] * b[2]
        "add    %%rax, 24(%0)\n\t"  // temp[3] += rax
        "adc    %%rdx, 32(%0)\n\t"  // temp[4] += rdx + carry
        "adcq   $0, 40(%0)\n\t"     // temp[5] += carry
        "adcq   $0, 48(%0)\n\t"     // temp[6] += carry
        
        "mov    8(%1), %%rax\n\t"   // rax = a[1]
        "mulq   24(%2)\n\t"         // rdx:rax = a[1] * b[3]
        "add    %%rax, 32(%0)\n\t"  // temp[4] += rax
        "adc    %%rdx, 40(%0)\n\t"  // temp[5] += rdx + carry
        "adcq   $0, 48(%0)\n\t"     // temp[6] += carry
        "adcq   $0, 56(%0)\n\t"     // temp[7] += carry
        
        // Multiplication a[2] * b[j] for j=0,1,2,3
        "mov    16(%1), %%rax\n\t"  // rax = a[2]
        
        "mulq   0(%2)\n\t"          // rdx:rax = a[2] * b[0]
        "add    %%rax, 16(%0)\n\t"  // temp[2] += rax
        "adc    %%rdx, 24(%0)\n\t"  // temp[3] += rdx + carry
        "adcq   $0, 32(%0)\n\t"     // temp[4] += carry
        "adcq   $0, 40(%0)\n\t"     // temp[5] += carry
        "adcq   $0, 48(%0)\n\t"     // temp[6] += carry
        
        "mov    16(%1), %%rax\n\t"  // rax = a[2]
        "mulq   8(%2)\n\t"          // rdx:rax = a[2] * b[1]
        "add    %%rax, 24(%0)\n\t"  // temp[3] += rax
        "adc    %%rdx, 32(%0)\n\t"  // temp[4] += rdx + carry
        "adcq   $0, 40(%0)\n\t"     // temp[5] += carry
        "adcq   $0, 48(%0)\n\t"     // temp[6] += carry
        "adcq   $0, 56(%0)\n\t"     // temp[7] += carry
        
        "mov    16(%1), %%rax\n\t"  // rax = a[2]
        "mulq   16(%2)\n\t"         // rdx:rax = a[2] * b[2]
        "add    %%rax, 32(%0)\n\t"  // temp[4] += rax
        "adc    %%rdx, 40(%0)\n\t"  // temp[5] += rdx + carry
        "adcq   $0, 48(%0)\n\t"     // temp[6] += carry
        "adcq   $0, 56(%0)\n\t"     // temp[7] += carry
        
        "mov    16(%1), %%rax\n\t"  // rax = a[2]
        "mulq   24(%2)\n\t"         // rdx:rax = a[2] * b[3]
        "add    %%rax, 40(%0)\n\t"  // temp[5] += rax
        "adc    %%rdx, 48(%0)\n\t"  // temp[6] += rdx + carry
        "adcq   $0, 56(%0)\n\t"     // temp[7] += carry
        
        // Multiplication a[3] * b[j] for j=0,1,2,3
        "mov    24(%1), %%rax\n\t"  // rax = a[3]
        
        "mulq   0(%2)\n\t"          // rdx:rax = a[3] * b[0]
        "add    %%rax, 24(%0)\n\t"  // temp[3] += rax
        "adc    %%rdx, 32(%0)\n\t"  // temp[4] += rdx + carry
        "adcq   $0, 40(%0)\n\t"     // temp[5] += carry
        "adcq   $0, 48(%0)\n\t"     // temp[6] += carry
        "adcq   $0, 56(%0)\n\t"     // temp[7] += carry
        
        "mov    24(%1), %%rax\n\t"  // rax = a[3]
        "mulq   8(%2)\n\t"          // rdx:rax = a[3] * b[1]
        "add    %%rax, 32(%0)\n\t"  // temp[4] += rax
        "adc    %%rdx, 40(%0)\n\t"  // temp[5] += rdx + carry
        "adcq   $0, 48(%0)\n\t"     // temp[6] += carry
        "adcq   $0, 56(%0)\n\t"     // temp[7] += carry
        
        "mov    24(%1), %%rax\n\t"  // rax = a[3]
        "mulq   16(%2)\n\t"         // rdx:rax = a[3] * b[2]
        "add    %%rax, 40(%0)\n\t"  // temp[5] += rax
        "adc    %%rdx, 48(%0)\n\t"  // temp[6] += rdx + carry
        "adcq   $0, 56(%0)\n\t"     // temp[7] += carry
        
        "mov    24(%1), %%rax\n\t"  // rax = a[3]
        "mulq   24(%2)\n\t"         // rdx:rax = a[3] * b[3]
        "add    %%rax, 48(%0)\n\t"  // temp[6] += rax
        "adc    %%rdx, 56(%0)\n\t"  // temp[7] += rdx + carry
        
        :
        : "r" (temp), "r" (_a), "r" (_b)
        : "rax", "rdx", "cc", "memory"
    );
    
    // Now add c to temp (512 bits + 256 bits)
    asm volatile (
        "mov    0(%1), %%rax\n\t"   // rax = c[0]
        "add    %%rax, 0(%0)\n\t"   // temp[0] += c[0]
        
        "mov    8(%1), %%rax\n\t"   // rax = c[1]
        "adc    %%rax, 8(%0)\n\t"   // temp[1] += c[1] + carry
        
        "mov    16(%1), %%rax\n\t"  // rax = c[2]
        "adc    %%rax, 16(%0)\n\t"  // temp[2] += c[2] + carry
        
        "mov    24(%1), %%rax\n\t"  // rax = c[3]
        "adc    %%rax, 24(%0)\n\t"  // temp[3] += c[3] + carry
        
        "adcq   $0, 32(%0)\n\t"     // temp[4] += carry
        "adcq   $0, 40(%0)\n\t"     // temp[5] += carry
        "adcq   $0, 48(%0)\n\t"     // temp[6] += carry
        "adcq   $0, 56(%0)\n\t"     // temp[7] += carry
        
        :
        : "r" (temp), "r" (_c)
        : "rax", "cc", "memory"
    );
    
    // Copy temp to dl (low part) and dh (high part)
    memcpy(_dl, &temp[0], 32);  // temp[0..3] -> dl
    memcpy(_dh, &temp[4], 32);  // temp[4..7] -> dh
    return 0;
}