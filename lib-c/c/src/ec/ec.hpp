#ifndef EC_HPP
#define EC_HPP

#ifdef __cplusplus
extern "C" {
#endif

int AddPointEc (
    unsigned long dbl,
    const unsigned long * x1,  // 4 x 64 bits
    const unsigned long * y1,  // 4 x 64 bits
    const unsigned long * x2,  // 4 x 64 bits
    const unsigned long * y2,  // 4 x 64 bits
    unsigned long * x3,  // 4 x 64 bits
    unsigned long * y3  // 4 x 64 bits
);

int AddPointEcP (
    const unsigned long dbl,
    const unsigned long * p1,  // 8 x 64 bits
    const unsigned long * p2,  // 8 x 64 bits
    unsigned long * p3  // 8 x 64 bits
);

int InverseFpEc (
    const unsigned long * a,  // 8 x 64 bits
    unsigned long * r  // 8 x 64 bits
);

int InverseFnEc (
    const unsigned long * a,  // 8 x 64 bits
    unsigned long * r  // 8 x 64 bits
);

int SqrtFpEcParity (
    const unsigned long * a,  // 8 x 64 bits
    const unsigned long parity,
    unsigned long * r  // 8 x 64 bits
);

#define FCALL_ID_INVERSE_FP_EC 1
#define FCALL_ID_INVERSE_FN_EC 2
#define FCALL_ID_SQRT_FP_EC_PARITY 3

struct FcallContext
{
    unsigned long function_id; // identifies what function to call
    unsigned long params_max_size; // max length of input parameters array
    unsigned long params_size; // input parameters array valid data size
    unsigned long params[32]; // input parameters array
    unsigned long result_max_size; // max length of output result array
    unsigned long result_size; // output result array valid data size (written by fcall)
    unsigned long result[32]; // output result array (written by fcall)
};

int Fcall (
    struct FcallContext * ctx  // fcall context
);

int InverseFpEcCtx (
    struct FcallContext * ctx  // fcall context
);

int InverseFnEcCtx (
    struct FcallContext * ctx  // fcall context
);

int SqrtFpEcParityCtx (
    struct FcallContext * ctx  // fcall context
);

int Arith256 (
    const unsigned long * a,  // 4 x 64 bits
    const unsigned long * b,  // 4 x 64 bits
    const unsigned long * c,  // 4 x 64 bits
    unsigned long * dl, // 4 x 64 bits
    unsigned long * dh // 4 x 64 bits
);

int Arith256Mod (
    const unsigned long * a,  // 4 x 64 bits
    const unsigned long * b,  // 4 x 64 bits
    const unsigned long * c,  // 4 x 64 bits
    const unsigned long * module,  // 4 x 64 bits
    unsigned long * d // 4 x 64 bits
);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
