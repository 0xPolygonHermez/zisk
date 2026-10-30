/*
 * zkvm_io.h — Ethereum Foundation zkVM I/O C interface. Mirrors the standard at:
 *   github.com/eth-act/zkevm-standards
 *
 * The private-input and public-output channels. Note these two symbols are NOT
 * `zkvm_`-prefixed: the standard names them plainly.
 *
 * ZisK implements both with no C marshalling layer, exactly like the accelerators:
 * each is a zkvmcall thunk in src/zkvm_calls.s (`csrs <id>, x0; ret`), which the
 * transpiler turns into a jump to `zisklib_read_input` / `zisklib_write_output` in
 * ziskasm/zisklib/zkvm_io.zisk. A guest compiles against this header and links
 * `zisklib_c` (see CMakeLists.txt).
 *
 * A thunk is only a `csrs`: the guest runs under ZisK, whose transpiler gives it
 * meaning, and nowhere else.
 *
 * write_log (draft eth-act PR #27) writes diagnostic text that is not part of the
 * proven statement. On RISC-V it is an inline zkvmcall: two `csrs` naming the
 * registers that hold its arguments (definitions/src/zkvmcall.rs). The transpiler turns
 * them into one step that reads both registers and does nothing else, so a log costs
 * one step and the ROM, and so the program's verification key, does not depend on it;
 * ziskemu prints the text only when asked to (`ziskemu --write-log`); other
 * executions and proofs ignore it. With
 * ZKVM_IO_CALLS defined before including this header, or off RISC-V, write_log is only
 * declared, and is called through its thunk in src/zkvm_calls.s.
 */
#ifndef ZKVM_IO_H
#define ZKVM_IO_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Reports the private input record: `*buf_ptr` = &input[0] (or NULL when the input
 * is empty) and `*buf_size` = its length in bytes. The buffer is read-only and
 * lives for the whole run. Never fails; idempotent — repeated calls report the same
 * record and have no side effects. */
void read_input(const uint8_t** buf_ptr, size_t* buf_size);

/* Appends `size` bytes to the public output. Successive calls concatenate into one
 * byte stream, so a partial trailing word is carried across calls. Never fails. */
void write_output(const uint8_t* output, size_t size);

/* Writes `utf8_len` bytes of UTF-8 diagnostic text. It is not part of the public
 * output and does not affect the execution; ZisK prints it only when the emulator is
 * asked to. Never fails. */
#if defined(__riscv) && !defined(ZKVM_IO_CALLS)
static inline __attribute__((always_inline)) void write_log(const uint8_t* utf8_bytes,
                                                            size_t utf8_len) {
    /* The "memory" clobber makes the compiler store the text before the log */
    __asm__ volatile("csrs 0x8B4, %0\n\t"   /* zkvmcall, argument 0 */
                     "csrs 0x8E0, %1"         /* argument 1 */
                     : : "r"(utf8_bytes), "r"(utf8_len) : "memory");
}
#else
void write_log(const uint8_t* utf8_bytes, size_t utf8_len);
#endif

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ZKVM_IO_H */
