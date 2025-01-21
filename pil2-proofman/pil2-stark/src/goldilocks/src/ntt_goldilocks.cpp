#include "ntt_goldilocks.hpp"

//Explicar extend parameter
//Explicar inverse parameter
//Extension parameter

static inline u_int64_t BR(u_int64_t x, u_int64_t domainPow)
{
    x = (x >> 16) | (x << 16);                              //swaps 32bit halves of x
    x = ((x & 0xFF00FF00) >> 8) | ((x & 0x00FF00FF) << 8);  //swaps 16bit halves of 32bit halves
    x = ((x & 0xF0F0F0F0) >> 4) | ((x & 0x0F0F0F0F) << 4);  //swaps 8bit halves of 16bit halves
    x = ((x & 0xCCCCCCCC) >> 2) | ((x & 0x33333333) << 2);  //swaps 4bit halves of 8bit halves
    return (((x & 0xAAAAAAAA) >> 1) | ((x & 0x55555555) << 1)) >> (32 - domainPow); //swaps 2bit halves of 4bit halves
}

/**
 * @brief Iterations of the NTT algorithm
 * 
 * @param dst destination pointer
 * @param src source pointer
 * @param nrows number of rows (power of 2)
 * @param offset_cols offset of the first column considered
 * @param ncols number of columns considered
 * @param ncols_all total number of columns
 * @param nphase number of phases of the NTT
 * @param aux auxiliary buffer
 * @param inverse if true, computes the inverse NTT
 * @param extend if true, multiplies the result by r_ (adoc optimization for the LDE)
 * */
void NTT_Goldilocks::NTT_iters(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t nrows, u_int64_t offset_cols, u_int64_t ncols, u_int64_t ncols_all, u_int64_t nphase, Goldilocks::Element *aux, bool inverse, bool extend)
{
    Goldilocks::Element *dst_;
    if (dst != NULL)
    {
        dst_ = dst;
    }
    else
    {
        dst_ = src;
    }
    Goldilocks::Element *a = dst_;
    uint64_t strideA = ncols_all;
    uint64_t offsetA = offset_cols;
    Goldilocks::Element *a2 = aux;
    uint64_t strideA2 = ncols;
    uint64_t offsetA2 = 0;
    Goldilocks::Element *tmp;
    uint64_t strideTmp;
    uint64_t offsetTmp;

    u_int64_t domainPow = log2(nrows);
    assert(((u_int64_t)1 << domainPow) == nrows);
    if (nphase < 1 || domainPow == 0)
    {
        nphase = 1;
    }
    else if (nphase > domainPow)
    {
        nphase = domainPow;
    }
    u_int64_t maxBatchPow = s / nphase;
    u_int64_t res = s % nphase;
    if (res > 0)
    {
        maxBatchPow += 1;
    }
    bool iseven = true;
    tmp = a;
    strideTmp = strideA;
    offsetTmp = offsetA;
    
    if (nphase % 2 == 1)
    {
        iseven = false;
        tmp = a2;
        strideTmp = strideA2;
        offsetTmp = offsetA2;
    }
    reversePermutation(tmp, strideTmp, offsetTmp, src, ncols_all, offset_cols, nrows, ncols);
    if (iseven == false)
    {
        tmp = a2;
        strideTmp = strideA2;
        offsetTmp = offsetA2;
        a2 = a;
        strideA2 = strideA;
        offsetA2 = offsetA;
        a = tmp;
        strideA = strideTmp;
        offsetA = offsetTmp;
    }

    omp_set_dynamic(0);
    omp_set_num_threads(nThreads);
    uint64_t count = 1;
    for (u_int64_t s = 1; s <= domainPow; s += maxBatchPow, ++count)
    {
        if (res > 0 && count == res + 1 && maxBatchPow > 1)
        {
            maxBatchPow -= 1;
        }
        u_int64_t sInc = s + maxBatchPow <= domainPow ? maxBatchPow : domainPow - s + 1;
        u_int64_t rs = s - 1;
        u_int64_t re = domainPow - 1;
        u_int64_t rb = 1 << rs;
        u_int64_t rm = (1 << (re - rs)) - 1;
        u_int64_t batchSize = 1 << sInc;
        u_int64_t nBatches = nrows / batchSize;

        int chunk1 = nBatches / nThreads;
        if (chunk1 == 0)
        {
            chunk1 = 1;
        }

#pragma omp parallel for schedule(static, chunk1)
        for (u_int64_t b = 0; b < nBatches; b++)
        {
            for (u_int64_t si = 0; si < sInc; si++)
            {
                u_int64_t m = 1 << (s + si);
                u_int64_t mdiv2 = m >> 1;
                u_int64_t mdiv2i = 1 << si;
                u_int64_t mi = mdiv2i * 2;
                for (u_int64_t i = 0; i < (batchSize >> 1); i++)
                {
                    u_int64_t ki = b * batchSize + (i / mdiv2i) * mi;
                    u_int64_t ji = i % mdiv2i;

                    u_int64_t offset1 = (ki + ji + mdiv2i) * strideA + offsetA;
                    u_int64_t offset2 = (ki + ji) * strideA + offsetA;

                    u_int64_t j = (b * batchSize / 2 + i);
                    j = (j & rm) * rb + (j >> (re - rs));
                    j = j % mdiv2;

                    Goldilocks::Element w = root(s + si, j);
                    for (u_int64_t k = 0; k < ncols; ++k)
                    {
                        Goldilocks::Element t = w * a[offset1 + k];
                        Goldilocks::Element u = a[offset2 + k];

                        Goldilocks::add(a[offset2 + k], t, u);
                        Goldilocks::sub(a[offset1 + k], u, t);
                    }
                }
            }
            if (s + maxBatchPow <= domainPow || !inverse)
            {
                //case: any phase and not inverse
                for (u_int64_t x = 0; x < batchSize; x++)
                {
                    u_int64_t offset_a2 = (x * nBatches + b) * strideA2 + offsetA2;
                    u_int64_t offset_a = (b * batchSize + x) * strideA + offsetA;
                    std::memcpy(&a2[offset_a2], &a[offset_a], ncols * sizeof(Goldilocks::Element));
                }
            }
            else
            {
                if (extend)
                {
                    //case: last phase and extend
                    for (u_int64_t x = 0; x < batchSize; x++)
                    {
                        u_int64_t dsty = intt_idx((x * nBatches + b), nrows);
                        u_int64_t offset_a2 = dsty * strideA2 + offsetA2;
                        u_int64_t offset_a = (b * batchSize + x) * strideA + offsetA;
                        for (uint64_t k = 0; k < ncols; k++)
                        {
                            Goldilocks::mul(a2[offset_a2 + k], a[offset_a + k], r_[dsty]);
                        }
                    }
                }
                else 
                {
                    //case: last phase and inverse
                    assert(inverse);
                    for (u_int64_t x = 0; x < batchSize; x++)
                    {
                        u_int64_t dsty = intt_idx((x * nBatches + b), nrows);
                        u_int64_t offset_a2 = dsty * strideA2 + offsetA2;
                        u_int64_t offset_a = (b * batchSize + x) * strideA + offsetA;
                        for (uint64_t k = 0; k < ncols; k++)
                        {
                            Goldilocks::mul(a2[offset_a2 + k], a[offset_a + k], powTwoInv[domainPow]);
                        }
                    }
                }
            }
        }
        tmp = a2;
        strideTmp = strideA2;
        offsetTmp = offsetA2;
        a2 = a;
        strideA2 = strideA;
        offsetA2 = offsetA;
        a = tmp;
        strideA = strideTmp;
        offsetA = offsetTmp;
    }
    if (a != dst_)
    {
        if (nrows > 1)
        {
            assert(0); // should never need this copy...
        }
        Goldilocks::parcpy(dst_, a, nrows * ncols, nThreads);
    }
}

