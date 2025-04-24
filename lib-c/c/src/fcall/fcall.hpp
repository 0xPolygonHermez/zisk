#ifndef ARITH_HPP
#define ARITH_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Identifiers of the functions supported by free call
#define FCALL_ID_INVERSE_FP_EC 1
#define FCALL_ID_INVERSE_FN_EC 2
#define FCALL_ID_SQRT_FP_EC_PARITY 3
#define FCALL_ID_MSB_POS_256 4

// Fcall context
struct FcallContext
{
    uint64_t function_id; // identifies what function to call
    uint64_t params_max_size; // max length of input parameters array
    uint64_t params_size; // input parameters array valid data size
    uint64_t params[32]; // input parameters array
    uint64_t result_max_size; // max length of output result array
    uint64_t result_size; // output result array valid data size (written by fcall)
    uint64_t result[32]; // output result array (written by fcall)
};

// Fcall function; calls the corresponding function based on function identifier
int Fcall (
    struct FcallContext * ctx  // fcall context
);

// Functions supported by fcall, in fcall context format
int InverseFpEcCtx (
    struct FcallContext * ctx  // fcall context
);
int InverseFnEcCtx (
    struct FcallContext * ctx  // fcall context
);
int SqrtFpEcParityCtx (
    struct FcallContext * ctx  // fcall context
);
int MsbPos256Ctx (
    struct FcallContext * ctx  // fcall context
);

// Functions supported by fcall, in u64 array format
int InverseFpEc (
    const uint64_t * a, // 8 x 64 bits
          uint64_t * r  // 8 x 64 bits
);
int InverseFnEc (
    const uint64_t * a, // 8 x 64 bits
          uint64_t * r  // 8 x 64 bits
);
int SqrtFpEcParity (
    const uint64_t * a,  // 8 x 64 bits
    const uint64_t   parity,
          uint64_t * r  // 8 x 64 bits
);
int MsbPos256 (
    const uint64_t * a, // 8 x 64 bits
          uint64_t * r  // 2 x 64 bits
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
