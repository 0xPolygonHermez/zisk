# ziskasm/lang/tinygo — TinyGo bindings for the ZisK assembly library

Go package `zkvm` for TinyGo guests compiled to WebAssembly (`-target=wasip1`). It
exposes the functions of the ZisK C headers in [`../c/include`](../c/include/) and runs
them with the hand-written `.zisk` routines under [`ziskasm/zisklib/`](../../zisklib/).
It is the TinyGo sibling of [`ziskasm/lang/c`](../c/) and [`ziskasm/lang/rust`](../rust/):
all three reach the same routines, and only the surface language differs.

## How it works

The package calls the C headers through cgo ([`calls_zisk.go`](calls_zisk.go)). TinyGo
compiles the cgo part with its own clang and calls C functions directly, so each call
costs what a Go function call costs. The functions stay undefined, and the wasm linker
turns each one into an import from module `env`, with the header's signature lowered by
the wasm32 C ABI (`uint64_t` is `i64`, everything else `i32`):

```
import env.zkvm_keccak256: (i32, i32, i32) -> i32
```

`wasm2rom` implements every such import (see
[`transpilers/wasm/src/zkvm.rs`](../../../transpilers/wasm/src/zkvm.rs)):

1. it loads the arguments into `a0..a7`;
2. it rebases each pointer onto the guest's linear memory and checks the range it
   covers;
3. it calls the routine a RISC-V guest reaches through the matching zkvmcall.
   `WriteLog` and the fcalls call no routine: the stub issues the instructions itself.

The library is merged into the ROM only when the module calls one of its routines.

Because the interface is the C headers themselves, the same imports serve a C guest
compiled to wasm with clang, or a Rust guest with an `extern "C"` block, with no change
in ZisK.

## Usage

```go
import "github.com/0xPolygonHermez/zisk/ziskasm/lang/tinygo"

digest := zkvm.Keccak256(data)
ok, err := zkvm.Secp256k1Verify(&msg, &sig, &pubkey)
sum := zkvm.U256Add(&a, &b)
inv := zkvm.HintBn254FpInv(&x) // unverified: check x * inv == 1 before using it
```

| Header | Family | Functions |
|---|---|---|
| `zkvm_accelerators.h` (EF) | Hashes | `Keccak256`, `KeccakF1600`, `Sha256`, `Ripemd160` |
| | secp256k1 / P-256 | `Secp256k1Verify`, `Secp256k1Ecrecover`, `Secp256r1Verify` |
| | Big integers | `Modexp` |
| | BN254 | `Bn254G1Add`, `Bn254G1Mul`, `Bn254Pairing` |
| | BLAKE2 | `Blake2f` |
| | BLS12-381 | `Bls12G1Add`, `Bls12G1Msm`, `Bls12G2Add`, `Bls12G2Msm`, `Bls12Pairing`, `Bls12MapFpToG1`, `Bls12MapFp2ToG2` |
| | KZG | `KzgPointEval` |
| `zkvm_u256.h` (EF) | EVM words, big-endian (`U256`) | `U256Add` ... `U256SignExtend`: all 27 operations, named after the C ones (`U256DivMod`, `U256SLt`, ...) |
| `zkvm_u256_le.h` (ZisK) | EVM words, little-endian limbs (`U256LE`) | `U256LEAdd` ... `U256LESignExtend`: the same 27 |
| `zkvm_random.h` (EF) | Host randomness | `RandomU64` |
| `zkvm_io.h` (EF) | Diagnostic log | `WriteLog` |
| `zkvm_zisklib.h` (ZisK) | 256-bit limb arithmetic | `ZisklibAdd`, `Inv256`, `OverflowingAdd256`, `OverflowingSub256`, `OverflowingMul256`, `DivRem256`, `ReduceMod256`, `AddMod256`, `MulMod256`, `InvMod256`, `PowMod256`, `OverflowingPow256` |
| | Signatures | `EcdsaVerifySecp256k1`, `EcdsaRecoverSecp256k1`, `SchnorrVerifySecp256k1`, `EcdsaVerifySecp256r1` |
| | Pairings and curves | `PairingCheckBn254`, `PairingCheckBls12381`, `MapToCurveG1Bls12381`, `MapToCurveG2Bls12381`, `HashToCurveG2Bls12381`, `BlsVerifyBls12381`, `VerifyKzgProofBls12381` |
| | Big integers | `ModexpU64` |
| `zkvm_fcall.h` (ZisK) | Raw fcalls | `Fcall`, `FcallGet`, `FcallRead`, and the hint IDs `FcallSecp256k1FpInv` ... |
| | Typed hints | `HintSecp256k1FpInv`, `HintSecp256k1FnInv`, `HintSecp256k1FpSqrt`, `HintSecp256k1GlvDecompose`, `HintSecp256r1FnInv`, `HintBn254FpInv`, `HintBn254Fp2Inv`, `HintBn254TwistAddLineCoeffs`, `HintBn254TwistDblLineCoeffs`, `HintBls12381FpInv`, `HintBls12381FpSqrt`, `HintBls12381Fp2Inv`, `HintBls12381Fp2Sqrt`, `HintBls12381TwistAddLineCoeffs`, `HintBls12381TwistDblLineCoeffs`, `HintBinDecomp`, `HintMsbPos256`, `HintMsbPos384`, `HintUint256Div`, `HintUint256Inv`, `HintUint256InvMod`, `HintBigintDiv`, `HintEd25519FpSqrtRatio` |

