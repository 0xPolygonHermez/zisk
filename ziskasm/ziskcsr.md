# ZisK zkVM ABI: calling accelerators and U256 through CSR sequences

## Scope

A ZisK guest reaches every EF accelerator and U256 function with a CSR instruction sequence that the transpiler recognises and rewrites; no C marshalling layer runs. This spec gives the exact instruction sequences, for guest authors writing C, C++ or assembly against `zkvm_accelerators.h`, `zkvm_u256.h` and `zkvm_u256_le.h`.

There are two call forms:

- **Inline.** The CSR sequence is emitted at the call site, with no call. Used where the whole function is one precompile or one marker: `zkvm_keccak_f1600`, the memory functions in `zkvm_mem.h`, the JUMPDEST bitmap, and short U256 functions.
- **Thunk (non-inline).** A normal function call into a two-instruction stub, `csrs <id>, x0; ret`. The transpiler turns the `csrs` into a jump to a hand-written `ziskasm_zkvm_*` routine in the ZisK library. Used by every function that does real work: hashes, curves, pairings, modexp, U256 division and exp.

The choice is a trade-off between size and overhead. Inline suits short functions: a call costs no argument moves, call or return, and the extra ROM per call site is small. Thunk suits long functions: one shared copy of the routine serves every call site, so a long body is not duplicated in ROM, and the call overhead is small next to the work.

The sequences only have meaning under the ZisK transpiler. On any other RISC-V target a `csrs` to these addresses is an ordinary CSR write and the call computes nothing.

## Inline form

An inline function emits its CSR sequence at the call site, with no call and no `ra`. There are three kinds.

### Inline zkvmcall

One `csrs` per pointer argument. Argument 0 rides on the function's own CSR; argument k (k ≥ 1) rides on `0x8E0 + k − 1`.

```asm
csrs 0x888, rA          # zkvm_u256_le_mul: argument 0 = a
csrs 0x8E0, rB          # argument 1 = b
csrs 0x8E1, rR          # argument 2 = result
```

The transpiler replaces the whole sequence with the `.zisk` routine's body, reading the named registers where the routine reads `r10`, `r11`, …: no argument moves, no call, no return. The body writes only memory and the virtual registers `r32`..`r39`, never a RISC-V register, so the asm declares no register clobbers. The status is the constant `ZKVM_EOK`, which the compiler folds away.

To the guest program, an inline zkvmcall behaves like one more instruction: it reads its operands, writes its result, and leaves everything else as it was. The inlined code has its own resources, separate from the program's:

- **ROM.** Only the body's first instruction takes the address of the `csrs`. The rest are placed at internal (odd) ROM addresses that RISC-V code never uses, so the program's own code layout is unchanged. Read-only constants live in the ZisK library ROM (`ZISKLIB_ROM_ADDR`, 1 MB).
- **RAM.** Scratch variables live in the ZisK library RAM (`ZISKLIB_RAM_ADDR`, 192 KB). The guest linker scripts reserve that range, so the guest's own data and heap never reach it.
- **Registers.** The body uses only `r32`..`r39`. These are main-trace registers that RISC-V code cannot name, so no guest register is spilled, saved or clobbered.

The only memory the program sees change is the result buffer.

From the program's point of view, the whole sequence is one instruction as long as the sequence itself. In the example above, the three 4-byte `csrs` behave as a single 12-byte instruction, and execution continues at the instruction after the third `csrs`:

```text
0x80001000  csrs 0x888, rA   ->  body instruction 1 (at the RISC-V address)
                                 body instructions 2..n at internal addresses,
                                 the last one jumping to 0x8000100c
0x80001004  csrs 0x8E0, rB   ->  never executed
0x80001008  csrs 0x8E1, rR   ->  never executed
0x8000100c  next instruction <-  execution continues here
```

The transpiler places the body's first ZisK instruction at the address of the first `csrs`. The other body instructions go to internal addresses: odd ROM addresses, which RISC-V code never uses, so they interleave with the program's code without displacing it. The body's exits jump to the address after the last `csrs`. The addresses of the second and later `csrs` are never reached. If the program jumps to one of them, the execution halts with an error.

### Precompile

One instruction; the register holds a pointer.

```asm
csrs 0x800, rS          # keccak-f[1600] on the 25-word state at rS, in place
```

### Marker pair

A `csrs` (or `csrrs`) followed immediately by an `add` or `addi` with `rd = x0`. The transpiler folds the pair into one DMA or EVM operation. A size that is a constant up to 2047 rides as the `addi` immediate and makes the operation one ZisK instruction; a size in a register costs one more.

