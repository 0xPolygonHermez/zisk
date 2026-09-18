# C-binding end-to-end test

Proves the [`ziskasm/lang/c`](../) binding works end to end: a real C guest calls
`ziskos_keccak`, and at transpile time `elf2rom` redirects that symbol to the
hand-written `ziskasm_zkvm_keccak256` routine in `ziskasm/zisklib/zkvm/keccak.zisk`, so the
`.zisk` implementation runs in the guest's place.

## Level 1 — the C binding + redirect (runs today)

```bash
./build_and_run.sh            # uses riscv64-unknown-elf-gcc + target/release/ziskemu
```

It builds a minimal freestanding C guest ([`main.c`](main.c)) that calls
`ziskos_keccak(input, 0, out)`, emits the 32-byte result to the ZisK public-output
region, and the script checks it against the canonical `keccak256("") =
c5d2460186f7233c…d85a470`.

- **PASS** (the real hash) ⇒ the redirect fired and the `.zisk` routine produced
  the correct result.
- If the redirect had *not* fired, the C stub in `src/zisklib_stubs.c` fills the
  output with `0xBA`, so a wrong `bababa…` hash would show — the negative control.

This confirms the mechanism is real: any ELF (C, C++, Rust) that exports and calls
a `ziskos_*` symbol from the `REDIRECTS` table gets the shared `.zisk` routine.

Overridable env vars: `RISCV_CC`, `ZISKEMU`, `ZISK_ROOT`, `OUT`.

## U256 aliasing conformance

[`u256_alias_guest.c`](u256_alias_guest.c) checks the one promise `zkvm_u256.h` makes
that the other guests never exercise: *"The result pointer MAY alias any input
pointer."* Every `ziskasm_zkvm_u256_*` routine must finish reading its operands before
writing any result word, or an in-place call like `add(&x, &y, &x)` — the normal shape
for an EVM interpreter, where operands are popped and the result pushed over the same
stack slot — would read back its own output.

```bash
riscv64-unknown-elf-gcc -march=rv64ima -mabi=lp64 -mcmodel=medany -nostdlib \
    -ffreestanding -O2 -I. -I../include -T zisk_guest.ld -o /tmp/u256.elf \
    _start.s u256_alias_guest.c ../src/zkvm_stubs.c
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

## Level 2 — a real block through ziskethone's cpp-guest

Same mechanism, applied to the block prover. In `../../../../../ziskethone`:

1. **Toolchain.** The C++ guest needs xpack `riscv-none-elf-g++` 14 or 16 on
   `PATH` (Ubuntu's `riscv64-unknown-elf-g++` 13 lacks libstdc++ headers — it can
   build this pure-C example but not the C++ guest).
2. **Wire one precompile.** In `cpp-guest/zisk/keccak_zisk.cpp`, replace the body
   of `ethash_keccak256` with a call to `ziskos_keccak` (from `<zisklib.h>`); add
   `src/zisklib_stubs.c` and this binding's `include/` to the cmake target.
   keccak is the cleanest first cut — the whole guest funnels through that one
   symbol.
3. **Build + run.** Build the guest ELF, then run it through the *local*
   `target/release/ziskemu` (which carries the `REDIRECTS` table) on a framed
   block input (see `ziskethone/cpp-guest/zisk/README.md`). The public output —
   the block hash — must match the reference `52f6334943830a72…`.
4. **Measure.** `-m` for steps, `-X` for the proving-cost report; compare the
   keccak share against the baseline (this block was ~48.25M steps, keccak 35% of
   cost).

Do NOT `--strip` the guest ELF: `elf2rom` resolves the stubs by name in `.symtab`.
No linker-script change is needed — the `.zisk` code is merged into the ROM by
`elf2rom`, not linked into the ELF.
