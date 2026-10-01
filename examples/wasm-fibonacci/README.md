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

## Zisk ROM / assembly output

`wasm2zisk` is the WebAssembly counterpart of `riscv2zisk`: it transpiles a `.wasm` guest and
saves the ROM as an x86-64 NASM file for the given generation method (`--gen=0` fast,
`1` minimal traces, `2` ROM histogram, `7` memory ops).

```bash
cargo run --release -p zisk-transpiler-wasm --bin wasm2zisk -- \
    target/wasm32-wasip1/release/wasm-fibonacci.wasm /tmp/fib.asm --gen=0
```

## Testing

The emulator integration test `emulator/tests/wasm_example.rs` builds this crate for
`wasm32-wasip1` and validates both runs end to end (transpile + emulate + check output). It is
skipped with a notice when the `wasm32-wasip1` rustup target is not installed.

## Go guests

Go's `GOOS=wasip1 GOARCH=wasm` output also runs, provided the emulator is built with the `float`
feature: the Go runtime executes f64 arithmetic even in integer-only programs, and the wasm
machine lowers f32/f64 onto the RISC-V soft-float library that this feature links into every ROM.

```bash
cargo build --release -p ziskemu --features float
GOOS=wasip1 GOARCH=wasm go build -o /tmp/guest.wasm .
ziskemu --elf /tmp/guest.wasm -i /tmp/input.bin   # 8-byte LE length, data, zero-padded to 8
```
