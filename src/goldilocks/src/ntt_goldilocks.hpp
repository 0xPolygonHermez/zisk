#ifndef NTT_GOLDILOCKS
#define NTT_GOLDILOCKS

#include "goldilocks_base_field.hpp"
#include <cassert>
#include <gmp.h>
#include <omp.h>

#define NUM_PHASES 3
#define NUM_BLOCKS 1

class NTT_Goldilocks
{
private:
    u_int32_t s = 0;
    u_int32_t nThreads;
    uint64_t nqr;
    Goldilocks::Element *roots;
    Goldilocks::Element *powTwoInv;
    Goldilocks::Element *r;
    Goldilocks::Element *r_; //contains r_i * powTwoInv[domainPow] 
    int extension; //used to indicate the size of the extension (adoc optimization for the LDE)

    static u_int32_t log2(u_int64_t size)
    {
        {
            assert(size != 0);
            u_int32_t res = 0;
            while (size != 1)
            {
                size >>= 1;
                res++;
            }
            return res;
        }
    }
    void NTT_iters(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t offset_cols, u_int64_t ncols, u_int64_t ncols_all, u_int64_t nphase, Goldilocks::Element *aux, bool inverse, bool extend);
    inline int intt_idx(int i, int N)
    {
        return i == 0 ? 0 : N - i;
    }
    inline Goldilocks::Element &root(u_int32_t domainPow, u_int64_t idx)
    {
        return roots[idx << (s - domainPow)];
    }
    inline void computeR(int N)
    {
        u_int64_t domainPow = log2(N);
        r = new Goldilocks::Element[N];
        r_ = new Goldilocks::Element[N];
        r[0] = Goldilocks::one();
        r_[0] = powTwoInv[domainPow];
        for (int i = 1; i < N; i++)
        {
            Goldilocks::mul(r[i], r[i - 1], Goldilocks::shift());
            Goldilocks::mul(r_[i], r[i], powTwoInv[domainPow]);
        }
    }
    void reversePermutation(Goldilocks::Element *dst, uint64_t strideDst, uint64_t offsetDst,  Goldilocks::Element *src, uint64_t strideSrc, uint64_t offsetSrc, u_int64_t size, uint64_t ncols);

public:

    NTT_Goldilocks(u_int64_t maxDomainSize, u_int32_t _nThreads = 0, int extension_ = 1)
    {

        r = NULL;
        r_ = NULL;
        if (maxDomainSize == 0)
            return;
        nThreads = _nThreads == 0 ? omp_get_max_threads() : _nThreads;
        extension = extension_;

        u_int32_t domainPow = NTT_Goldilocks::log2(maxDomainSize);

        mpz_t m_qm1d2;
        mpz_t m_q;
        mpz_t m_nqr;
        mpz_t m_aux;
        mpz_init(m_qm1d2);
        mpz_init(m_q);
        mpz_init(m_nqr);
        mpz_init(m_aux);

        u_int64_t negone = GOLDILOCKS_PRIME - 1;

        mpz_import(m_aux, 1, 1, sizeof(u_int64_t), 0, 0, &negone);
        mpz_add_ui(m_q, m_aux, 1);
        mpz_fdiv_q_2exp(m_qm1d2, m_aux, 1);

        mpz_set_ui(m_nqr, 2);
        mpz_powm(m_aux, m_nqr, m_qm1d2, m_q);
        while (mpz_cmp_ui(m_aux, 1) == 0)
        {
            mpz_add_ui(m_nqr, m_nqr, 1);
            mpz_powm(m_aux, m_nqr, m_qm1d2, m_q);
        }

        s = 1;
        mpz_set(m_aux, m_qm1d2);
        while ((!mpz_tstbit(m_aux, 0)) && (s < domainPow))
        {
            mpz_fdiv_q_2exp(m_aux, m_aux, 1);
            s++;
        }

        nqr = mpz_get_ui(m_nqr);

        if (s < domainPow)
        {
            throw std::range_error("Domain size too big for the curve");
        }

        uint64_t nRoots = 1LL << s;

        roots = (Goldilocks::Element *)malloc(nRoots * sizeof(Goldilocks::Element));
        if(roots == NULL){
            std::cerr << "Error allocating memory for roots" << std::endl;
            exit(1);
        }
        powTwoInv = (Goldilocks::Element *)malloc((s + 1) * sizeof(Goldilocks::Element));
        if(powTwoInv == NULL){
            std::cerr << "Error allocating memory for powTwoInv" << std::endl;
            exit(1);
        }

        roots[0] = Goldilocks::one();
        powTwoInv[0] = Goldilocks::one();

        if (nRoots > 1)
        {
            // calculate the first root of unity
            // mpz_powm(m_aux, m_nqr, m_aux, m_q);
            // roots[1] = Goldilocks::fromU64(mpz_get_ui(m_aux));
            roots[1] = Goldilocks::w(domainPow);

            // modular inverse of 2
            mpz_set_ui(m_aux, 2);
            mpz_invert(m_aux, m_aux, m_q);
            powTwoInv[1] = Goldilocks::fromU64(mpz_get_ui(m_aux));
        }

        // calculate the rest of roots of unity
        for (uint64_t i = 2; i < nRoots; i++)
        {
            roots[i] = roots[i - 1] * roots[1];
        }
        Goldilocks::Element aux = roots[nRoots - 1] * roots[1];
        assert(Goldilocks::toU64(aux) == 1);
        for (uint64_t i = 2; i <= s; i++)
        {
            powTwoInv[i] = powTwoInv[i - 1] * powTwoInv[1];
        }

        mpz_clear(m_qm1d2);
        mpz_clear(m_q);
        mpz_clear(m_nqr);
        mpz_clear(m_aux);
    };
    ~NTT_Goldilocks()
    {
        if (s != 0)
        {
            free(roots);
            free(powTwoInv);
        }
        if (r != NULL)
        {
            delete (r);
        }
        if (r_ != NULL)
        {
            delete (r_);
        }
    }
    
    void NTT(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t ncols = 1, Goldilocks::Element *buffer = NULL, u_int64_t nphase = NUM_PHASES, u_int64_t nblock = NUM_BLOCKS, bool inverse = false, bool extend = false);
    inline void INTT(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t ncols = 1, Goldilocks::Element *buffer = NULL, u_int64_t nphase = NUM_PHASES, u_int64_t nblock = NUM_BLOCKS, bool extend = false);    
    void extendPol(Goldilocks::Element *output, Goldilocks::Element *input, uint64_t N_Extended, uint64_t N, uint64_t ncols, Goldilocks::Element *buffer = NULL, u_int64_t nphase = NUM_PHASES, u_int64_t nblock = NUM_BLOCKS);
};
// extend parameter is used to indicate tha the polinomial will be extended after the INTT
void NTT_Goldilocks::INTT(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t ncols, Goldilocks::Element *buffer, u_int64_t nphase, u_int64_t nblock, bool extend)
{
    NTT(dst, src, size, ncols, buffer, nphase, nblock, true, extend);
}

#endif