The layouts are the headers'. An accelerator that can fail returns `ErrFailed` for
`ZKVM_EFAIL`, an input that is not a valid point, signature or encoding. The U256
functions never fail. A zisklib routine that reports its own error status returns it as
a `StatusError`.

- **Alignment:** byte buffers need no particular alignment. A pointer to `uint64_t`
  limbs (`U256LE`, the zisklib and hint arrays) must be 8-byte aligned, which Go's
  `uint64` arrays always are; an unaligned one traps.
- **Bounds:** a pointer range outside the guest's memory traps.
- **Preconditions:** the zisklib routines do not validate every input, and a call that
  breaks a precondition ends the program or halts the emulator. The Go functions check
  the ones they can (a zero divisor or modulus, the modexp lengths, the pair counts)
  and panic instead.
- **Hints are not proven.** A hint comes from the executor, and the proof does not
  cover how it was computed: check every result against its definition before using
  it. `Fcall` rejects the executor's own fcalls (the input readiness, the keccak-f
  cache and the random seed) with `ErrFcallRejected`. Read exactly as many result words
  as the hint produces (see [`zkvm_fcall.h`](../c/include/zkvm_fcall.h)): reading past
  the last one halts the executor. The typed `Hint*` functions do this for you.
- **Randomness:** `RandomU64` draws from the seed the host provides with each run. A
  guest that calls it cannot run with hints.
- **Other targets:** off TinyGo `wasip1`, the functions exist only so the package
  compiles ([`calls_other.go`](calls_other.go)), and calling one panics.

A wasm guest reads its input from stdin and writes its public output to stdout (WASI),
so `read_input` and `write_output` of `zkvm_io.h` are not wrapped.

Build a guest with:

```bash
tinygo build -target=wasip1 -no-debug -opt=z -panic=trap -o guest.wasm .
```

Add `-scheduler=none` when nothing in the program starts a goroutine. Every TinyGo
module uses floating point, even an empty `main`, so running and proving it needs
ZisK built with the `float` feature. See the float limitation in
[`transpilers/wasm/README.md`](../../../transpilers/wasm/README.md#current-limitations).

## Example

[`example/`](example/) calls every function of the package, except `RandomU64`, and
checks each result against a known answer:

- the accelerators, against the vectors of the C example guests in
  [`../c/example`](../c/example/), with their failure cases: a bad signature, recovery
  id 2, an invalid pairing and a wrong KZG evaluation;
- the 27 U256 operations in both byte orders, on 52 cases that include the EVM edge
  cases (division by zero, `-2^255 / -1`, saturating shifts);
- the zisklib functions, with signatures, pairings and hashes to the curve from the
  same vectors, py_ecc (including RFC 9380's `abc` vector for hash-to-G2) and a BIP-340
  signature made by the generator;
- every hint, and the two IDs `Fcall` rejects.

The expected values come from independent Python models in `gen_vectors.py` (EVM
semantics, py_ecc, field arithmetic), not from the code under test. The example prints
`zkvm: 197 checks passed`. `RandomU64` is left out because a guest that draws random
values needs the host's seed and cannot run with hints; the `.wat` tests cover it.

```bash
cd example
python3 gen_vectors.py ../../c/example > vectors.go && gofmt -w vectors.go   # only to regenerate
tinygo build -target=wasip1 -no-debug -opt=z -scheduler=none -panic=trap -o zkvm-example.wasm .
ziskemu -e zkvm-example.wasm -c --write-log
```

Tested with TinyGo 0.44.0 and Go 1.27.3. The example runs in about 3,650,000 steps, and
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
- TinyGo 0.44 warns that the cgo snippets and the package have different target
  triples (`wasm32-unknown-wasip1` and `wasm32-unknown-wasi`) when it links them. The
  warning is harmless.
- `zkvm_mem.h` and `zkvm_evm.h` are not covered: wasm has its own bulk-memory
  operations, and the transpiler does not implement the JUMPDEST import.
