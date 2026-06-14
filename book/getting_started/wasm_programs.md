# WebAssembly Programs

In addition to its native RISC-V guest, ZisK can run **`wasm32` + WASI** guests. A WebAssembly
module is transpiled to the Zisk ISA at load time, the same way a RISC-V ELF is — so everything
downstream (the emulator, the state machines, the prover) is unchanged. The guest machine is
selected automatically from the file's magic bytes: `\x7fELF` → RISC-V, `\0asm` → WebAssembly.

The appeal of the wasm machine is that it needs **no custom toolchain**: any program that compiles
to the stock `wasm32-wasip1` target can be proven by ZisK.

## Building a wasm guest

Add the target once:

```bash
rustup target add wasm32-wasip1
```

Then build with plain cargo:

```bash
cargo build --release --target wasm32-wasip1
```

or through the ZisK CLI, which places the artifact under `target/elf/wasm32-wasip1/…`:

```bash
cargo-zisk build --release --machine wasm
```

## Running

`wasmemu` transpiles a `.wasm` module and emulates it; guest stdout/stderr stream to the console.

```bash
wasmemu path/to/guest.wasm                 # no input
wasmemu -i input.bin path/to/guest.wasm    # input.bin is fed to the guest's stdin
wasmemu -x path/to/guest.wasm              # also dump the public output region as hex
```

`ziskemu --elf guest.wasm -i input.bin` accepts `.wasm` files too (the magic bytes are detected
automatically), and `cargo-zisk execute|prove --elf guest.wasm --emulator` proves them.

## Guest input / output

* **stdin** (`fd_read`) is mapped to the Zisk input region. The input blob is the standard ZisK
  format: an 8-byte little-endian length prefix followed by the data.
* **stdout / stderr** (`fd_write`) are streamed to the console (UART) and mirrored into the public
  output region, so the first bytes a program prints are recoverable from the proof's public output.

## Supported subset

The wasm machine targets the MVP **integer** subset:

* full control flow — `block` / `loop` / `if` / `else` / `br` / `br_if` / `br_table` / `return`;
* `i32` and `i64` arithmetic, logic, comparison, shifts/rotates, `clz` / `ctz` / `popcnt`, and the
  integer conversions / sign extensions;
* locals and globals;
* direct `call` and `call_indirect` (with a structural type check);
* linear memory — loads/stores of every width, `memory.size` / `memory.grow`, and bulk
  `memory.copy` / `memory.fill`;
* a minimal WASI surface: `proc_exit`, `fd_read` (stdin), `fd_write` (stdout/stderr), and
  deterministic stubs for `args_*`, `environ_*`, `random_get` (zero-filled), `clock_time_get`, and
  `fd_fdstat_get`.

Floating point (`f32` / `f64`), SIMD, threads and reference types are **rejected at transpile time**
with a clear error, rather than miscompiled.

### Notes and current limits

* `random_get` is zero-filled and `clock_time_get` returns 0 — a zkVM execution must be
  deterministic, so there is no entropy or wall clock.
* `memory.copy` performs a forward byte copy; fully-overlapping `dst > src` copies are not handled.
* `br_table` is supported for branches that carry no result values.
* Only the first 32 words (256 bytes) of stdout are part of the *public* output; the full stream
  still reaches the console.
* The ASM-accelerated backend is RISC-V only; wasm guests run on the emulator backend
  (`--emulator`).

See `examples/wasm-sum` for a complete, runnable example.
