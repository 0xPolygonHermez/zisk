#ifndef GOLDILOCKS_SCALAR
#define GOLDILOCKS_SCALAR
#include "goldilocks_base_field.hpp"

inline void Goldilocks::copy(Element &dst, const Element &src) { dst.fe = src.fe; };

inline void Goldilocks::copy(Element *dst, const Element *src) { dst->fe = src->fe; };

inline Goldilocks::Element Goldilocks::add(const Element &in1, const Element &in2)
{
    Goldilocks::Element result;
    Goldilocks::add(result, in1, in2);
    return result;
}

inline void Goldilocks::add(Element &result, const Element &in1, const Element &in2)
{
#if USE_ASSEMBLY == 1
   uint64_t in_1 = in1.fe;
   uint64_t in_2 = in2.fe;
   __asm__("xor   %%r10, %%r10\n\t"
           "mov   %1, %0\n\t"
           "add   %2, %0\n\t"
           "cmovc %3, %%r10\n\t"
           "add   %%r10, %0\n\t"
           "jnc  1f\n\t"
           "add   %3, %0\n\t"
           "1: \n\t"
           : "=&a"(result.fe)
           : "r"(in_1), "r"(in_2), "m"(CQ), "m"(ZR)
           : "%r10");
#else
   uint64_t in_1 = in1.fe;
   if(in_1 >= GOLDILOCKS_PRIME){
       in_1 -= GOLDILOCKS_PRIME;
   }
   result.fe = in_1 + in2.fe;
   if(in_1 > result.fe){
       result.fe -= GOLDILOCKS_PRIME;
   }
#endif
}

inline Goldilocks::Element Goldilocks::inc(const Goldilocks::Element &fe)
{
    Goldilocks::Element result;
    if (fe.fe < GOLDILOCKS_PRIME - 2)
    {
        result.fe = fe.fe + 1;
    }
    else if (fe.fe == GOLDILOCKS_PRIME - 1)
    {
        result.fe = 0;
    }
    else
    {
        result = Goldilocks::add(fe, Goldilocks::one());
    }
    return result;
}

inline Goldilocks::Element Goldilocks::sub(const Element &in1, const Element &in2)
{
    Goldilocks::Element result;
    Goldilocks::sub(result, in1, in2);
    return result;
}

inline void Goldilocks::sub(Element &result, const Element &in1, const Element &in2)
{
#if USE_ASSEMBLY == 1
    uint64_t in_1 = in1.fe;
    uint64_t in_2 = in2.fe;
    __asm__("xor   %%r10, %%r10\n\t"
            "mov   %1, %0\n\t"
            "sub   %2, %0\n\t"
            "cmovc %3, %%r10\n\t"
            "sub   %%r10, %0\n\t"
            "jnc  1f\n\t"
            "sub   %3, %0\n\t"
            "1: \n\t"
            : "=&a"(result.fe)
            : "r"(in_1), "r"(in_2), "m"(CQ), "m"(ZR)
            : "%r10");
#else
    uint64_t in_2 = in2.fe;
    if(in_2 >= GOLDILOCKS_PRIME){
        in_2 -= GOLDILOCKS_PRIME;
    }
    result.fe = in1.fe - in_2;
    if(in_2 > in1.fe){
        result.fe += GOLDILOCKS_PRIME;
    }
#endif
#if GOLDILOCKS_DEBUG == 1
    result.fe = result.fe % GOLDILOCKS_PRIME;
#endif
}

inline Goldilocks::Element Goldilocks::dec(const Goldilocks::Element &fe)
{
    Goldilocks::Element result;
    if (fe.fe > 0)
    {
        result.fe = fe.fe - 1;
    }
    else
    {
        result.fe = GOLDILOCKS_PRIME - 1;
    }
    return result;
}

inline Goldilocks::Element Goldilocks::mul(const Element &in1, const Element &in2)
{
    Goldilocks::Element result;
    Goldilocks::mul(result, in1, in2);
    return result;
}

/*
* Stable version used until new optimization based on branch_hint was introduced (see mul function)
*/
inline void Goldilocks::mul1(Element &result, const Element &in1, const Element &in2)
{

#if USE_ASSEMBLY == 1
    __asm__("mov   %1, %0\n\t"
            "mul   %2\n\t"
            // "xor   %%rbx, %%rbx\n\t"
            "mov   %%edx, %%ebx\n\t"
            "sub   %4, %%rbx\n\t"
            "rol   $32, %%rdx\n\t"
            //"xor   %%rcx, %%rcx;\n\t"
            "mov   %%edx, %%ecx\n\t"
            "sub   %%rcx, %%rdx\n\t"
            "add   %4, %%rcx\n\t"
            "sub   %%rbx, %%rdx\n\t"
            //"mov   %3,%%r10 \n\t"
            "xor   %%rbx, %%rbx\n\t"
            "add   %%rdx, %0\n\t"
            "cmovc %3, %%rbx\n\t"
            "add   %%rbx, %0\n\t"
            // TODO: migrate to labels
            //"xor   %%rbx, %%rbx\n\t"
            //"sub   %%rcx, %0\n\t"
            //"cmovc %%r10, %%rbx\n\t"
            //"sub   %%rbx, %0\n\t"
            "sub   %%rcx, %0\n\t"
            "jnc  1f\n\t"
            "sub   %3, %0\n\t"
            "1: \n\t"
            : "=&a"(result.fe)
            : "r"(in1.fe), "r"(in2.fe), "m"(CQ), "m"(TWO32)
            : "%rbx", "%rcx", "%rdx");

#if GOLDILOCKS_DEBUG == 1
    result.fe = result.fe % GOLDILOCKS_PRIME;
#endif
#else 
    mul(result, in1, in2);
#endif
}

