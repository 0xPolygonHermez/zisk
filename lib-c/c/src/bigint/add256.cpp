#include "add256.hpp"
#include "../common/utils.hpp"

int Add256 (
    const uint64_t * _a,  // 4 x 64 bits
    const uint64_t * _b,  // 4 x 64 bits
    const uint64_t cin,
          uint64_t * _c   // 4 x 64 bits
)
{
    uint8_t carry;
    
    asm volatile (
        "bt     $0, %4\n\t"          // set carry flag from cin bit 0
        
        "mov    (%1), %%rax\n\t"     // load a[0]
        "adc    (%2), %%rax\n\t"     // add b[0] + carry
        "mov    %%rax, (%3)\n\t"     // store c[0]
        
        "mov    8(%1), %%rax\n\t"    // load a[1]
        "adc    8(%2), %%rax\n\t"    // add b[1] + carry
        "mov    %%rax, 8(%3)\n\t"    // store c[1]
        
        "mov    16(%1), %%rax\n\t"   // load a[2]
        "adc    16(%2), %%rax\n\t"   // add b[2] + carry
        "mov    %%rax, 16(%3)\n\t"   // store c[2]
        
        "mov    24(%1), %%rax\n\t"   // load a[3]
        "adc    24(%2), %%rax\n\t"   // add b[3] + carry
        "mov    %%rax, 24(%3)\n\t"   // store c[3]
        
        "setc   %0\n\t"              // carry flag -> result
        : "=r" (carry)
        : "r" (_a), "r" (_b), "r" (_c), "r" (cin)
        : "rax", "cc", "memory"
    );
    
    return carry;
}