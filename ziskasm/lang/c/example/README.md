# C-binding end-to-end test

Proves the [`ziskasm/lang/c`](../) binding works end to end: a real C guest calls
`zkvm_keccak256`, and at transpile time `elf2rom` turns that zkvmcall into a jump to
the hand-written `ziskasm_zkvm_keccak256` routine in `ziskasm/zisklib/zkvm/keccak.zisk`,
so the `.zisk` implementation runs in the guest's place.

## Level 1 — the C binding (runs today)

```bash
./build_and_run.sh            # uses riscv64-unknown-elf-gcc + target/release/ziskemu
```

It builds a minimal freestanding C guest ([`ef_keccak_guest.c`](ef_keccak_guest.c))
that calls `zkvm_keccak256(input, 0, &out)`, emits the 32-byte result to the ZisK
public-output region, and the script checks it against the canonical
`keccak256("") = c5d2460186f7233c…d85a470`. It builds `ziskemu` first if
`target/release/ziskemu` does not exist.

Overridable env vars: `RISCV_CC`, `ZISKEMU`, `ZISK_ROOT`, `OUT`.

## U256 aliasing conformance

[`u256_alias_guest.c`](u256_alias_guest.c) checks the one promise `zkvm_u256.h` makes
that the other guests never exercise: *"The result pointer MAY alias any input
pointer."* Every `ziskasm_zkvm_u256_*` routine must finish reading its operands before
writing any result word, or an in-place call like `add(&x, &y, &x)` — the normal shape
for an EVM interpreter, where operands are popped and the result pushed over the same
stack slot — would read back its own output.

```bash
riscv64-unknown-elf-gcc -march=rv64ima_zicsr -mabi=lp64 -mcmodel=medany -nostdlib \
    -ffreestanding -O2 -I. -I../include -T ../../../../ziskbuild/zisk_linker_script.ld -o /tmp/u256.elf \
    ../src/_start.s u256_alias_guest.c ../src/zkvm_calls.s
: > /tmp/empty.bin
../../../../target/release/ziskemu -e /tmp/u256.elf -i /tmp/empty.bin -o /tmp/u256.bin
xxd -p -l 58 -c 58 /tmp/u256.bin
```

It runs each op into a distinct buffer, re-runs it with the result aliased onto each
input in turn, and compares — 27 ops, 56 checks, covering the three-input
`addmod`/`mulmod` and the two-output `divmod`/`sdivmod` in both orders. Unlike the
other guests it is self-checking, so there is no golden vector: one byte per case,
`00` = pass, `01` = mismatch.

Expected output: **56 zero bytes, then `0100`.** Those last two are a negative control
(`ne(&A,&B)` then `ne(&A,&A)`) — without them an all-zero result would be
indistinguishable from a harness that never compared anything.

## U256 signed semantics

[`u256_semantics_guest.c`](u256_semantics_guest.c) covers the sign-sensitive half of
the U256 ABI that the aliasing guest does not touch: `slt`, `sgt`, `sdiv`, `smod`,
`sar` and `signextend`, plus unsigned `mod` at its edge divisors. These are where EVM
semantics are easiest to get subtly wrong
— truncate-toward-zero rather than floor division, the modulo taking the sign of the
*dividend*, the `-2^255 / -1` overflow that wraps to itself, arithmetic vs logical
right shift, and shift counts ≥ 256 saturating to `0` or `-1`.

```bash
riscv64-unknown-elf-gcc -march=rv64ima_zicsr -mabi=lp64 -mcmodel=medany -nostdlib \
    -ffreestanding -O2 -I. -I../include -T ../../../../ziskbuild/zisk_linker_script.ld -o /tmp/u256sem.elf \
    ../src/_start.s u256_semantics_guest.c ../src/zkvm_calls.s
: > /tmp/empty.bin
../../../../target/release/ziskemu -e /tmp/u256sem.elf -i /tmp/empty.bin -o /tmp/sem.bin
xxd -p -l 59 -c 59 /tmp/sem.bin
```

57 cases, self-checking as above: `00` = pass, `01` = wrong value **or** a non-`EOK`
status. Expected output: **57 zero bytes, then `0100`** (the same negative control).

The expected values are golden vectors from an *independent* Python model of the EVM
semantics — deliberately not transcribed from this backend, so the test cannot agree
with a bug by construction (see the warning about hand-transcribed vectors in
[`zisklib.md`](../../../zisklib.md)). To extend it, add the case to that model and
regenerate rather than hand-writing a 32-byte constant.