void NTT_Goldilocks::NTT(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t ncols, Goldilocks::Element *buffer, u_int64_t nphase, u_int64_t nblock, bool inverse, bool extend)
{
    if (ncols == 0 || size == 0)
    {
        return;
    }
    if (nblock < 1)
    {
        nblock = 1;
    }
    if (nblock > ncols)
    {
        nblock = ncols;
    }

    u_int64_t offset_cols = 0;
    u_int64_t ncols_block = ncols / nblock;
    u_int64_t ncols_res = ncols % nblock;
    u_int64_t ncols_alloc = ncols_block;
    if (ncols_res > 0)
    {
        ncols_alloc += 1;
    }
    Goldilocks::Element *aux = NULL;
    if (buffer == NULL)
    {
        aux = (Goldilocks::Element *)malloc(sizeof(Goldilocks::Element) * size * ncols_alloc);
        if(aux == NULL){
            std::cerr << "Error: NTT_Goldilocks::NTT: Memory allocation failed" << std::endl;   
            exit(1);
        }
    }
    else
    {
        aux = buffer;
    }
    
    for (u_int64_t ib = 0; ib < nblock; ++ib)
    {
        uint64_t aux_ncols = ncols_block;
        if (ib < ncols_res)
            aux_ncols += 1;
        NTT_Goldilocks::NTT_iters(dst, src, size, offset_cols, aux_ncols, ncols, nphase, aux, inverse, extend);
        offset_cols += aux_ncols;
    }
    if (buffer == NULL)
    {
        free(aux);
    }
}
/**
 * @brief permutation of components of an array in bit-reversal order. If dst==src the permutation is performed on-site.
 *
 * @param dst destination pointer (may be equal to src)
 * @param strideDst stride between consecutive elements of the same column in dst array
 * @param offsetDst offset of the first element of the first column in dst array
 * @param src source pointer
 * @param strideSrc stride between consecutive elements of the same column in src array
 * @param offsetSrc offset of the first element of the first column in src array
 * @param nrows number rows
 * @param ncols number of columns being permuted
 */
