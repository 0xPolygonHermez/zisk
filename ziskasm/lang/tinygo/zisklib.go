package zkvm

// The ZisK library functions outside the EF standard (zkvm_zisklib.h): 256-bit
// arithmetic on little-endian u64 limbs, secp256k1 and P-256 signatures, BN254 and
// BLS12-381 pairings, maps and hashes to curves, BLS and KZG verification, and
// arbitrary-precision modexp.
//
// These are raw routines that do not validate every input: a call that breaks a
// precondition ends the program or halts the emulator, and either way no proof of
// the run exists. The functions below check the preconditions they can (a zero
// divisor or modulus, the modexp lengths, the pair counts) and panic instead.

import "unsafe"

// StatusError is a ZisK library routine's own error status (see zkvm_zisklib.h).
type StatusError uint64

func (e StatusError) Error() string {
	return "zkvm: ZisK library status " + itoa(uint64(e))
}

func itoa(v uint64) string {
	var b [20]byte
	i := len(b)
	for {
		i--
		b[i] = byte('0' + v%10)
		v /= 10
		if v == 0 {
			return string(b[i:])
		}
	}
}

func limbsPtr(l *[4]uint64) unsafe.Pointer { return unsafe.Pointer(l) }

func nonZero(x *[4]uint64, what string) {
	if x[0]|x[1]|x[2]|x[3] == 0 {
		panic("zkvm: " + what + " is zero")
	}
}

// ZisklibAdd returns a + b: the smallest library call, a demo.
func ZisklibAdd(a, b uint64) uint64 {
	return zisklibAdd(a, b)
}

// Inv256 returns a^-1 mod 2^256, and false when a is even (not invertible).
func Inv256(a *[4]uint64) (inv [4]uint64, ok bool) {
	ok = zisklibInv256(limbsPtr(a), limbsPtr(&inv)) == 1
	return
}

// OverflowingAdd256 returns a + b mod 2^256 and the carry out.
func OverflowingAdd256(a, b *[4]uint64) (sum [4]uint64, carry bool) {
	carry = zisklibOverflowingAdd256(limbsPtr(a), limbsPtr(b), limbsPtr(&sum)) != 0
	return
}

// OverflowingSub256 returns a - b mod 2^256 and the borrow out.
func OverflowingSub256(a, b *[4]uint64) (diff [4]uint64, borrow bool) {
	borrow = zisklibOverflowingSub256(limbsPtr(a), limbsPtr(b), limbsPtr(&diff)) != 0
	return
}

// OverflowingMul256 returns the low 256 bits of a * b, and whether the product
// overflowed them.
func OverflowingMul256(a, b *[4]uint64) (product [4]uint64, overflow bool) {
	overflow = zisklibOverflowingMul256(limbsPtr(a), limbsPtr(b), limbsPtr(&product)) != 0
	return
}

// DivRem256 returns a / b and a % b. It panics when b is 0.
func DivRem256(a, b *[4]uint64) (q, r [4]uint64) {
	nonZero(b, "divisor")
	zisklibDivRem256(limbsPtr(a), limbsPtr(b), limbsPtr(&q), limbsPtr(&r))
	return
}

// ReduceMod256 returns a mod m. It panics when m is 0.
func ReduceMod256(a, m *[4]uint64) (r [4]uint64) {
	nonZero(m, "modulus")
	zisklibReduceMod256(limbsPtr(a), limbsPtr(m), limbsPtr(&r))
	return
}

// AddMod256 returns (a + b) mod m. It panics when m is 0.
func AddMod256(a, b, m *[4]uint64) (r [4]uint64) {
	nonZero(m, "modulus")
	zisklibAddMod256(limbsPtr(a), limbsPtr(b), limbsPtr(m), limbsPtr(&r))
	return
}

// MulMod256 returns (a * b) mod m. It panics when m is 0.
func MulMod256(a, b, m *[4]uint64) (r [4]uint64) {
	nonZero(m, "modulus")
	zisklibMulMod256(limbsPtr(a), limbsPtr(b), limbsPtr(m), limbsPtr(&r))
	return
}

