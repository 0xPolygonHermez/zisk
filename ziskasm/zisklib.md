# zisklib — calling ZisK-assembly routines from a guest program

`zisklib` lets a normal guest program (Rust → RISC-V ELF) call routines that are
written by hand in ZisK assembly (`.zisk`) instead of compiled from a high-level
language. The routine runs as ordinary ZisK instructions on the main state
machine, so it is **provable with no new secondary state machine** — it is just
faster / more compact than the equivalent compiled code, and can use ZisK ops
(precompiles like `keccak`) directly.

The guest calls a plain function; the wiring that swaps in the hand-written
implementation happens at **transpile time** and is invisible to the guest source.

## The layering

```
guest (Rust)                       zisklib::keccak256(&[u8]) -> [u8;32]     ← ergonomic wrapper (pure Rust)
                                        │  marshals &[u8] -> (ptr,len), returns [u8;32]
zkvmcall thunk (C ABI)             zkvm_keccak256(*const u8, usize, *mut u8) ← naked fn: csrs 0x850, x0; ret
                                        │  (the transpiler turns the csrs into a jump)
ziskasm routine                    ziskasm_zkvm_keccak256:  … keccak op per block …  ← ziskasm/zisklib/zkvm/keccak.zisk
                                                                                (placed in the reserved ZISKLIB ROM region)
```

- The **ergonomic wrapper** and the **zkvmcall thunk** live in the `zisklib` crate
  (`ziskasm/lang/rust/`). The thunk is a two-instruction naked function whose
  `csrs` names the zkvmcall ID (`definitions/src/zkvmcall.rs`).
- During transpilation, [`elf2rom`](../transpilers/riscv/src/elf2rom.rs) finds each
  zkvmcall by its `csrs` and **replaces it with a jump** to the matching hand-written
  routine (assembled from `ziskasm/zisklib/*.zisk` and merged into the ROM at a
  reserved region). The routine returns straight to the guest caller. Calls are
  found by instruction, so the guest ELF may be stripped.

`ziskasm/lang/rust/` is the **Rust** binding; sibling `ziskasm/lang/<language>/`
directories can provide the same surface for other guest languages.

---

## Using the library in a guest

### 1. Depend on the crate

In the guest crate's `Cargo.toml`:

```toml
[dependencies]
ziskos = { workspace = true }
zisklib = { path = "…/ziskasm/lang/rust" }   # adjust the relative path
```

### 2. Call a function

```rust
#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let input: &[u8] = ziskos::io::read_slice();
    let digest: [u8; 32] = zisklib::keccak256(input);   // runs the ziskasm routine
    ziskos::io::commit_slice(&digest);
}
```

That's it — `zisklib::keccak256` looks and behaves like a normal function. On the
ZisK target the transpiler routes it through the hand-written `ziskasm_zkvm_keccak256`.

### 3. Build and run

Build the guest ELF with the **repository's** `cargo-zisk` (the version bundled in
`~/.zisk/bin` may be an older release whose linker-script wiring differs):

```
target/debug/cargo-zisk build --release        # from the guest crate directory
```

Run / prove it through the normal ELF pipeline; the zkvmcalls are resolved inside
`elf2rom`, so nothing special is needed:

```
ziskemu -e <guest>.elf -i input.bin -c          # emulate
cargo-zisk prove -e <guest>.elf -i input.bin    # prove
```

A complete, self-checking example is in
[`examples/zisklib-demo/guest/`](../examples/zisklib-demo/guest/): it exercises
every routine below against hardcoded known-answer vectors and commits a single
`ok` boolean.

## Current API

Grouped by family: **hashes** (keccak256, sha256, blake2b), and the **256-bit
integer** suite (`[u64; 4]`, little-endian limbs) covering arithmetic, division,
modular arithmetic, inverses, and exponentiation. Every routine below is verified
against known-answer vectors in the demo guest.

