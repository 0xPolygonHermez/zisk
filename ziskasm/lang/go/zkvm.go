// Package zkvm gives Go guests compiled to wasm, with TinyGo (-target=wasip1) or
// standard Go (GOOS=wasip1 GOARCH=wasm), the EF zkVM accelerators of
// zkvm_accelerators.h, run by the ZisK library routines.
//
// It is the Go sibling of ziskasm/lang/c and ziskasm/lang/rust. Under TinyGo the
// package calls the standard C header through cgo; TinyGo leaves each function
// undefined, so the wasm linker turns it into an import from module "env"
// (env.zkvm_keccak256 and so on). Under standard Go, which has no cgo on wasm, the
// same imports are declared with //go:wasmimport. wasm2rom implements every such
// import with a call to the hand-written .zisk routine a RISC-V guest reaches
// through the matching zkvmcall (see transpilers/wasm/src/zkvm.rs). The guest
// therefore runs only under ZisK; on any other target the functions panic.
//
// Buffers need no particular alignment. Every pointer range is bounds-checked
// against the guest's linear memory, and an out-of-range one traps.
package zkvm

import (
	"errors"
	"unsafe"
)

// ErrFailed is returned when an accelerator reports ZKVM_EFAIL: an input that is
// not a valid point, signature or encoding.
var ErrFailed = errors.New("zkvm: accelerator failed")

// Bn254PairingPair is one (G1, G2) pair of a BN254 pairing check, as
// zkvm_bn254_pairing_pair (EIP-197 encoding).
type Bn254PairingPair struct {
	G1 [64]byte
	G2 [128]byte
}

// Bls12G1MsmPair is one (point, scalar) term of a BLS12-381 G1 multi-scalar
// multiplication, as zkvm_bls12_381_g1_msm_pair.
type Bls12G1MsmPair struct {
	Point  [96]byte
	Scalar [32]byte
}

// Bls12G2MsmPair is one (point, scalar) term of a BLS12-381 G2 multi-scalar
// multiplication, as zkvm_bls12_381_g2_msm_pair.
type Bls12G2MsmPair struct {
	Point  [192]byte
	Scalar [32]byte
}

// Bls12PairingPair is one (G1, G2) pair of a BLS12-381 pairing check, as
// zkvm_bls12_381_pairing_pair.
type Bls12PairingPair struct {
	G1 [96]byte
	G2 [192]byte
}

func check(status int32) error {
	if status != 0 {
		return ErrFailed
	}
	return nil
}

// mustSucceed is for the hashes and keccak-f, which never fail.
func mustSucceed(status int32) {
	if status != 0 {
		panic("zkvm: an infallible accelerator failed")
	}
}

func bytesPtr(b []byte) unsafe.Pointer {
	return unsafe.Pointer(unsafe.SliceData(b))
}

// Keccak256 returns the Keccak-256 digest of data (the Ethereum hash, not SHA3-256).
func Keccak256(data []byte) (digest [32]byte) {
	mustSucceed(keccak256(bytesPtr(data), uintptr(len(data)), unsafe.Pointer(&digest)))
	return
}

// KeccakF1600 applies the Keccak-f[1600] permutation to state in place.
func KeccakF1600(state *[25]uint64) {
	mustSucceed(keccakF1600(unsafe.Pointer(state)))
}

// Sha256 returns the SHA-256 digest of data.
func Sha256(data []byte) (digest [32]byte) {
	mustSucceed(sha256(bytesPtr(data), uintptr(len(data)), unsafe.Pointer(&digest)))
	return
}

// Ripemd160 returns the RIPEMD-160 digest of data.
func Ripemd160(data []byte) (digest [20]byte) {
	// The ABI right-aligns the 20-byte digest in 32 bytes, after 12 zero bytes.
	var out [32]byte
	mustSucceed(ripemd160(bytesPtr(data), uintptr(len(data)), unsafe.Pointer(&out)))
	copy(digest[:], out[12:])
	return
}

// Secp256k1Verify reports whether sig (r || s, big-endian) is a valid ECDSA
// signature of the 32-byte hash msg by pubkey (x || y, big-endian).
func Secp256k1Verify(msg *[32]byte, sig *[64]byte, pubkey *[64]byte) (bool, error) {
	var ok bool
	err := check(secp256k1Verify(unsafe.Pointer(msg), unsafe.Pointer(sig),
		unsafe.Pointer(pubkey), unsafe.Pointer(&ok)))
	return ok, err
}

// Secp256k1Ecrecover recovers the public key (x || y, big-endian) that signed the
// 32-byte hash msg with sig (r || s) and recovery id recid (0 or 1).
func Secp256k1Ecrecover(msg *[32]byte, sig *[64]byte, recid uint8) (pubkey [64]byte, err error) {
	err = check(secp256k1Ecrecover(unsafe.Pointer(msg), unsafe.Pointer(sig), recid,
		unsafe.Pointer(&pubkey)))
	return
}