// InvMod256 returns a^-1 mod m, and false when it does not exist. It panics when
// m is 0.
func InvMod256(a, m *[4]uint64) (inv [4]uint64, ok bool) {
	nonZero(m, "modulus")
	ok = zisklibInvMod256(limbsPtr(a), limbsPtr(m), limbsPtr(&inv)) == 1
	return
}

// PowMod256 returns base^exp mod m. It needs m >= 2: it panics when m is 0, and
// with m == 1 and exp == 0 it returns 1, not 0.
func PowMod256(base, exp, m *[4]uint64) (r [4]uint64) {
	nonZero(m, "modulus")
	zisklibPowMod256(limbsPtr(base), limbsPtr(exp), limbsPtr(m), limbsPtr(&r))
	return
}

// OverflowingPow256 returns the low 256 bits of base^exp, and whether the power
// overflowed them.
func OverflowingPow256(base, exp *[4]uint64) (r [4]uint64, overflow bool) {
	overflow = zisklibOverflowingPow256(limbsPtr(base), limbsPtr(exp), limbsPtr(&r)) != 0
	return
}

// EcdsaVerifySecp256k1 reports whether (r, s) is a valid secp256k1 signature of
// the message hash z under the public key pk (x then y).
func EcdsaVerifySecp256k1(pk *[8]uint64, z, r, s *[4]uint64) bool {
	return zisklibEcdsaVerifySecp256k1(unsafe.Pointer(pk), limbsPtr(z), limbsPtr(r), limbsPtr(s)) == 1
}

// EcdsaRecoverSecp256k1 recovers the public key (x then y) that signed the message
// hash z with (r, s) and the recovery id recid, which must be 0 or 1 (ids 2 and 3
// return StatusError(3)).
func EcdsaRecoverSecp256k1(r, s, z *[4]uint64, recid uint64) (pk [8]uint64, err error) {
	status := zisklibEcdsaRecoverSecp256k1(limbsPtr(r), limbsPtr(s), limbsPtr(z), recid,
		unsafe.Pointer(&pk))
	if status != 0 {
		err = StatusError(status)
	}
	return
}

// SchnorrVerifySecp256k1 reports whether (r, s) is a valid BIP-340 signature of msg
// under the public key with x coordinate pkX.
func SchnorrVerifySecp256k1(pkX, r, s *[4]uint64, msg []byte) bool {
	return zisklibSchnorrVerifySecp256k1(limbsPtr(pkX), limbsPtr(r), limbsPtr(s), bytesPtr(msg),
		uint64(len(msg))) == 1
}

// EcdsaVerifySecp256r1 reports whether (r, s) is a valid P-256 signature of the
// message hash z under the public key pk (x then y).
func EcdsaVerifySecp256r1(pk *[8]uint64, z, r, s *[4]uint64) bool {
	return zisklibEcdsaVerifySecp256r1(unsafe.Pointer(pk), limbsPtr(z), limbsPtr(r), limbsPtr(s)) == 1
}

// pairingStatus turns a pairing check's status (0 accept, 1 reject, 2.. invalid
// input) into its result.
func pairingStatus(status uint64) (bool, error) {
	switch status {
	case 0:
		return true, nil
	case 1:
		return false, nil
	}
	return false, StatusError(status)
}

// PairingCheckBn254 reports whether the product of the pairings e(g1[i], g2[i]) is
// 1. Each G1 point is x then y, each G2 point x then y in Fp2 (real part, then
// imaginary). An invalid point returns a StatusError: 2 or 3 for a G1 point that is
// not canonical or not on the curve, 4, 5 or 6 for a G2 point that is not
// canonical, on the curve or in the subgroup. It panics when the slices differ in
// length.
func PairingCheckBn254(g1 [][8]uint64, g2 [][16]uint64) (bool, error) {
	if len(g1) != len(g2) {
		panic("zkvm: PairingCheckBn254: as many G1 as G2 points are needed")
	}
	return pairingStatus(zisklibPairingCheckBn254(unsafe.Pointer(unsafe.SliceData(g1)),
		unsafe.Pointer(unsafe.SliceData(g2)), uint64(len(g1))))
}