| Rust API (`zisklib::`) | ziskasm routine | Notes |
|------------------------|-----------------|-------|
| `keccak256(input: &[u8]) -> [u8; 32]` | `ziskasm_zkvm_keccak256` | keccak256 digest of any-length, any-alignment input. |
| `sha256(input: &[u8]) -> [u8; 32]` | `ziskasm_zkvm_sha256` | SHA2-256 (FIPS 180-4) digest of any-length, any-alignment input. |
| `blake2b_compress(rounds, h: &mut [u64;8], m: &[u64;16], t: &[u64;2], f: bool)` | `ziskasm_zkvm_blake2f` | BLAKE2b compression function F (RFC 7693) — low-level primitive; caller does blocking/padding. |
| `{overflowing,checked,saturating,wrapping}_add256` / `_sub256` | `zisklib_overflowing_add256` / `_sub256` | 256-bit (`[u64; 4]`) add / subtract; the variants are Rust wrappers over the two overflowing cores. |
| `{overflowing,checked,wrapping}_neg256` | (`zisklib_overflowing_sub256`) | 256-bit negation (`0 - a`). |
| `{overflowing,checked,saturating,wrapping}_mul256` / `_square256` | `zisklib_overflowing_mul256` | 256-bit multiply / square (low 256 bits + overflow); square = `mul(a, a)`. |
| `div_rem256` / `{wrapping,checked}_div256` / `_rem256` / `div_ceil256` | `zisklib_div_rem256` | 256-bit Euclidean division (hint + arith256 verify + `r < b` check); `checked_*` guard `b == 0` in Rust, the others panic/halt. |
| `reduce_mod256` / `add_mod256` / `mul_mod256` / `square_mod256` | `zisklib_reduce_mod256` / `_add_mod256` / `_mul_mod256` | modular reduce / add / multiply / square (`arith256_mod` precompile); `modulus == 0` returns `0` (guarded in Rust). |
| `inv256(a: &[u64; 4]) -> Option<[u64; 4]>` | `zisklib_inv256` | Inverse mod 2^256 (hint + arith256 verify). |
| `inv_mod256(a, modulus) -> Option<[u64; 4]>` | `zisklib_inv_mod256` | Modular inverse (fcall hint; verifies `a·inv ≡ 1 (mod m)` or a gcd witness for non-existence). |
| `pow_mod256(base, exp, modulus)` | `zisklib_pow_mod256` | Modular exponentiation `base^exp mod m` (square-and-multiply over `arith256_mod`); `m in {0,1}` → 0. |
| `{overflowing,checked,saturating,wrapping}_pow256` | `zisklib_overflowing_pow256` | `base^exp mod 2^256` with overflow flag (square-and-multiply over `arith256`). |
| `zkvm_zisklib_add(a: u64, b: u64) -> u64` (raw thunk) | `zisklib_add` | Demo / smoke-test (a + b). |

The crate also wraps the secp256k1 / secp256r1 signatures, the BN254 and BLS12-381
pairings, maps and hashes to curves, BLS and KZG verification, and modexp (see
[`lang/rust/src/lib.rs`](lang/rust/src/lib.rs)). The surface grows over time; see
"Adding a routine" below.

---

## How a call reaches the routine

1. **Reserved ROM/RAM regions.** `ziskasm/zisklib/*.zisk` is assembled in *library
   mode* (no launcher / `_start` / BIOS) at `ZISKLIB_ROM_ADDR` — a 1 MB region
   carved just below the float library (see `core/src/mem.rs`). Its `const` data
   sits right after the code; its mutable scratch/variables go to `ZISKLIB_RAM_ADDR`
   (a reserved RAM slice). The guest linker fences these off so guest allocations
   never collide.
2. **Merge.** `elf2rom` assembles the library (`ziskasm::assemble_zisk_library`,
   embedding each `.zisk` file at compile time via `include_str!`) and merges its
   instructions + data into the guest's ROM.