## Level 2 — a real block through ziskethone's cpp-guest

ziskethone's C++ guest (branch `feature/zkvm-abi`) reaches all of its crypto, EVM
arithmetic, memory operations and I/O through this binding. Its
[`cpp-guest/zisk/README.md`](../../../../../ziskethone/cpp-guest/zisk/README.md)
explains how to build it and run a block through `ziskemu`; the public output (the
block hash) must match the native guest's.

## Host randomness

[`zkvm_random_guest.c`](zkvm_random_guest.c) calls `zkvm_random_u64` (`zkvm_random.h`)
500 times. It checks in the guest that the values are all distinct and that every bit
varies, and it outputs the first 20 values (crossing the 17-word SHAKE256 block
boundary), the XOR of all 500, and the result of those checks. The values are the
SHAKE256 stream of the 256-bit seed, so pass the same seed (its 32 bytes as 64 hex
digits) to the emulator and to
[`zkvm_random_check.py`](zkvm_random_check.py), which compares the output with Python's
`hashlib.shake_256`:

```bash
riscv64-unknown-elf-gcc -march=rv64ima_zicsr -mabi=lp64 -mcmodel=medany -nostdlib \
    -ffreestanding -O2 -I../include -T ../../../../ziskbuild/zisk_linker_script.ld -o /tmp/random.elf \
    ../src/_start.s zkvm_random_guest.c ../src/zkvm_calls.s
: > /tmp/empty.bin
SEED=000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f
../../../../target/release/ziskemu -e /tmp/random.elf -i /tmp/empty.bin -o /tmp/random.bin \
    --random-seed $SEED
./zkvm_random_check.py $SEED /tmp/random.bin
```

Without `--random-seed`, ziskemu draws a fresh seed for each run (`-v` prints it, so
the run can be replayed). The committed copy of this guest in
`elf-regressions/zkvm_random/` is checked by `emulator/tests/zkvm_random.rs`.

## Fork-join regions

[`zkvm_parallel_guest.c`](zkvm_parallel_guest.c) runs SAXPY (`y = a*x + y`) over four
slices of `y` as one fork-join region (`zkvm_parallel.h`). It demonstrates the raw ABI,
not a speed-up: each task runs about 10^4 steps, and the driver pads every task to a
chunk boundary (2^18 steps), so the region runs far more padding than work and is
slower than a sequential loop. A region pays off with tasks much longer than a chunk.

A task owns one page-aligned range of memory (`mem_start..mem_end`), which must hold
both its stack and everything it writes: run in parallel, a task writes into a private
copy of the memory outside that range, and those writes are lost. So each task's slice
of `y` lives in its arena, next to its stack. `x` and the arguments are written before
the region, so every task can read them. A task list that breaks these rules (ranges
not page-aligned or overlapping, outside the RAM, or a stack outside its range), or that
lies in the memory of one of its tasks, does not fail: the assembly emulator runs the region one task after another, with only a warning
in its log.

Before the region the guest also computes the expected `y`, sequentially. After it, the
guest compares the two and outputs the sum of `y` (184516608) and the result of the
comparison (0 when they match):

```bash
riscv64-unknown-elf-gcc -march=rv64ima_zicsr -mabi=lp64 -mcmodel=medany -nostdlib \
    -ffreestanding -O2 -I../include -T ../../../../ziskbuild/zisk_linker_script.ld -o /tmp/saxpy.elf \
    ../src/_start.s zkvm_parallel_guest.c ../src/zkvm_calls.s
: > /tmp/empty.bin
../../../../target/release/ziskemu -e /tmp/saxpy.elf -i /tmp/empty.bin -o /tmp/saxpy.bin
xxd -p -l 16 /tmp/saxpy.bin    # 0080ff0a000000000000000000000000
```

`ziskemu` runs the tasks one after another; the assembly emulator runs them at the same
time, with the same minimal traces (see the book's *Parallel Execution* page for how to
compare them).

[`zkvm_parallel_fcsr_guest.c`](zkvm_parallel_fcsr_guest.c) checks what a region does with
the FCSR, which floating-point code rewrites at every operation: each of four tasks
outputs the FCSR it starts with and leaves another one, and the guest outputs the FCSR
after the region. All five are the value the caller set before the region (0x60).