| Operation | Constant size | Size in a register |
| --- | --- | --- |
| memcpy(dst, src, n) | `csrs 0x813, src` / `addi x0, dst, n` | `csrs 0x813, src` / `add x0, dst, rN` |
| memcmp(a, b, n) → rd | `csrrs rd, 0x814, b` / `addi x0, a, n` | `csrrs rd, 0x814, b` / `add x0, a, rN` |
| memset(dst, c, n) | `csrsi 0x816, 2` / `addi x0, dst, n` / `addi x0, dst, c` | `csrs 0x816, dst` / `addi x0, rN, c` |
| jumpdest bitmap(code, n, bitmap) | — | `csrs 0x81C, code` / `add x0, bitmap, rN` |

The memset fill byte must be an immediate. With a fill known only at run time, `zkvm_memset` calls the out-of-line `zkvm_memset_any`, which picks the immediate from a 256-entry jump table. The memcmp result rides on the `csrrs` destination; putting it on the `add` instead is the deprecated spelling.

The instructions of a sequence must stay adjacent and in order. In C, keep each sequence in one `asm` statement.

## Thunk form (non-inline)

A thunk call is an ordinary RISC-V function call. The callee is two instructions:

```asm
zkvm_sha256:
    csrs    0x851, x0     # zkvmcall: the function ID is the CSR address
    ret                   # never executed
```

Unlike the inline form, a thunk does not copy the routine into the program. An inline zkvmcall inserts its own copy of the body at each call site, reading the registers that site's `csrs` sequence names. A thunk's routine has one copy in the ZisK library ROM, shared by every call site: the program jumps to it, and it returns to the caller. The arguments therefore have to be in the fixed registers of the calling convention.

The contract:

1. **Arguments.** The caller follows the RISC-V calling convention: arguments in `a0`..`a7` (pointers, lengths, counts), return address in `ra`. The C prototypes in the headers are the signatures.
2. **Transpilation.** A `csrs <id>, x0` with `id` in `0x850`..`0x8BF` is a zkvmcall. The transpiler replaces it with a jump, without link, to the matching `ziskasm_zkvm_*` routine. The rest of the thunk is never reached; the `ret` only makes the stub look like a normal function to tools.
3. **Return.** The routine reads its arguments from `a0`..`a7`, returns a `zkvm_status` in `a0` (`ZKVM_EOK` = 0 on success) and `ret`s straight to the caller through `ra`.
4. **Clobbers.** As any call: caller-saved registers are clobbered, callee-saved ones survive. Memory is read and written only through the argument pointers.
5. **Identification.** The transpiler matches the instruction, not the symbol name, so a stripped ELF works and any function containing that `csrs` is a thunk.

In C or C++ there is nothing special to write: include the header, link `zisklib_c` (which assembles `zkvm_calls.s`), and call the function.

## CSR addresses

The function ID is the CSR address. zkvmcalls occupy `0x850`..`0x8BF`; IDs are never reused or renumbered. The source of truth is `definitions/src/zkvmcall.rs`, which a transpiler test checks against `zkvm_calls.s`.

| CSR | Function | Form |
| --- | --- | --- |
| `0x800` | `zkvm_keccak_f1600` (keccak-f precompile) | inline |
| `0x813` | `zkvm_memcpy` (DMA copy, overlap-safe) | inline marker |
| `0x814` | `zkvm_memcmp` (DMA compare) | inline marker |
| `0x816` | `zkvm_memset` (DMA fill) | inline marker |
| `0x81C` | `zkvm_evm_jumpdest_bitmap` | inline marker |
| `0x850` | `zkvm_keccak256` | thunk |
| `0x851` | `zkvm_sha256` | thunk |
| `0x852` | `zkvm_ripemd160` | thunk |
| `0x853` | `zkvm_secp256k1_verify` | thunk |
| `0x854` | `zkvm_secp256k1_ecrecover` | thunk |
| `0x855` | `zkvm_secp256r1_verify` | thunk |
| `0x856` | `zkvm_modexp` | thunk |
| `0x857`..`0x859` | `zkvm_bn254_g1_add`, `_g1_mul`, `_pairing` | thunk |
| `0x85A` | `zkvm_blake2f` | thunk |
| `0x85B` | `zkvm_kzg_point_eval` | thunk |
| `0x85C`..`0x862` | `zkvm_bls12_g1_add`, `_g1_msm`, `_g2_add`, `_g2_msm`, `_pairing`, `_map_fp_to_g1`, `_map_fp2_to_g2` | thunk |
| `0x863`, `0x864` | `read_input`, `write_output` (`zkvm_io.h`) | thunk |
| `0x865`..`0x87F` | EF big-endian U256 (`zkvm_u256.h`): add, sub, mul, div, mod, divmod, addmod, mulmod, exp, sdiv, smod, sdivmod, lt, gt, slt, sgt, eq, iszero, and, or, xor, not, byte, shl, shr, sar, signextend | inline zkvmcall, except the division family and exp (thunk) |
| `0x880`..`0x89A` | ZisK little-endian U256 (`zkvm_u256_le.h`): div, mod, divmod, sdiv, smod, sdivmod, add, sub, mul, addmod, mulmod, exp, lt, gt, slt, sgt, eq, iszero, and, or, xor, not, byte, shl, shr, sar, signextend | inline zkvmcall, except the division family (`0x880`..`0x885`) and exp (`0x88B`) (thunk) |
| `0x89B`..`0x8B2` | ZisK library functions (`zkvm_zisklib.h`) | thunk |
| `0x8E0`..`0x8E7` | arguments 1..8 of an inline zkvmcall | argument carrier |

