/* unaligned_out_le_inline.c -- unaligned_out.c with zkvm_u256_le.h in its
   ZKVM_U256_LE_INLINE mode, where add/sub/mul/addmod/mulmod are C definitions
   that call the precompiles themselves and must route an unaligned result
   through an aligned temporary. Same output, so the same golden snapshot. */
#define ZKVM_U256_LE_INLINE
#include "unaligned_out.c"
