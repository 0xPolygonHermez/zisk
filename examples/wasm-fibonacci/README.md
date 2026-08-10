# wasm-fibonacci — a WebAssembly guest for ZisK

This example demonstrates ZisK's **wasm32 + WASI guest machine**. Unlike the RISC-V examples, it
needs no custom Zisk toolchain: it is a plain Rust program built for the stock `wasm32-wasip1`
target. ZisK transpiles the `.wasm` module to its internal ISA at load time, exactly as it does
for RISC-V ELF guests.

The program reads an 8-byte little-endian `u64` `n` from stdin (default 10) and prints
`fib(n)`, with `fib(0) = 0` and `fib(1) = 1`.

## Build

With the stock wasm target (one-time `rustup target add wasm32-wasip1`):

```bash
cargo build --release --target wasm32-wasip1
# -> target/wasm32-wasip1/release/wasm-fibonacci.wasm
```

## Run

`ziskemu` detects the `.wasm` magic bytes, transpiles the module to a Zisk ROM and emulates it.
Guest stdout is mirrored into the public output region.

```bash
# default n = 10
ziskemu --elf target/wasm32-wasip1/release/wasm-fibonacci.wasm
# -> fib(10) = 55

# feed n = 90 on stdin: an 8-byte length prefix followed by the 8-byte value
printf '\x08\x00\x00\x00\x00\x00\x00\x00\x5a\x00\x00\x00\x00\x00\x00\x00' > /tmp/n.bin
ziskemu --elf target/wasm32-wasip1/release/wasm-fibonacci.wasm -i /tmp/n.bin
# -> fib(90) = 2880067194370816120
```

## Testing

The emulator integration test `emulator/tests/wasm_example.rs` builds this crate for
`wasm32-wasip1` and validates both runs end to end (transpile + emulate + check output). It is
skipped with a notice when the `wasm32-wasip1` rustup target is not installed.