void NTT_Goldilocks::reversePermutation(Goldilocks::Element *dst, uint64_t strideDst, uint64_t offsetDst,  Goldilocks::Element *src, uint64_t strideSrc, uint64_t offsetSrc, u_int64_t nrows, uint64_t ncols)
{
    uint32_t domainSize = log2(nrows);
    if (dst != src)
    {
        if (extension <= 1)
        {
#pragma omp parallel for schedule(static)
            for (u_int64_t i = 0; i < nrows; i++)
            {
                u_int64_t r = BR(i, domainSize);
                u_int64_t offset_r1 = r * strideSrc + offsetSrc;
                u_int64_t offset_i1 = i * strideDst + offsetDst;
                std::memcpy(&dst[offset_i1], &src[offset_r1], ncols * sizeof(Goldilocks::Element));
            }
        }
        else
        {
            //When the source is suposed to be an extension of a vector of size; nrows/extension, then we know that the source is zero from de component nrows/extension to nrows
            u_int64_t ext_rows = nrows / extension;

#pragma omp parallel for schedule(static)
            for (u_int64_t i = 0; i < nrows; i++)
            {
                u_int64_t r = BR(i, domainSize);
                u_int64_t offset_r1 = r * strideSrc + offsetSrc;
                u_int64_t offset_i1 = i * strideDst + offsetDst;
                if (r < ext_rows)
                {
                    std ::memcpy(&dst[offset_i1], &src[offset_r1], ncols * sizeof(Goldilocks::Element));
                }
                else
                {
                    std::memset(&dst[offset_i1], 0, ncols * sizeof(Goldilocks::Element));
                }
            }
        }
    }
    else
    {
        if (extension <= 1)
        {
#pragma omp parallel for schedule(static)
            for (u_int64_t i = 0; i < nrows; i++)
            {
                u_int64_t r = BR(i, domainSize);
                u_int64_t offset_r = r * strideSrc + offsetSrc;
                u_int64_t offset_i = i * strideDst + offsetDst;
                if (r < i)
                {
                    Goldilocks::Element tmp[ncols];
                    std::memcpy(&tmp[0], &src[offset_r], ncols * sizeof(Goldilocks::Element));
                    std::memcpy(&dst[offset_r], &src[offset_i], ncols * sizeof(Goldilocks::Element));
                    std::memcpy(&dst[offset_i], &tmp[0], ncols * sizeof(Goldilocks::Element));
                }
            }
        }
        else
        {
            //When the source is suposed to be an extension of a vector of size; nrows/extension, then we know that the source is zero from de component nrows/extension to nrows
            u_int64_t ext_rows = nrows / extension;

#pragma omp parallel for schedule(static)
            for (u_int64_t i = 0; i < nrows; i++)
            {
                u_int64_t r = BR(i, domainSize);
                u_int64_t offset_r = r * strideSrc + offsetSrc;
                u_int64_t offset_i = i * strideDst + offsetDst;
                if (r < ext_rows)
                {
                    Goldilocks::Element tmp[ncols];
                    std::memcpy(&tmp[0], &src[offset_r], ncols * sizeof(Goldilocks::Element));
                    std::memcpy(&dst[offset_r], &src[offset_i], ncols * sizeof(Goldilocks::Element));
                    std::memcpy(&dst[offset_i], &tmp[0], ncols * sizeof(Goldilocks::Element));
                }
                else
                {
                    std::memset(&dst[offset_i], 0, ncols * sizeof(Goldilocks::Element));
                }
            }

        }
    }
}

void NTT_Goldilocks::extendPol(Goldilocks::Element *output, Goldilocks::Element *input, uint64_t N_Extended, uint64_t N, uint64_t ncols, Goldilocks::Element *buffer, u_int64_t nphase, u_int64_t nblock)
{
    if (N == 0 || ncols == 0) {
        return;
    }

#ifdef __USE_CUDA__
    uint64_t nPack;
    if (ncols < 16) {
        return extendPol_GPU(output, input, N_Extended, N, ncols);
    } else if (ncols < 32) {
        nPack = 4;
    } else if (ncols < 80) {
        nPack = 8;
    } else {
        nPack = 16;
    }
    return extendPol_MultiGPU(output, input, N_Extended, N, ncols, NULL, nPack);
#else
    NTT_Goldilocks ntt_extension(N_Extended, nThreads, N_Extended / N);

    Goldilocks::Element *tmp = NULL;
    if (buffer == NULL)
    {
        tmp = (Goldilocks::Element *)malloc(N_Extended * ncols * sizeof(Goldilocks::Element));
        if(tmp == NULL){
            std::cerr << "Error: NTT_Goldilocks::extendPol: Memory allocation failed" << std::endl;   
            exit(1);
        }
    }
    else
    {
        tmp = buffer;
    }
    // TODO: Pre-compute r
    if (r == NULL)
    {
        computeR(N);
    }

    INTT(output, input, N, ncols, tmp, nphase, nblock, true);
    ntt_extension.NTT(output, output, N_Extended, ncols, tmp, nphase, nblock);

    if (buffer == NULL)
    {
        free(tmp);
    }
#endif
}