// PairingCheckBls12381 reports whether the product of the pairings e(g1[i], g2[i])
// is 1, with the BLS12-381 points laid out as for PairingCheckBn254. An invalid
// point returns a StatusError from 2 to 7. It panics when the slices differ in
// length.
func PairingCheckBls12381(g1 [][12]uint64, g2 [][24]uint64) (bool, error) {
	if len(g1) != len(g2) {
		panic("zkvm: PairingCheckBls12381: as many G1 as G2 points are needed")
	}
	return pairingStatus(zisklibPairingCheckBls12381(unsafe.Pointer(unsafe.SliceData(g1)),
		unsafe.Pointer(unsafe.SliceData(g2)), uint64(len(g1))))
}

// MapToCurveG1Bls12381 maps the field element u to a G1 point (x then y). It
// returns a StatusError when u is not below the field modulus.
func MapToCurveG1Bls12381(u *[6]uint64) (point [12]uint64, err error) {
	if status := zisklibMapToCurveG1Bls12381(unsafe.Pointer(u), unsafe.Pointer(&point)); status != 0 {
		err = StatusError(status)
	}
	return
}

// MapToCurveG2Bls12381 maps the Fp2 element u to a G2 point (x then y). It
// returns a StatusError when u is not canonical.
func MapToCurveG2Bls12381(u *[12]uint64) (point [24]uint64, err error) {
	if status := zisklibMapToCurveG2Bls12381(unsafe.Pointer(u), unsafe.Pointer(&point)); status != 0 {
		err = StatusError(status)
	}
	return
}

// HashToCurveG2Bls12381 hashes msg to a G2 point (x then y) with the domain
// separation tag dst.
func HashToCurveG2Bls12381(msg, dst []byte) (point [24]uint64) {
	zisklibHashToCurveG2Bls12381(bytesPtr(msg), uint64(len(msg)), bytesPtr(dst), uint64(len(dst)),
		unsafe.Pointer(&point))
	return
}

// BlsVerifyBls12381 reports whether sig (a compressed G2 point) is a valid BLS
// signature of msg under the public key pk (a compressed G1 point).
func BlsVerifyBls12381(pk *[48]byte, msg []byte, sig *[96]byte) bool {
	return zisklibBlsVerifyBls12381(unsafe.Pointer(pk), bytesPtr(msg), uint64(len(msg)),
		unsafe.Pointer(sig)) == 1
}

// VerifyKzgProofBls12381 reports whether proof shows that the polynomial committed
// to by commitment evaluates to y at z (EIP-4844; commitment and proof are
// compressed G1 points).
func VerifyKzgProofBls12381(z, y *[32]byte, commitment, proof *[48]byte) bool {
	return zisklibVerifyKzgProofBls12381(unsafe.Pointer(z), unsafe.Pointer(y),
		unsafe.Pointer(commitment), unsafe.Pointer(proof)) == 1
}

// ModexpU64 returns base^exp mod modulus, each a little-endian u64 limb array of 1
// to 132 limbs; the result has at least 4 limbs. It panics when a length is out of
// range.
func ModexpU64(base, exp, modulus []uint64) []uint64 {
	for _, n := range [3]int{len(base), len(exp), len(modulus)} {
		if n < 1 || n > 132 {
			panic("zkvm: ModexpU64: every operand needs 1 to 132 limbs")
		}
	}
	result := make([]uint64, max(4, (len(modulus)+3)/4*4))
	n := zisklibModexpU64(unsafe.Pointer(unsafe.SliceData(base)), uintptr(len(base)),
		unsafe.Pointer(unsafe.SliceData(exp)), uintptr(len(exp)),
		unsafe.Pointer(unsafe.SliceData(modulus)), uintptr(len(modulus)),
		unsafe.Pointer(unsafe.SliceData(result)))
	return result[:n]
}
