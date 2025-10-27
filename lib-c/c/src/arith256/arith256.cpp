#include "arith256.hpp"
#include "../common/utils.hpp"

int Arith256 (
    const uint64_t * _a,  // 4 x 64 bits
    const uint64_t * _b,  // 4 x 64 bits
    const uint64_t * _c,  // 4 x 64 bits
          uint64_t * _dl, // 4 x 64 bits
          uint64_t * _dh  // 4 x 64 bits
)
{
    // Convert input parameters to scalars
    mpz_class a, b, c;
    array2scalar(_a, a);
    array2scalar(_b, b);
    array2scalar(_c, c);

    // Calculate the result as a scalar
    mpz_class d;
    d = (a * b) + c;

    // Decompose d = dl + dh<<256 (dh = d)
    mpz_class dl;
    dl = d & ScalarMask256;
    d >>= 256;

    // Convert scalars to output parameters
    scalar2array(dl, _dl);
    scalar2array(d, _dh);

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