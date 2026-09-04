# C-binding end-to-end test

Proves the [`ziskasm/lang/c`](../) binding works end to end: a real C guest calls
`ziskos_keccak`, and at transpile time `elf2rom` redirects that symbol to the
hand-written `zisklib_keccak` routine in `ziskasm/zisklib/keccak.zisk`, so the
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
