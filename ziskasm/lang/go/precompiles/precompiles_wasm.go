//go:build wasm

package precompiles

import "unsafe"

// Host imports: one per ZisK precompile, in the "env" module. The wasm machine
// lowers each to the precompile's own instruction.

//go:wasmimport env keccak_f
//go:noescape
func env_keccak_f(state unsafe.Pointer)

//go:wasmimport env sha256_f
//go:noescape
func env_sha256_f(state, block unsafe.Pointer)

//go:wasmimport env blake3f
//go:noescape
func env_blake3f(state, block unsafe.Pointer)

//go:wasmimport env blake2sf
//go:noescape
func env_blake2sf(state, block unsafe.Pointer)

//go:wasmimport env poseidon1
//go:noescape
func env_poseidon1(state unsafe.Pointer)

//go:wasmimport env poseidon2
//go:noescape
func env_poseidon2(state unsafe.Pointer)

//go:wasmimport env poseidon2_koalabear
//go:noescape
func env_poseidon2_koalabear(state unsafe.Pointer)

//go:wasmimport env arith256
//go:noescape
func env_arith256(block unsafe.Pointer)

//go:wasmimport env arith256_mod
//go:noescape
func env_arith256_mod(block unsafe.Pointer)

//go:wasmimport env arith384_mod
//go:noescape
func env_arith384_mod(block unsafe.Pointer)

//go:wasmimport env add256
//go:noescape
func env_add256(block unsafe.Pointer) uint64

//go:wasmimport env secp256k1_add
//go:noescape
func env_secp256k1_add(p1, p2 unsafe.Pointer)

//go:wasmimport env secp256k1_dbl
//go:noescape
func env_secp256k1_dbl(p unsafe.Pointer)

//go:wasmimport env secp256r1_add
//go:noescape
func env_secp256r1_add(p1, p2 unsafe.Pointer)

//go:wasmimport env secp256r1_dbl
//go:noescape
func env_secp256r1_dbl(p unsafe.Pointer)

//go:wasmimport env bn254_curve_add
//go:noescape
func env_bn254_curve_add(p1, p2 unsafe.Pointer)

//go:wasmimport env bn254_curve_dbl
//go:noescape
func env_bn254_curve_dbl(p unsafe.Pointer)

//go:wasmimport env bn254_complex_add
//go:noescape
func env_bn254_complex_add(a, b unsafe.Pointer)

//go:wasmimport env bn254_complex_sub
//go:noescape
func env_bn254_complex_sub(a, b unsafe.Pointer)

//go:wasmimport env bn254_complex_mul
//go:noescape
func env_bn254_complex_mul(a, b unsafe.Pointer)

//go:wasmimport env bls12_381_curve_add
//go:noescape
func env_bls12_381_curve_add(p1, p2 unsafe.Pointer)

//go:wasmimport env bls12_381_curve_dbl
//go:noescape
func env_bls12_381_curve_dbl(p unsafe.Pointer)

//go:wasmimport env bls12_381_complex_add
//go:noescape
func env_bls12_381_complex_add(a, b unsafe.Pointer)

//go:wasmimport env bls12_381_complex_sub
//go:noescape
func env_bls12_381_complex_sub(a, b unsafe.Pointer)

//go:wasmimport env bls12_381_complex_mul
//go:noescape
func env_bls12_381_complex_mul(a, b unsafe.Pointer)

//go:wasmimport env babyjubjub_add
//go:noescape
func env_babyjubjub_add(p1, p2 unsafe.Pointer)

//go:wasmimport env ed25519_add
//go:noescape
func env_ed25519_add(p1, p2 unsafe.Pointer)

// addr is the linear-memory address of p, as the parameter blocks carry it.
// Go's wasm garbage collector does not move objects, so the address stays
// valid for the duration of the call.
func addr(p unsafe.Pointer) uint64 { return uint64(uintptr(p)) }

// -- hashes --------------------------------------------------------------

// KeccakF applies the Keccak-f[1600] permutation to state, in place.
func KeccakF(state *[25]uint64) { env_keccak_f(unsafe.Pointer(state)) }

// Sha256F applies the SHA-256 compression function: state (h0..h7, two per
// word, h0 in the low half of word 0) is updated with the 64-byte block.
func Sha256F(state *[4]uint64, block *[8]uint64) {
	env_sha256_f(unsafe.Pointer(state), unsafe.Pointer(block))
}

// Blake3F applies the BLAKE3 compression function to state with the block.
func Blake3F(state *[8]uint64, block *[8]uint64) {
	env_blake3f(unsafe.Pointer(state), unsafe.Pointer(block))
}

// Blake2sF applies the BLAKE2s compression function to state with the block.
func Blake2sF(state *[8]uint64, block *[8]uint64) {
	env_blake2sf(unsafe.Pointer(state), unsafe.Pointer(block))
}

// Poseidon1 applies the Poseidon permutation to the 16-element state in place.
func Poseidon1(state *[16]uint64) { env_poseidon1(unsafe.Pointer(state)) }

// Poseidon2 applies the Poseidon2 permutation to the 16-element state in place.
func Poseidon2(state *[16]uint64) { env_poseidon2(unsafe.Pointer(state)) }

// Poseidon2Koalabear applies the Poseidon2 permutation over the KoalaBear
// field to the 8-element state in place.
func Poseidon2Koalabear(state *[8]uint64) { env_poseidon2_koalabear(unsafe.Pointer(state)) }

// -- big-integer arithmetic ----------------------------------------------
//
// 256-bit values are 4 little-endian uint64 limbs, 384-bit values 6.

