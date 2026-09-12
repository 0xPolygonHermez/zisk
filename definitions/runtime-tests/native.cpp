#include "../../emulator-asm/src/koala_poseidon2.hpp"

extern "C" bool koala_native_test(uint64_t *state) {
    return koala_poseidon2_packed(state);
}