Every inline zkvmcall can also be reached as a thunk: a lone `csrs <id>, x0` expands the same routine on `a0`, `a1`, … and sets `a0 = ZKVM_EOK`. A guest built against a header that only declares the function keeps working.

## Examples

**Inline zkvmcall: 256-bit multiply, little-endian limbs.** As defined in `zkvm_u256_le.h`. The memory operands tell the compiler what is read and written; no register is clobbered.

```c
static inline __attribute__((always_inline)) zkvm_status zkvm_u256_le_mul(
    const zkvm_u256_le* a, const zkvm_u256_le* b, zkvm_u256_le* result) {
    __asm__("csrs 0x888, %1\n\t"     /* zkvmcall, argument 0 */
            "csrs 0x8E0, %2\n\t"     /* argument 1 */
            "csrs 0x8E1, %3"         /* argument 2 */
            : "=m"(*result) : "r"(a), "r"(b), "r"(result), "m"(*a), "m"(*b));
    return ZKVM_EOK;
}
```

**Inline precompile: keccak-f[1600].**

```c
static inline zkvm_status zkvm_keccak_f1600(uint64_t* state) {
    __asm__ volatile("csrs 0x800, %0" : : "r"(state) : "memory");
    return ZKVM_EOK;
}
```

**Inline marker: a fixed 32-byte copy.** With a constant size the call becomes one ZisK instruction.

```c
zkvm_memcpy(dst, src, 32);
/* emits:  csrs 0x813, a1
 *         addi x0, a0, 32   */
```

**Thunk: SHA-256 from C.** A plain call; the thunk comes from `zkvm_calls.s` in `zisklib_c`.

```c
#include "zkvm_accelerators.h"

zkvm_sha256_hash h;
zkvm_status st = zkvm_sha256(msg, msg_len, &h);   /* st == ZKVM_EOK */
```

**Thunk: SHA-256 from assembly.** Load the arguments as for any call. `call` reaches the thunk, whose `csrs` the transpiler turns into a jump to `ziskasm_zkvm_sha256`; it returns to the instruction after `call`.

```asm
    la    a0, msg          # data
    li    a1, 64           # len
    la    a2, digest       # output (32 bytes)
    call  zkvm_sha256      # status in a0
```

**Thunk without a stub.** The `csrs` can be issued directly; it behaves as a tail call, so `ra` must already hold the return address.

```asm
    # a0 = a, a1 = b, a2 = quotient, ra = return address
    csrs  0x880, x0        # zkvm_u256_le_div: jumps to the routine, returns to ra
```

## Rules

- **Alignment.** Pass 8-byte-aligned buffers to the accelerator and U256 functions; the header types carry `ALIGN8`. The JUMPDEST bitmap requires `code` and `bitmap` 8-byte aligned and `size > 0`, otherwise `zkvm_evm_jumpdest_bitmap` returns `ZKVM_EFAIL` without writing.
- **Aliasing.** U256 results may alias their inputs. `zkvm_memcpy` is overlap-safe, so it is also a memmove.
- **Immediates.** An `addi` immediate is at most 2047. Larger or run-time sizes use the register form.
- **Status.** Thunk routines return a real `zkvm_status` in `a0`. Inline U256 and keccak-f always return `ZKVM_EOK`.
- **Adjacency.** Nothing may be scheduled between the instructions of a marker pair or an inline zkvmcall. One `asm` statement per sequence guarantees it.
- **Other targets.** Off ZisK the headers only declare the functions, and host builds must supply their own implementations.

## Reference sources

All in the zisk repo:

| Topic | File |
| --- | --- |
| Function IDs, inline argument CSRs | `definitions/src/zkvmcall.rs` |
| Thunks | `ziskasm/lang/c/src/zkvm_calls.s` |
| EF accelerators, keccak-f inline | `ziskasm/lang/c/include/zkvm_accelerators.h` |
| EF and little-endian U256 | `ziskasm/lang/c/include/zkvm_u256.h`, `zkvm_u256_le.h` |
| Memory markers, out-of-line `mem*` | `ziskasm/lang/c/include/zkvm_mem.h`, `src/zkvm_mem.s` |
| JUMPDEST marker | `ziskasm/lang/c/include/zkvm_evm.h` |
| Marker transpilation | `transpilers/riscv/src/riscv2zisk_context.rs` |
| Routine bodies | `ziskasm/zisklib/zkvm/` |
