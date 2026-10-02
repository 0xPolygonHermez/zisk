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
`keccak256("") = c5d2460186f7233c…d85a470`. A `ziskemu` built without the
`ziskasm` feature rejects the guest at transpile time; the script checks for the
feature first and builds it if needed.

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
`sar` and `signextend`. These are where EVM semantics are easiest to get subtly wrong
— truncate-toward-zero rather than floor division, the modulo taking the sign of the
*dividend*, the `-2^255 / -1` overflow that wraps to itself, arithmetic vs logical
right shift, and shift counts ≥ 256 saturating to `0` or `-1`.

```bash
riscv64-unknown-elf-gcc -march=rv64ima_zicsr -mabi=lp64 -mcmodel=medany -nostdlib \
    -ffreestanding -O2 -I. -I../include -T ../../../../ziskbuild/zisk_linker_script.ld -o /tmp/u256sem.elf \
    ../src/_start.s u256_semantics_guest.c ../src/zkvm_calls.s
: > /tmp/empty.bin
../../../../target/release/ziskemu -e /tmp/u256sem.elf -i /tmp/empty.bin -o /tmp/sem.bin
xxd -p -l 43 -c 43 /tmp/sem.bin
```

41 cases, self-checking as above: `00` = pass, `01` = wrong value **or** a non-`EOK`
status. Expected output: **41 zero bytes, then `0100`** (the same negative control).

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