inline void Goldilocks::mul2(Element &result, const Element &in1, const Element &in2)
{

#if USE_ASSEMBLY == 1
    __asm__(
        "mov   %1, %%rax\n\t"
        "mul   %2\n\t"
        "divq   %3\n\t"
        : "=&d"(result.fe)
        : "r"(in1.fe), "r"(in2.fe), "m"(Q)
        : "%rax");

#if GOLDILOCKS_DEBUG == 1
    result.fe = result.fe % GOLDILOCKS_PRIME;
#endif
#else
    mul(result, in1, in2);
#endif
}

inline void branch_hint() {
        asm("nop"); 
}
inline void Goldilocks::add_no_double_carry(uint64_t &result, const uint64_t &in1, const uint64_t &in2)
{

#if USE_ASSEMBLY == 1
    __asm__("xor   %%r10, %%r10\n\t"
            "mov   %1, %0\n\t"
            "add   %2, %0\n\t"
            "cmovc %3, %%r10\n\t"
            "add   %%r10, %0\n\t"
            : "=&a"(result)
            : "r"(in1), "r"(in2), "m"(CQ)
            : "%r10");
#endif
}
/**
 * Optimized version inspired in Plonky3 optimizations, using branch_hint hint the processor that the branch is unlikely to be taken
 */

inline void Goldilocks::mul(Element &result, const Element &in1, const Element &in2){

   
    uint64_t rh;
    uint64_t rl;    
   
    __uint128_t res = static_cast<__uint128_t>(in1.fe) * static_cast<__uint128_t>(in2.fe);
    rl = (uint64_t)res;
    rh = (uint64_t)(res>>64);   
    uint64_t rhh = rh >> 32;
    uint64_t rhl = rh & 0xFFFFFFFF;
    
    uint64_t aux1;
    aux1 = rl - rhh;
    if(rhh>rl){ //this branch is unlikely to be taken
        branch_hint(); 
        aux1-=0xFFFFFFFF;
    }
    uint64_t aux = 0xFFFFFFFF* rhl;
    // aux1 <= 2^64-1
    // aux <= (2^32-1)*(2^32-1) = 2^64-2^32+1-2^32 = P-2^32
    // aux1 + aux <= 2^64-1 + P-2^32 = P+P-2=2P-2
    #if USE_ASSEMBLY == 1   
        add_no_double_carry(result.fe, aux1, aux);
    #else
        Goldilocks::Element aux1_, aux2_;
        aux1_.fe = aux1;
        aux2_.fe = aux;
        add(result, aux2_, aux1_);
    #endif

}

inline Goldilocks::Element Goldilocks::square(const Element &in1) { return mul(in1, in1); };

inline void Goldilocks::square(Element &result, const Element &in1) { return mul(result, in1, in1); };

inline Goldilocks::Element Goldilocks::div(const Element &in1, const Element &in2) { return mul(in1, inv(in2)); };

inline void Goldilocks::div(Element &result, const Element &in1, const Element &in2) { mul(result, in1, inv(in2)); };

inline Goldilocks::Element Goldilocks::neg(const Element &in1) { return sub(Goldilocks::zero(), in1); };

inline void Goldilocks::neg(Element &result, const Element &in1) { return sub(result, Goldilocks::zero(), in1); };

inline bool Goldilocks::isZero(const Element &in1) { return equal(in1, Goldilocks::zero()); };

inline bool Goldilocks::isOne(const Element &in1) { return equal(in1, Goldilocks::one()); };

inline bool Goldilocks::isNegone(const Element &in1) { return equal(in1, Goldilocks::negone()); };

inline bool Goldilocks::equal(const Element &in1, const Element &in2) { return Goldilocks::toU64(in1) == Goldilocks::toU64(in2); }

inline Goldilocks::Element Goldilocks::inv(const Element &in1)
{
    Goldilocks::Element result;
    Goldilocks::inv(result, in1);
    return result;
};

inline Goldilocks::Element Goldilocks::mulScalar(const Element &base, const uint64_t &scalar)
{
    Goldilocks::Element result;
    Goldilocks::mulScalar(result, base, scalar);
    return result;
};
inline void Goldilocks::mulScalar(Element &result, const Element &base, const uint64_t &scalar)
{
    Element eScalar = fromU64(scalar);
    mul(result, base, eScalar);
};

inline Goldilocks::Element Goldilocks::exp(Element base, uint64_t exp)
{
    Goldilocks::Element result;
    Goldilocks::exp(result, base, exp);
    return result;
};

inline void Goldilocks::exp(Element &result, Element base, uint64_t exp)
{
    result = Goldilocks::one();

    for (;;)
    {
        if (exp & 1)
            mul(result, result, base);
        exp >>= 1;
        if (!exp)
            break;
        mul(base, base, base);
    }
};
#endif