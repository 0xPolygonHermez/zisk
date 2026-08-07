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
raw ABI boundary (C ABI)           ziskos_keccak(*const u8, usize, *mut u8) ← #[no_mangle] stub, placeholder body
                                        │  (transpile-time symbol redirect)
ziskasm routine                    zisklib_keccak:  … keccak op per block …  ← ziskasm/lib/zisklib_keccak.zisk
                                                                                (placed in the reserved ZISKLIB ROM region)
```

- The **ergonomic wrapper** and the **raw stub** live in the `zisklib` crate
  (`ziskasm/lang/rust/`). The stub is a real `#[no_mangle]` symbol with a
  throwaway body.
- During transpilation, [`elf2rom`](../transpilers/common/src/elf2rom.rs) finds the
  stub's symbol in the guest ELF and **redirects its entry** to the matching
  hand-written `zisklib_*` routine (assembled from `ziskasm/lib/*.zisk` and merged
  into the ROM at a reserved region). The routine returns straight to the guest
  caller.

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
ZisK target the transpiler routes it through the hand-written `zisklib_keccak`.

### 3. Build and run

Build the guest ELF with the **repository's** `cargo-zisk` (the version bundled in
`~/.zisk/bin` may be an older release whose linker-script wiring differs):

```
target/debug/cargo-zisk build --release        # from the guest crate directory
```

Run / prove it through the normal ELF pipeline; the redirect happens inside
`elf2rom`, so nothing special is needed:

```
ziskemu -e <guest>.elf -i input.bin -c          # emulate
cargo-zisk prove -e <guest>.elf -i input.bin    # prove
```

A complete, self-checking example (which also diffs the result against the
reference `ziskos::zisklib::keccak256`) is in
[`examples/zisklib-demo/guest/`](../examples/zisklib-demo/guest/).

## Current API

| Rust API (`zisklib::`) | ziskasm routine | Notes |
|------------------------|-----------------|-------|
| `keccak256(input: &[u8]) -> [u8; 32]` | `zisklib_keccak` | keccak256 digest of any-length, any-alignment input. |
| `sha256(input: &[u8]) -> [u8; 32]` | `zisklib_sha256` | SHA2-256 (FIPS 180-4) digest of any-length, any-alignment input. |
| `blake2b_compress(rounds, h: &mut [u64;8], m: &[u64;16], t: &[u64;2], f: bool)` | `zisklib_blake2b_compress` | BLAKE2b compression function F (RFC 7693) — low-level primitive; caller does blocking/padding. |
| `{overflowing,checked,saturating,wrapping}_add256` / `_sub256` | `zisklib_overflowing_add256` / `_sub256` | 256-bit (`[u64; 4]`) add / subtract; the variants are Rust wrappers over the two overflowing cores. |
| `{overflowing,checked,wrapping}_neg256` | (`zisklib_overflowing_sub256`) | 256-bit negation (`0 - a`). |
| `{overflowing,checked,saturating,wrapping}_mul256` / `_square256` | `zisklib_overflowing_mul256` | 256-bit multiply / square (low 256 bits + overflow); square = `mul(a, a)`. |
| `div_rem256` / `{wrapping,checked}_div256` / `_rem256` / `div_ceil256` | `zisklib_div_rem256` | 256-bit Euclidean division (hint + arith256 verify + `r < b` check); `checked_*` guard `b == 0` in Rust, the others panic/halt. |
| `reduce_mod256` / `add_mod256` / `mul_mod256` / `square_mod256` | `zisklib_reduce_mod256` / `_add_mod256` / `_mul_mod256` | modular reduce / add / multiply / square (`arith256_mod` precompile); `modulus == 0` returns `0` (guarded in Rust). |
| `inv256(a: &[u64; 4]) -> Option<[u64; 4]>` | `zisklib_inv256` | Inverse mod 2^256 (hint + arith256 verify). |
| `inv_mod256(a, modulus) -> Option<[u64; 4]>` | `zisklib_inv_mod256` | Modular inverse (fcall hint; verifies `a·inv ≡ 1 (mod m)` or a gcd witness for non-existence). |
| `pow_mod256(base, exp, modulus)` | `zisklib_pow_mod256` | Modular exponentiation `base^exp mod m` (square-and-multiply over `arith256_mod`); `m in {0,1}` → 0. |
| `{overflowing,checked,saturating,wrapping}_pow256` | `zisklib_overflowing_pow256` | `base^exp mod 2^256` with overflow flag (square-and-multiply over `arith256`). |
| `ziskos_add(a: u64, b: u64) -> u64` | `zisklib_add` | Demo / smoke-test (a + b). |

The surface grows over time; see "Adding a routine" below.

---

## How the redirect works

1. **Reserved ROM/RAM regions.** `ziskasm/lib/*.zisk` is assembled in *library
   mode* (no launcher / `_start` / BIOS) at `ZISKLIB_ROM_ADDR` — a 1 MB region
   carved just below the float library (see `core/src/mem.rs`). Its `const` data
   sits right after the code; its mutable scratch/variables go to `ZISKLIB_RAM_ADDR`
   (a reserved RAM slice). The guest linker fences these off so guest allocations
   never collide.