// Arith256 computes dl:dh = a*b + c (dl the low 256 bits, dh the high).
func Arith256(a, b, c, dl, dh *[4]uint64) {
	block := [5]uint64{
		addr(unsafe.Pointer(a)), addr(unsafe.Pointer(b)), addr(unsafe.Pointer(c)),
		addr(unsafe.Pointer(dl)), addr(unsafe.Pointer(dh)),
	}
	env_arith256(unsafe.Pointer(&block))
}

// Arith256Mod computes d = (a*b + c) mod m.
func Arith256Mod(a, b, c, m, d *[4]uint64) {
	block := [5]uint64{
		addr(unsafe.Pointer(a)), addr(unsafe.Pointer(b)), addr(unsafe.Pointer(c)),
		addr(unsafe.Pointer(m)), addr(unsafe.Pointer(d)),
	}
	env_arith256_mod(unsafe.Pointer(&block))
}

// Arith384Mod computes d = (a*b + c) mod m on 384-bit values.
func Arith384Mod(a, b, c, m, d *[6]uint64) {
	block := [5]uint64{
		addr(unsafe.Pointer(a)), addr(unsafe.Pointer(b)), addr(unsafe.Pointer(c)),
		addr(unsafe.Pointer(m)), addr(unsafe.Pointer(d)),
	}
	env_arith384_mod(unsafe.Pointer(&block))
}

// Add256 computes c = a + b + cin (cin is 0 or 1) and returns the carry-out.
func Add256(a, b *[4]uint64, cin uint64, c *[4]uint64) uint64 {
	block := [4]uint64{addr(unsafe.Pointer(a)), addr(unsafe.Pointer(b)), cin, addr(unsafe.Pointer(c))}
	return env_add256(unsafe.Pointer(&block))
}

// -- elliptic curves -----------------------------------------------------
//
// Affine points are x then y, each a little-endian limb array: 4 limbs for the
// 256-bit curves ([8]uint64 per point), 6 for BLS12-381 ([12]uint64). Complex
// (quadratic extension) elements are the real then the imaginary part.

// Secp256k1Add sets p1 = p1 + p2 on secp256k1.
func Secp256k1Add(p1, p2 *[8]uint64) { env_secp256k1_add(unsafe.Pointer(p1), unsafe.Pointer(p2)) }

// Secp256k1Dbl sets p = 2p on secp256k1.
func Secp256k1Dbl(p *[8]uint64) { env_secp256k1_dbl(unsafe.Pointer(p)) }

// Secp256r1Add sets p1 = p1 + p2 on secp256r1 (P-256).
func Secp256r1Add(p1, p2 *[8]uint64) { env_secp256r1_add(unsafe.Pointer(p1), unsafe.Pointer(p2)) }

// Secp256r1Dbl sets p = 2p on secp256r1 (P-256).
func Secp256r1Dbl(p *[8]uint64) { env_secp256r1_dbl(unsafe.Pointer(p)) }

// Bn254CurveAdd sets p1 = p1 + p2 on BN254's G1.
func Bn254CurveAdd(p1, p2 *[8]uint64) { env_bn254_curve_add(unsafe.Pointer(p1), unsafe.Pointer(p2)) }

// Bn254CurveDbl sets p = 2p on BN254's G1.
func Bn254CurveDbl(p *[8]uint64) { env_bn254_curve_dbl(unsafe.Pointer(p)) }

// Bn254ComplexAdd sets a = a + b in BN254's Fp2.
func Bn254ComplexAdd(a, b *[8]uint64) { env_bn254_complex_add(unsafe.Pointer(a), unsafe.Pointer(b)) }

// Bn254ComplexSub sets a = a - b in BN254's Fp2.
func Bn254ComplexSub(a, b *[8]uint64) { env_bn254_complex_sub(unsafe.Pointer(a), unsafe.Pointer(b)) }

// Bn254ComplexMul sets a = a * b in BN254's Fp2.
func Bn254ComplexMul(a, b *[8]uint64) { env_bn254_complex_mul(unsafe.Pointer(a), unsafe.Pointer(b)) }

// Bls12381CurveAdd sets p1 = p1 + p2 on BLS12-381's G1.
func Bls12381CurveAdd(p1, p2 *[12]uint64) {
	env_bls12_381_curve_add(unsafe.Pointer(p1), unsafe.Pointer(p2))
}

// Bls12381CurveDbl sets p = 2p on BLS12-381's G1.
func Bls12381CurveDbl(p *[12]uint64) { env_bls12_381_curve_dbl(unsafe.Pointer(p)) }

// Bls12381ComplexAdd sets a = a + b in BLS12-381's Fp2.
func Bls12381ComplexAdd(a, b *[12]uint64) {
	env_bls12_381_complex_add(unsafe.Pointer(a), unsafe.Pointer(b))
}

// Bls12381ComplexSub sets a = a - b in BLS12-381's Fp2.
func Bls12381ComplexSub(a, b *[12]uint64) {
	env_bls12_381_complex_sub(unsafe.Pointer(a), unsafe.Pointer(b))
}

// Bls12381ComplexMul sets a = a * b in BLS12-381's Fp2.
func Bls12381ComplexMul(a, b *[12]uint64) {
	env_bls12_381_complex_mul(unsafe.Pointer(a), unsafe.Pointer(b))
}

// BabyjubjubAdd sets p1 = p1 + p2 on Baby Jubjub.
func BabyjubjubAdd(p1, p2 *[8]uint64) { env_babyjubjub_add(unsafe.Pointer(p1), unsafe.Pointer(p2)) }

// Ed25519Add sets p1 = p1 + p2 on Ed25519.
func Ed25519Add(p1, p2 *[8]uint64) { env_ed25519_add(unsafe.Pointer(p1), unsafe.Pointer(p2)) }