// Secp256r1Verify reports whether sig is a valid P-256 ECDSA signature of the
// 32-byte hash msg by pubkey, all big-endian as in Secp256k1Verify.
func Secp256r1Verify(msg *[32]byte, sig *[64]byte, pubkey *[64]byte) (bool, error) {
	var ok bool
	err := check(secp256r1Verify(unsafe.Pointer(msg), unsafe.Pointer(sig),
		unsafe.Pointer(pubkey), unsafe.Pointer(&ok)))
	return ok, err
}

// Modexp returns base^exp mod modulus (EIP-198), all big-endian byte strings; the
// result is as long as modulus.
func Modexp(base, exp, modulus []byte) ([]byte, error) {
	out := make([]byte, len(modulus))
	err := check(modexp(bytesPtr(base), uintptr(len(base)), bytesPtr(exp), uintptr(len(exp)),
		bytesPtr(modulus), uintptr(len(modulus)), bytesPtr(out)))
	return out, err
}

// Bn254G1Add returns p1 + p2 on the BN254 G1 curve (EIP-196 encoding).
func Bn254G1Add(p1, p2 *[64]byte) (sum [64]byte, err error) {
	err = check(bn254G1Add(unsafe.Pointer(p1), unsafe.Pointer(p2), unsafe.Pointer(&sum)))
	return
}

// Bn254G1Mul returns scalar * point on the BN254 G1 curve (EIP-196 encoding).
func Bn254G1Mul(point *[64]byte, scalar *[32]byte) (product [64]byte, err error) {
	err = check(bn254G1Mul(unsafe.Pointer(point), unsafe.Pointer(scalar), unsafe.Pointer(&product)))
	return
}

// Bn254Pairing reports whether the product of the pairings of pairs is one (EIP-197).
func Bn254Pairing(pairs []Bn254PairingPair) (bool, error) {
	var ok bool
	err := check(bn254Pairing(unsafe.Pointer(unsafe.SliceData(pairs)), uintptr(len(pairs)),
		unsafe.Pointer(&ok)))
	return ok, err
}

// Blake2f runs the BLAKE2b compression function F (EIP-152) on the state h in
// place, with message block m, offset counters t and final-block flag final.
func Blake2f(rounds uint32, h *[64]byte, m *[128]byte, t *[16]byte, final bool) error {
	var f uint8
	if final {
		f = 1
	}
	return check(blake2f(rounds, unsafe.Pointer(h), unsafe.Pointer(m), unsafe.Pointer(t), f))
}

// KzgPointEval reports whether proof shows that the polynomial committed to by
// commitment evaluates to y at z (EIP-4844 point evaluation).
func KzgPointEval(commitment *[48]byte, z, y *[32]byte, proof *[48]byte) (bool, error) {
	var ok bool
	err := check(kzgPointEval(unsafe.Pointer(commitment), unsafe.Pointer(z), unsafe.Pointer(y),
		unsafe.Pointer(proof), unsafe.Pointer(&ok)))
	return ok, err
}

// Bls12G1Add returns p1 + p2 on the BLS12-381 G1 curve.
func Bls12G1Add(p1, p2 *[96]byte) (sum [96]byte, err error) {
	err = check(bls12G1Add(unsafe.Pointer(p1), unsafe.Pointer(p2), unsafe.Pointer(&sum)))
	return
}

// Bls12G1Msm returns the sum of scalar * point over pairs, on BLS12-381 G1.
func Bls12G1Msm(pairs []Bls12G1MsmPair) (sum [96]byte, err error) {
	err = check(bls12G1Msm(unsafe.Pointer(unsafe.SliceData(pairs)), uintptr(len(pairs)),
		unsafe.Pointer(&sum)))
	return
}

// Bls12G2Add returns p1 + p2 on the BLS12-381 G2 curve.
func Bls12G2Add(p1, p2 *[192]byte) (sum [192]byte, err error) {
	err = check(bls12G2Add(unsafe.Pointer(p1), unsafe.Pointer(p2), unsafe.Pointer(&sum)))
	return
}

// Bls12G2Msm returns the sum of scalar * point over pairs, on BLS12-381 G2.
func Bls12G2Msm(pairs []Bls12G2MsmPair) (sum [192]byte, err error) {
	err = check(bls12G2Msm(unsafe.Pointer(unsafe.SliceData(pairs)), uintptr(len(pairs)),
		unsafe.Pointer(&sum)))
	return
}

// Bls12Pairing reports whether the product of the pairings of pairs is one.
func Bls12Pairing(pairs []Bls12PairingPair) (bool, error) {
	var ok bool
	err := check(bls12Pairing(unsafe.Pointer(unsafe.SliceData(pairs)), uintptr(len(pairs)),
		unsafe.Pointer(&ok)))
	return ok, err
}

// Bls12MapFpToG1 maps a base-field element to a BLS12-381 G1 point (EIP-2537).
func Bls12MapFpToG1(element *[48]byte) (point [96]byte, err error) {
	err = check(bls12MapFpToG1(unsafe.Pointer(element), unsafe.Pointer(&point)))
	return
}

// Bls12MapFp2ToG2 maps an extension-field element to a BLS12-381 G2 point (EIP-2537).
func Bls12MapFp2ToG2(element *[96]byte) (point [192]byte, err error) {
	err = check(bls12MapFp2ToG2(unsafe.Pointer(element), unsafe.Pointer(&point)))
	return
}
