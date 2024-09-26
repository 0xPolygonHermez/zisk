#include "goldilocks_base_field.hpp"
#include "goldilocks_base_field_tools.hpp"
#include "goldilocks_base_field_scalar.hpp"

const Goldilocks::Element Goldilocks::ZR = {(uint64_t)0x0000000000000000LL};
const Goldilocks::Element Goldilocks::Q = {(uint64_t)0xFFFFFFFF00000001LL};
const Goldilocks::Element Goldilocks::MM = {(uint64_t)0xFFFFFFFeFFFFFFFFLL};
const Goldilocks::Element Goldilocks::CQ = {(uint64_t)0x00000000FFFFFFFFLL};
const Goldilocks::Element Goldilocks::R2 = {(uint64_t)0xFFFFFFFe00000001LL};

const Goldilocks::Element Goldilocks::W[33] = {
    Goldilocks::fromU64(0x1),
    Goldilocks::fromU64(18446744069414584320ULL),
    Goldilocks::fromU64(281474976710656ULL),
    Goldilocks::fromU64(16777216ULL),
    Goldilocks::fromU64(4096ULL),
    Goldilocks::fromU64(64ULL),
    Goldilocks::fromU64(8ULL),
    Goldilocks::fromU64(2198989700608ULL),
    Goldilocks::fromU64(4404853092538523347ULL),
    Goldilocks::fromU64(6434636298004421797ULL),
    Goldilocks::fromU64(4255134452441852017ULL),
    Goldilocks::fromU64(9113133275150391358ULL),
    Goldilocks::fromU64(4355325209153869931ULL),
    Goldilocks::fromU64(4308460244895131701ULL),
    Goldilocks::fromU64(7126024226993609386ULL),
    Goldilocks::fromU64(1873558160482552414ULL),
    Goldilocks::fromU64(8167150655112846419ULL),
    Goldilocks::fromU64(5718075921287398682ULL),
    Goldilocks::fromU64(3411401055030829696ULL),
    Goldilocks::fromU64(8982441859486529725ULL),
    Goldilocks::fromU64(1971462654193939361ULL),
    Goldilocks::fromU64(6553637399136210105ULL),
    Goldilocks::fromU64(8124823329697072476ULL),
    Goldilocks::fromU64(5936499541590631774ULL),
    Goldilocks::fromU64(2709866199236980323ULL),
    Goldilocks::fromU64(8877499657461974390ULL),
    Goldilocks::fromU64(3757607247483852735ULL),
    Goldilocks::fromU64(4969973714567017225ULL),
    Goldilocks::fromU64(2147253751702802259ULL),
    Goldilocks::fromU64(2530564950562219707ULL),
    Goldilocks::fromU64(1905180297017055339ULL),
    Goldilocks::fromU64(3524815499551269279ULL),
    Goldilocks::fromU64(7277203076849721926ULL)};

#if USE_MONTGOMERY == 0
const Goldilocks::Element Goldilocks::ONE = {(uint64_t)0x0000000000000001LL};
const Goldilocks::Element Goldilocks::ZERO = {(uint64_t)0x0000000000000000LL};
const Goldilocks::Element Goldilocks::NEGONE = {(uint64_t)0xFFFFFFFF00000000LL};
const Goldilocks::Element Goldilocks::TWO32 = {0x0000000100000000LL};
const Goldilocks::Element Goldilocks::SHIFT = Goldilocks::fromU64(7);

#else
const Goldilocks::Element Goldilocks::ONE = {(uint64_t)0x00000000FFFFFFFFLL};
const Goldilocks::Element Goldilocks::ZERO = {(uint64_t)0x0000000000000000LL};
const Goldilocks::Element Goldilocks::NEGONE = {(uint64_t)0XFFFFFFFE00000002LL};
const Goldilocks::Element Goldilocks::SHIFT = Goldilocks::fromU64(7);

#endif

/*
    Scalar operations
*/
void Goldilocks::parcpy(Element *dst, const Element *src, uint64_t size, int num_threads_copy)
{
    if (num_threads_copy < 1)
    {
        num_threads_copy = 1;
    }
    uint64_t components_thread = (size + num_threads_copy - 1) / num_threads_copy;

#pragma omp parallel for num_threads(num_threads_copy)
    for (uint64_t i = 0; i < size; i += components_thread)
    {
        uint64_t dim_ = components_thread * sizeof(Goldilocks::Element);
        if (size - i < components_thread)
        {
            dim_ = (size - i) * sizeof(Goldilocks::Element);
        }
        std::memcpy(&dst[i], &src[i], dim_);
    }
}

void Goldilocks::parSetZero(Element *dst, uint64_t size, int num_threads_copy)
{

    if (num_threads_copy < 1)
    {
        num_threads_copy = 1;
    }
    uint64_t components_thread = (size + num_threads_copy - 1) / num_threads_copy;

#pragma omp parallel for num_threads(num_threads_copy)
    for (uint64_t i = 0; i < size; i += components_thread)
    {
        uint64_t dim_ = components_thread * sizeof(Goldilocks::Element);
        if (size - i < components_thread)
        {
            dim_ = (size - i) * sizeof(Goldilocks::Element);
        }
        std::memset(&dst[i], 0, dim_);
    }
}

// TODO: Review and optimize inv imlementation
void Goldilocks::inv(Element &result, const Element &in1)
{
    if (Goldilocks::isZero(in1))
    {
        std::cerr << "Error: Goldilocks::inv called with zero" << std::endl;
        exit(-1);
    }
    u_int64_t t = 0;
    u_int64_t r = GOLDILOCKS_PRIME;
    u_int64_t newt = 1;

    u_int64_t newr = Goldilocks::toU64(in1);
    Element q;
    Element aux1;
    Element aux2;
    while (newr != 0)
    {
        q = Goldilocks::fromU64(r / newr);
        aux1 = Goldilocks::fromU64(t);
        aux2 = Goldilocks::fromU64(newt);
        t = Goldilocks::toU64(aux2);
        newt = Goldilocks::toU64(Goldilocks::sub(aux1, Goldilocks::mul(q, aux2)));
        aux1 = Goldilocks::fromU64(r);
        aux2 = Goldilocks::fromU64(newr);
        r = Goldilocks::toU64(aux2);
        newr = Goldilocks::toU64(Goldilocks::sub(aux1, Goldilocks::mul(q, aux2)));
    }

    Goldilocks::fromU64(result, t);
#if GOLDILOCKS_DEBUG == 1 && USE_MONTGOMERY == 0
    result.fe = result.fe % GOLDILOCKS_PRIME;
#endif
};