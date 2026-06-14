# wasm-sum — a WebAssembly guest for ZisK

This example demonstrates ZisK's **wasm32 + WASI guest machine**. Unlike the RISC-V examples, it
needs no custom Zisk toolchain: it is a plain Rust program built for the stock `wasm32-wasip1`
target. ZisK transpiles the `.wasm` module to its internal ISA at load time, exactly as it does for
RISC-V ELF guests.

The program reads an 8-byte little-endian `u64` `n` from stdin and prints `1 + 2 + … + n`.

## Build

With the stock wasm target (one-time `rustup target add wasm32-wasip1`):

```bash
cargo build --release --target wasm32-wasip1
# -> target/wasm32-wasip1/release/wasm-sum.wasm
```

or via the ZisK CLI, which places the artifact under `target/elf/wasm32-wasip1/…`:

```bash
cargo-zisk build --release --machine wasm
```

## Run

`wasmemu` transpiles the module to a Zisk ROM and emulates it. Guest stdout streams to the console.

```bash
# default n = 100
wasmemu target/wasm32-wasip1/release/wasm-sum.wasm
# -> sum of 1..=100 = 5050

# feed n = 1000 on stdin
printf '\xe8\x03\x00\x00\x00\x00\x00\x00' > /tmp/n.bin
wasmemu -i /tmp/n.bin target/wasm32-wasip1/release/wasm-sum.wasm
# -> sum of 1..=1000 = 500500
```

`ziskemu --elf wasm-sum.wasm` also works — the `.wasm` magic bytes are detected automatically.

## Proving

Because transpilation produces an ordinary Zisk ROM, the emulator-backed prover path works the same
as for RISC-V guests:

```bash
cargo-zisk prove --elf target/wasm32-wasip1/release/wasm-sum.wasm -i /tmp/n.bin --emulator
```

## Supported subset

The wasm machine currently targets the MVP **integer** subset: full control flow, `i32`/`i64`
arithmetic/logic/comparison/conversion, locals/globals, direct and indirect calls, linear memory
(loads/stores, `memory.size`/`grow`, bulk `memory.copy`/`fill`), and a minimal WASI surface
(`proc_exit`, `fd_read` on stdin, `fd_write` on stdout/stderr, plus `args`/`environ`/`random_get`/
`clock_time_get` stubs). Floating point, SIMD, threads and reference types are rejected at
transpile time with a clear error.
