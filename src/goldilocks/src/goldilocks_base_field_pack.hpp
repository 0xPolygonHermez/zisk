#ifndef GOLDILOCKS_PACK
#define GOLDILOCKS_PACK
#include "goldilocks_base_field.hpp"
#include <cassert>
/*
    Implementations for expressions:
*/
    
    inline void Goldilocks::copy_pack( uint64_t nrowsPack, Element *dst, const Element *src){
        for (uint64_t i = 0; i < nrowsPack; ++i)
        {
            dst[i].fe = src[i].fe;
        }
    }


    inline void Goldilocks::copy_pack( uint64_t nrowsPack, Element *dst, uint64_t stride_dst, const Element *src){
        for (uint64_t i = 0; i < nrowsPack; ++i)
        {
            dst[i*stride_dst].fe = src[i].fe;
        }
    }
  
    inline void Goldilocks::op_pack( uint64_t nrowsPack, uint64_t op, Element *c, const Element *a, const Element *b){

        switch (op)
        {
        case 0:
            for (uint64_t i = 0; i < nrowsPack; ++i)
            {
                add(c[i], a[i], b[i]);
            }
            break;
        case 1:
            for (uint64_t i = 0; i < nrowsPack; ++i)
            {
                sub(c[i], a[i], b[i]);
            }
            break;
        case 2:
            for (uint64_t i = 0; i < nrowsPack; ++i)
            {
                mul(c[i], a[i], b[i]);
            }
            break;
        case 3:
            for (uint64_t i = 0; i < nrowsPack; ++i)
            {
                sub(c[i], b[i], a[i]);
            }
            break;
        default:
            assert(0);
            break;
        }
    }

#endif