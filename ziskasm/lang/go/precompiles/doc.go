// Package precompiles binds the ZisK precompiles (the hardware-like accelerators
// the prover has a dedicated state machine for: Keccak-f, SHA-256's compression,
// elliptic-curve group operations, big-integer arithmetic, ...) for Go guests
// compiled to wasm.
//
// Each function is the raw precompile, one instruction on ZisK: it takes its
// operands as pointers to little-endian uint64 words, updates them in place and
// has no padding, sponge or protocol around it. The byte-oriented functions of
// the ZisK C headers (Keccak256, Secp256k1Ecrecover, the EVM word arithmetic,
// ...), which run the hand-written ZisK library routines, are the parent
// package zkvm, for TinyGo and standard Go alike; this package is for a guest
// with its own protocol around the primitive.
//
// The precompiles are host imports of the "env" module (see
// transpilers/wasm/src/precompiles.rs), declared with //go:wasmimport in
// precompiles_wasm.go; the ZisK wasm machine range- and alignment-checks every
// pointer before running the instruction, and a bad one traps. Operands must be
// 8-byte aligned; Go aligns uint64 arrays and slices, so a *[N]uint64 or a
// []uint64 element is always acceptable. On any GOARCH but wasm the package does
// not build: the precompiles only exist on ZisK.
//
// Names follow the ziskos syscalls (ziskos/entrypoint/src/syscalls) and the
// wasm import names: KeccakF is "keccak_f", Arith256Mod is "arith256_mod".
package precompiles
