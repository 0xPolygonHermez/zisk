#include "float/float.h"

#ifdef __cplusplus
extern "C" {
#endif

int _zisk_main(int argc, char *argv[])
{
    // f3 = f1 + f2
    fregs[1] = F64_ONE;
    fregs[2] = F64_ONE;
    uint64_t inst = 0x022081D3; // fadd.d f1, f2, f3
    *(uint64_t *)FREG_INST = inst;
    _zisk_float();
    return 0;
}

#ifdef __cplusplus
} // extern "C"
#endif