# ziskasm/lang/tinygo — TinyGo bindings for the ZisK assembly library

Go package `zkvm` for TinyGo guests compiled to WebAssembly (`-target=wasip1`). It
exposes the Ethereum Foundation zkVM accelerators of
[`zkvm_accelerators.h`](../c/include/zkvm_accelerators.h) and runs them with the
hand-written `.zisk` routines under [`ziskasm/zisklib/`](../../zisklib/). It is the
TinyGo sibling of [`ziskasm/lang/c`](../c/) and [`ziskasm/lang/rust`](../rust/): all
three reach the same routines, and only the surface language differs.

## How it works

The package calls the standard C header through cgo
([`calls_zisk.go`](calls_zisk.go)). TinyGo compiles the cgo part with its own clang
and calls C functions directly, so each call costs what a Go function call costs. The
accelerators stay undefined, and the wasm linker turns each one into an import from
module `env`, with the header's signature lowered by the wasm32 C ABI:

```
import env.zkvm_keccak256: (i32, i32, i32) -> i32
```

`wasm2rom` implements every such import (see
[`transpilers/wasm/src/zkvm.rs`](../../../transpilers/wasm/src/zkvm.rs)):

1. it loads the arguments into `a0..a6`;
2. it rebases each pointer onto the guest's linear memory and checks the range it
   covers;
3. it calls the routine a RISC-V guest reaches through the matching zkvmcall.

The library is merged into the ROM only when the module imports an accelerator.

Because the interface is the C header itself, the same imports serve a C guest
compiled to wasm with clang, or a Rust guest with an `extern "C"` block, with no
change in ZisK.

## Usage

```go
import "github.com/0xPolygonHermez/zisk/ziskasm/lang/tinygo"

digest := zkvm.Keccak256(data)
ok, err := zkvm.Secp256k1Verify(&msg, &sig, &pubkey)
```

The package covers every function of `zkvm_accelerators.h`:

| Family | Functions |
|---|---|
| Hashes | `Keccak256`, `KeccakF1600`, `Sha256`, `Ripemd160` |
| secp256k1 / P-256 | `Secp256k1Verify`, `Secp256k1Ecrecover`, `Secp256r1Verify` |
| Big integers | `Modexp` |
| BN254 | `Bn254G1Add`, `Bn254G1Mul`, `Bn254Pairing` |
| BLAKE2 | `Blake2f` |
| BLS12-381 | `Bls12G1Add`, `Bls12G1Msm`, `Bls12G2Add`, `Bls12G2Msm`, `Bls12Pairing`, `Bls12MapFpToG1`, `Bls12MapFp2ToG2` |
| KZG | `KzgPointEval` |

The byte layouts are the header's. A function that can fail returns `ErrFailed` for
`ZKVM_EFAIL`, an input that is not a valid point, signature or encoding.

- **Alignment:** buffers need no particular alignment.
- **Bounds:** a pointer range outside the guest's memory traps.
- **Other targets:** off TinyGo `wasip1`, the functions exist only so the package
  compiles ([`calls_other.go`](calls_other.go)), and calling one panics.

Build a guest with:

```bash
tinygo build -target=wasip1 -no-debug -opt=z -panic=trap -o guest.wasm .
```

Add `-scheduler=none` when nothing in the program starts a goroutine. Every TinyGo
module uses floating point, even an empty `main`, so running and proving it needs
ZisK built with the `float` feature. See the float limitation in
[`transpilers/wasm/README.md`](../../../transpilers/wasm/README.md#current-limitations).

## Example

[`example/`](example/) calls every accelerator and checks each result against a known
answer: the vectors of the C example guests in [`../c/example`](../c/example/),
generated into `vectors.go` by `gen_vectors.py`. It also checks the failure cases:

- a bad signature;
- recovery id 2;
- an invalid pairing;
- a wrong KZG evaluation.

It prints `zkvm: 30 checks passed`.

```bash
cd example
tinygo build -target=wasip1 -no-debug -opt=z -scheduler=none -panic=trap -o zkvm-example.wasm .
ziskemu -e zkvm-example.wasm -c
```

Tested with TinyGo 0.44.0 and Go 1.27.3. The example runs in about 605,000 steps, and
constraint verification passes on the emulator and ASM backends. The build is
reproducible: the result is committed as
[`emulator/tests/data/tinygo-zkvm-example.wasm`](../../../emulator/tests/data/tinygo-zkvm-example.wasm)
and run by `tinygo_example_passes_every_check` in
[`emulator/tests/wasm_zkvm.rs`](../../../emulator/tests/wasm_zkvm.rs). Those tests also
cover the import lowering without TinyGo, using hand-written `.wat` modules.

## Notes

- TinyGo's cgo does not expand `${SRCDIR}`, so `calls_zisk.go` uses
  `-I../c/include`, which TinyGo resolves against the package directory.
- `C.bool` names a macro in TinyGo's cgo, so the `bool*` arguments use `C._Bool`.
- Only `zkvm_accelerators.h` is covered. The other headers in `../c/include`
  (`zkvm_u256.h`, `zkvm_zisklib.h`, ...) would need their own rows in `zkvm.rs`.
  A wasm guest does its I/O through WASI.