3. **zkvmcall.** Every `csrs <id>, x0` with an ID in the zkvmcall range
   (0x850..0x8BF) is replaced by a static tail-jump into the routine that
   `definitions/src/zkvmcall.rs` maps the ID to. Because it is a *tail* jump, the
   return address (`ra` / `r1`) is untouched, so the routine's `ret` returns to the
   guest caller. An *inline* zkvmcall (a `csrs` per argument) is instead replaced by
   the routine's body, on the registers the compiler chose.

The library is only assembled and merged when the guest uses a zkvmcall, so other
guests pay nothing.

---

## Adding a routine

Three edits plus the implementation. Say you want `zisklib_foo`.

### 1. Write the ziskasm routine — `ziskasm/zisklib/foo.zisk`

Follow the calling convention (RISC-V C ABI; ZisK registers *are* the RISC-V
registers):

- **Arguments** arrive in `r10..r17` (`a0..a7`); **return value** goes in `r10`
  (`a0`). The routine is entered via a tail-jump, so `r1` (`ra`) holds the guest
  return address and a final `ret` returns there.
- **Scratch freely:** `r5..r7` (`t0..t2`), `r12..r17` (`a2..a7`), `r28..r31`
  (`t3..t6`).
- **Must preserve** (do not clobber): `r8`/`r9` and `r18..r27` (`s0..s11`), `r2`
  (`sp`), `r3` (`gp`), `r4` (`tp`). ziskasm has no push/pop idiom, so prefer using
  only caller-saved registers; if you need more state, use a scratch buffer in
  `ZISKLIB_RAM` (below) rather than the stack.
- **Mutable state / scratch:** declare non-`const` data in the `.zisk` file — it is
  placed in `ZISKLIB_RAM` (writable), which is required for in-place ops like
  `keccak`. `const` data is placed in `ZISKLIB_ROM` (read-only).
- **Prefix internal labels** per family (e.g. `zk_` for keccak) so they stay unique
  when all `.zisk` files are concatenated into one library.

See [`ziskasm/zisklib/zkvm/keccak.zisk`](zisklib/zkvm/keccak.zisk) for a full example
(a keccak256 sponge that calls the `keccak` op once per rate block).

Two recurring shapes are worth knowing (both used throughout the `zisklib/uint256/*.zisk` files):

- **Precompile with a pointer header.** Most precompiles (`arith256`,
  `arith256_mod`, `add256`, `sha256`, `blake2`) take a small header — a run of
  pointers (and sometimes a direct value) — whose *address* goes in the `b`
  operand: e.g. `arith256_mod` reads `[&a, &b, &c, &module, &d]` and writes `d`.
  Build the header in a `ZISKLIB_RAM` scratch buffer at runtime, then invoke the op
  (`arith256_mod(0, r5)`). Precompiles never raise the register flag, so a plain
  fall-through follows (`jmp_offset1` is forced to 0 by the assembler).
- **Hint-then-verify.** For results that are cheap to *check* but expensive to
  *compute* (division, inverses), an `fcall` supplies an untrusted answer that the
  routine then verifies with a precompile (e.g. `div_rem256` hints `(q, r)`, then
  checks `q·b + r == a` and `r < b`); a bad hint halts via `copyb(...) , end`,
  mirroring the reference `assert!`. **The fcall passthrough throwaway must target a
  caller-saved register** (e.g. `r14`), never `r4` (`tp`), which is callee-saved.

### 2. Register it in the library — `ziskasm/src/zisklib.rs`

Add the source file to `ZISK_LIBRARY`:

```rust
pub const ZISK_LIBRARY: &[(&str, &str)] = &[
    // …
    ("foo", include_str!("../zisklib/foo.zisk")),
];
```

### 3. Give it a zkvmcall ID — `definitions/src/zkvmcall.rs`

Add a row with the next free ID. IDs are never reused or renumbered once assigned:

```rust
    zc(0x8B3, "zkvm_zisklib_foo", "zisklib_foo"),   // (ID, guest function, routine label)
```

### 4. Add the bindings

The C thunk goes in `ziskasm/lang/c/src/zkvm_calls.s` (a test in
`transpilers/common` checks that it matches the table) and its prototype in the
header for its family:

```
ZKVMCALL zkvm_zisklib_foo, 0x8B3
```