2. **Merge.** `elf2rom` assembles the library (`ziskasm::assemble_library_sources`,
   embedding each `.zisk` file at compile time via `include_str!`) and merges its
   instructions + data into the guest's ROM.
3. **Symbol redirect.** For each registered `(ziskos_*, zisklib_*)` pair, `elf2rom`
   looks up the guest stub's address and size in the ELF symbol table and, when
   transpilation reaches that address, emits a static tail-jump into the library
   routine and skips the stub body. Because it is a *tail* jump, the return address
   (`ra` / `r1`) is untouched, so the routine's `ret` returns to the guest caller.

The redirect and the library are only added when the guest actually references a
registered stub, so unused guests pay nothing.

---

## Adding a routine

Three edits plus the implementation. Say you want `zisklib_foo`.

### 1. Write the ziskasm routine — `ziskasm/lib/zisklib_foo.zisk`

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

See [`ziskasm/lib/zisklib_keccak.zisk`](lib/zisklib_keccak.zisk) for a full example
(a keccak256 sponge that calls the `keccak` op once per rate block).

### 2. Register it in the transpiler — `transpilers/common/src/elf2rom.rs`

Add the source file to `ZISK_LIBRARY` and the redirect pair to `REDIRECTS`:

```rust
const ZISK_LIBRARY: &[(&str, &str)] = &[
    // …
    ("zisklib_foo", include_str!("../../../ziskasm/lib/zisklib_foo.zisk")),
];
const REDIRECTS: &[(&str, &str)] = &[
    // …
    ("ziskos_foo", "zisklib_foo"),   // (guest stub symbol, library routine label)
];
```

### 3. Add the Rust binding — `ziskasm/lang/rust/src/lib.rs`

A raw stub (the ABI boundary) plus, if useful, an ergonomic wrapper:

```rust
/// Raw ABI boundary, redirected to `zisklib_foo`.
/// # Safety
/// … describe the pointer/length contract …
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn ziskos_foo(input: *const u8, len: usize, output: *mut u8) {
    // Placeholder body. MUST touch every argument via `black_box` (see below).
    let (_input, _len, output) = core::hint::black_box((input, len, output));
    for i in 0..OUTPUT_LEN { output.add(i).write(0xBA); }   // obvious sentinel
}

/// Ergonomic wrapper.
pub fn foo(input: &[u8]) -> [u8; OUTPUT_LEN] {
    let mut out = [0u8; OUTPUT_LEN];
    unsafe { ziskos_foo(input.as_ptr(), input.len(), out.as_mut_ptr()) };
    out
}
```

Rebuild the guest and it can call `zisklib::foo(...)`.

---

## Rules & gotchas

- **The stub must touch every argument** (via `core::hint::black_box`), and be
  `#[no_mangle] #[inline(never)]`. The redirected routine reads its arguments from
  `a0..a7`; if the stub's placeholder body ignored an argument, the optimizer would
  elide setting up that argument register at the call site (it only sees the stub
  body), leaving garbage for the real routine. A stub whose result is a pure
  constant is likewise folded away and its symbol garbage-collected — make the body
  observably depend on its inputs / have a side effect.
- **Give each stub a distinct body.** Two stubs with byte-identical bodies (same
  signature, same placeholder) are merged by identical-code folding into a single
  symbol at one address, so their separate `REDIRECTS` entries collide and both
  route to whichever was registered last. Use a per-stub sentinel constant to keep
  the machine code distinct. (Symptom: `readelf -s` shows two `ziskos_*` at the same
  address.)
- **Respect the callee-saved contract.** A routine that clobbers `s0..s11`,
  `sp`, `gp`, or `tp` will corrupt the guest after it returns.
- **Placeholder ≠ native implementation.** The stub body only runs off-target (it
  never runs under ZisK, where it is redirected). If you also want the program to
  run natively, give the wrapper a real fallback behind `#[cfg(not(zisk_guest))]`.
- **Use the repository `cargo-zisk`** to build guests (`target/debug/cargo-zisk`),
  not a stale installed release.
- **Performance:** keep the hot loop cheap. `zisklib_keccak` absorbs full rate
  blocks in a tight word loop (same cost whatever the length) and does byte-level
  work only for the ≤7-byte final tail, so arbitrary-length support adds no penalty
  to word-aligned inputs — no need for a separate fast path.

## File map

| Path | Role |
|------|------|
| [`ziskasm/lang/rust/`](lang/rust/) | Crate `zisklib`: Rust stubs + ergonomic wrappers. |
| [`ziskasm/lib/zisklib_*.zisk`](lib/) | Hand-written ziskasm routines, one file per family. |
| [`transpilers/common/src/elf2rom.rs`](../transpilers/common/src/elf2rom.rs) | `ZISK_LIBRARY` + `REDIRECTS` registries; assembles, merges, and redirects. |
| `core/src/mem.rs` | `ZISKLIB_ROM_ADDR` / `ZISKLIB_RAM_ADDR` reserved regions. |
| [`ziskasm/src/assembler.rs`](src/assembler.rs) | `assemble_library*` (library mode). |
| [`examples/zisklib-demo/guest/`](../examples/zisklib-demo/guest/) | Worked example guest. |

See also [`ziskasm.md`](ziskasm.md) (the `.zisk` language) and
[`ziskbin.md`](ziskbin.md) (the ROM-in-ELF binary format).