The Rust thunk, plus an ergonomic wrapper if useful, goes in
`ziskasm/lang/rust/src/lib.rs`; the macro looks the ID up by name:

```rust
zkvmcall! {
    /// `foo(input[0..len])` -> `output`, a zkvmcall to `zisklib_foo`.
    ///
    /// # Safety
    /// … describe the pointer/length contract …
    fn zkvm_zisklib_foo(input: *const u8, len: usize, output: *mut u8) -> ()
}

/// Ergonomic wrapper.
pub fn foo(input: &[u8]) -> [u8; OUTPUT_LEN] {
    let mut out = [0u8; OUTPUT_LEN];
    unsafe { zkvm_zisklib_foo(input.as_ptr(), input.len(), out.as_mut_ptr()) };
    out
}
```

Rebuild the guest and it can call `zisklib::foo(...)`.

---

## Rules & gotchas

- **Never reuse or renumber a zkvmcall ID.** An ELF built against an old table
  would silently call a different routine.
- **Respect the callee-saved contract.** A routine that clobbers `s0..s11`,
  `sp`, `gp`, or `tp` will corrupt the guest after it returns.
- **Thunks only run under ZisK.** On a host build a thunk panics. If you also want
  the program to run natively, give the wrapper a real fallback behind
  `#[cfg(not(zisk_guest))]`.
- **Use the repository `cargo-zisk`** to build guests (`target/debug/cargo-zisk`),
  not a stale installed release.
- **Performance:** keep the hot loop cheap. `ziskasm_zkvm_keccak256` absorbs full rate
  blocks in a tight word loop (same cost whatever the length) and does byte-level
  work only for the ≤7-byte final tail, so arbitrary-length support adds no penalty
  to word-aligned inputs. The `pow`/`pow_mod` routines find the exponent's
  most-significant set bit first, so a small exponent costs only as many
  squarings as it has bits.
- **Test the routine in isolation first.** Assemble the `.zisk` file with a small
  hand-written caller and run it under `ziskemu -z <dir> -c` before wiring the
  zkvmcall — it's far faster than the ~30 s guest rebuild. Compare against a
  *independently* produced golden vector (`sha256sum`, `b2sum`, a throwaway host
  harness): a value transcribed by hand is itself suspect, and a mismatch is as
  likely a bad expected constant as a bad routine — check element-by-element.

## File map

| Path | Role |
|------|------|
| [`ziskasm/lang/rust/`](lang/rust/) | Crate `zisklib`: Rust zkvmcall thunks + ergonomic wrappers. |
| [`ziskasm/lang/c/`](lang/c/) | The C binding: zkvmcall thunks (`src/zkvm_calls.s`) and headers. |
| [`definitions/src/zkvmcall.rs`](../definitions/src/zkvmcall.rs) | The zkvmcall table: ID → guest function → routine. |
| [`ziskasm/src/zisklib.rs`](src/zisklib.rs) | `ZISK_LIBRARY`: the `.zisk` files of the library. |
| [`ziskasm/zisklib/`](zisklib/) | Hand-written ziskasm routines, one file per family (`keccak.zisk`, …); large families get a subdirectory (`zisklib/uint256/`). Shared `pub define`s live in `mem.zisk` / `fcall.zisk`. |
| [`transpilers/riscv/src/elf2rom.rs`](../transpilers/riscv/src/elf2rom.rs) | Finds the zkvmcalls; assembles and merges the library; emits the jumps. |
| `core/src/mem.rs` | `ZISKLIB_ROM_ADDR` / `ZISKLIB_RAM_ADDR` reserved regions. |
| [`ziskasm/src/assembler.rs`](src/assembler.rs) | `assemble_library*` (library mode). |
| [`examples/zisklib-demo/guest/`](../examples/zisklib-demo/guest/) | Worked example guest. |

See also [`ziskasm.md`](ziskasm.md) (the `.zisk` language) and
[`ziskbin.md`](ziskbin.md) (the ROM-in-ELF binary format).
