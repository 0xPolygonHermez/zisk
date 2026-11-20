package zkvm

// Free-input calls (zkvm_fcall.h): unverified hints from the executor.
//
// A hint is a value that is expensive to compute and cheap to check, such as a
// field inverse or a square root. The proof does not cover how it was computed:
// check every result (for an inverse, that x * inv == 1) before relying on it, or a
// dishonest prover can feed the guest any value. Field elements are little-endian
// u64 limbs, and an Fp2 element is its real part followed by its imaginary part.

import (
	"errors"
	"unsafe"
)

// The hint IDs of zkvm_fcall.h.
const (
	FcallSecp256k1FpInv             = 1
	FcallSecp256k1FnInv             = 2
	FcallSecp256k1FpSqrt            = 3
	FcallSecp256k1GlvDecompose      = 4
	FcallSecp256r1FnInv             = 5
	FcallBn254FpInv                 = 6
	FcallBn254Fp2Inv                = 7
	FcallBn254TwistAddLineCoeffs    = 8
	FcallBn254TwistDblLineCoeffs    = 9
	FcallBls12381FpInv              = 10
	FcallBls12381FpSqrt             = 11
	FcallBls12381Fp2Inv             = 12
	FcallBls12381Fp2Sqrt            = 13
	FcallBls12381TwistAddLineCoeffs = 14
	FcallBls12381TwistDblLineCoeffs = 15
	FcallBinDecomp                  = 16
	FcallMsbPos256                  = 17
	FcallMsbPos384                  = 18
	FcallUint256Div                 = 19
	FcallUint256Inv                 = 20
	FcallUint256InvMod              = 21
	FcallBigintDiv                  = 22
	FcallEd25519FpSqrtRatio         = 26
	FcallMaxParams                  = 386
)

// ErrFcallRejected is returned by Fcall for an ID that is not a hint or more than
// FcallMaxParams parameters.
var ErrFcallRejected = errors.New("zkvm: fcall rejected")

// Fcall passes params and issues hint id; read its results with FcallGet or
// FcallRead, exactly as many as it produces (see zkvm_fcall.h): reading past the
// last one halts the executor.
func Fcall(id uint32, params []uint64) error {
	if fcall(id, unsafe.Pointer(unsafe.SliceData(params)), uintptr(len(params))) != 0 {
		return ErrFcallRejected
	}
	return nil
}

// FcallGet returns the next result word of the last hint issued.
func FcallGet() uint64 {
	return fcallGet()
}

// FcallRead reads the next len(results) result words of the last hint issued.
func FcallRead(results []uint64) {
	fcallRead(unsafe.Pointer(unsafe.SliceData(results)), uintptr(len(results)))
}

// hint issues the hint id, whose ID and parameter count are known to be valid.
func hint(id uint32, params ...uint64) {
	if Fcall(id, params) != nil {
		panic("zkvm: fcall rejected")
	}
}

func get4() (r [4]uint64) {
	FcallRead(r[:])
	return
}

func get6() (r [6]uint64) {
	FcallRead(r[:])
	return
}

func get8() (r [8]uint64) {
	FcallRead(r[:])
	return
}

func get12() (r [12]uint64) {
	FcallRead(r[:])
	return
}

func concat(parts ...[]uint64) []uint64 {
	var all []uint64
	for _, p := range parts {
		all = append(all, p...)
	}
	return all
}

// HintSecp256k1FpInv hints x^-1 in the secp256k1 base field.
func HintSecp256k1FpInv(x *[4]uint64) [4]uint64 {
	hint(FcallSecp256k1FpInv, x[:]...)
	return get4()
}

// HintSecp256k1FnInv hints x^-1 in the secp256k1 scalar field.
func HintSecp256k1FnInv(x *[4]uint64) [4]uint64 {
	hint(FcallSecp256k1FnInv, x[:]...)
	return get4()
}

// HintSecp256k1FpSqrt hints a square root of x in the secp256k1 base field, the odd
// one when odd is set. When x is not a square, isQR is false and root is a square
// root of x * NQR, for the field's fixed non-residue NQR.
func HintSecp256k1FpSqrt(x *[4]uint64, odd bool) (root [4]uint64, isQR bool) {
	parity := uint64(0)
	if odd {
		parity = 1
	}
	hint(FcallSecp256k1FpSqrt, x[0], x[1], x[2], x[3], parity)
	isQR = FcallGet() == 1
	return get4(), isQR
}

// HintSecp256k1GlvDecompose hints the GLV split of the scalar k: k = ±k1 ± k2·λ
// (mod n), with |k1|, |k2| < 2^128 and the signs neg1, neg2.
func HintSecp256k1GlvDecompose(k *[4]uint64) (k1, k2 [4]uint64, neg1, neg2 bool) {
	hint(FcallSecp256k1GlvDecompose, k[:]...)
	k1, k2 = get4(), get4()
	neg1, neg2 = FcallGet() == 1, FcallGet() == 1
	return
}

// HintSecp256r1FnInv hints x^-1 in the P-256 scalar field.
func HintSecp256r1FnInv(x *[4]uint64) [4]uint64 {
	hint(FcallSecp256r1FnInv, x[:]...)
	return get4()
}

// HintBn254FpInv hints x^-1 in the BN254 base field.
func HintBn254FpInv(x *[4]uint64) [4]uint64 {
	hint(FcallBn254FpInv, x[:]...)
	return get4()
}

// HintBn254Fp2Inv hints x^-1 in BN254's Fp2.
func HintBn254Fp2Inv(x *[8]uint64) [8]uint64 {
	hint(FcallBn254Fp2Inv, x[:]...)
	return get8()
}

// HintBn254TwistAddLineCoeffs hints the line through the BN254 twist points
// (x1, y1) and (x2, y2): λ = (y2 - y1) / (x2 - x1) and μ = y1 - λ·x1.
func HintBn254TwistAddLineCoeffs(x1, y1, x2, y2 *[8]uint64) (lambda, mu [8]uint64) {
	hint(FcallBn254TwistAddLineCoeffs, concat(x1[:], y1[:], x2[:], y2[:])...)
	return get8(), get8()
}

// HintBn254TwistDblLineCoeffs hints the tangent at the BN254 twist point (x, y),
// with y != 0: λ = 3x² / 2y and μ = y - λ·x.
func HintBn254TwistDblLineCoeffs(x, y *[8]uint64) (lambda, mu [8]uint64) {
	hint(FcallBn254TwistDblLineCoeffs, concat(x[:], y[:])...)
	return get8(), get8()
}

// HintBls12381FpInv hints x^-1 in the BLS12-381 base field.
func HintBls12381FpInv(x *[6]uint64) [6]uint64 {
	hint(FcallBls12381FpInv, x[:]...)
	return get6()
}

// HintBls12381FpSqrt hints a square root of x in the BLS12-381 base field, as
// HintSecp256k1FpSqrt does.
func HintBls12381FpSqrt(x *[6]uint64) (root [6]uint64, isQR bool) {
	hint(FcallBls12381FpSqrt, x[:]...)
	isQR = FcallGet() == 1
	return get6(), isQR
}

// HintBls12381Fp2Inv hints x^-1 in BLS12-381's Fp2.
func HintBls12381Fp2Inv(x *[12]uint64) [12]uint64 {
	hint(FcallBls12381Fp2Inv, x[:]...)
	return get12()
}

// HintBls12381Fp2Sqrt hints a square root of x in BLS12-381's Fp2, as
// HintSecp256k1FpSqrt does.
func HintBls12381Fp2Sqrt(x *[12]uint64) (root [12]uint64, isQR bool) {
	hint(FcallBls12381Fp2Sqrt, x[:]...)
	isQR = FcallGet() == 1
	return get12(), isQR
}

// HintBls12381TwistAddLineCoeffs hints the line through two BLS12-381 twist points,
// as HintBn254TwistAddLineCoeffs does.
func HintBls12381TwistAddLineCoeffs(x1, y1, x2, y2 *[12]uint64) (lambda, mu [12]uint64) {
	hint(FcallBls12381TwistAddLineCoeffs, concat(x1[:], y1[:], x2[:], y2[:])...)
	return get12(), get12()
}

// HintBls12381TwistDblLineCoeffs hints the tangent at a BLS12-381 twist point, as
// HintBn254TwistDblLineCoeffs does.
func HintBls12381TwistDblLineCoeffs(x, y *[12]uint64) (lambda, mu [12]uint64) {
	hint(FcallBls12381TwistDblLineCoeffs, concat(x[:], y[:])...)
	return get12(), get12()
}

// HintBinDecomp hints the binary digits of the little-endian limbs x, most
// significant first and without leading zeros ([0] for zero). It panics when x has
// more than FcallMaxParams-1 limbs.
func HintBinDecomp(x []uint64) []byte {
	hint(FcallBinDecomp, concat([]uint64{uint64(len(x))}, x)...)
	n := FcallGet()
	if n > 64*uint64(len(x)) {
		panic("zkvm: HintBinDecomp: too many digits")
	}
	bits := make([]byte, n)
	for i := range bits {
		bits[i] = byte(FcallGet())
	}
	return bits
}

// HintMsbPos256 hints the position of the most significant set bit across xs: the
// limb, and the bit within it, of the largest value. It panics when every value is
// 0.
func HintMsbPos256(xs ...*[4]uint64) (limb, bit int) {
	params := []uint64{uint64(len(xs))}
	zero := true
	for _, x := range xs {
		params = append(params, x[:]...)
		zero = zero && x[0]|x[1]|x[2]|x[3] == 0
	}
	if zero {
		panic("zkvm: HintMsbPos256: every value is zero")
	}
	hint(FcallMsbPos256, params...)
	return int(FcallGet()), int(FcallGet())
}

// HintMsbPos384 hints the position of the most significant set bit across the
// 384-bit x and y, as HintMsbPos256 does. It panics when both are 0.
func HintMsbPos384(x, y *[6]uint64) (limb, bit int) {
	var params [14]uint64
	copy(params[0:6], x[:])
	copy(params[8:14], y[:])
	zero := true
	for _, w := range params {
		zero = zero && w == 0
	}
	if zero {
		panic("zkvm: HintMsbPos384: both values are zero")
	}
	hint(FcallMsbPos384, params[:]...)
	return int(FcallGet()), int(FcallGet())
}

// HintUint256Div hints a / b and a % b. It panics when b is 0.
func HintUint256Div(a, b *[4]uint64) (q, r [4]uint64) {
	nonZero(b, "divisor")
	hint(FcallUint256Div, concat(a[:], b[:])...)
	return get4(), get4()
}

// HintUint256Inv hints a^-1 mod 2^256, and false when a is even.
func HintUint256Inv(a *[4]uint64) (inv [4]uint64, ok bool) {
	hint(FcallUint256Inv, a[:]...)
	ok = FcallGet() == 1
	return get4(), ok
}

// HintUint256InvMod hints a^-1 mod m. When it does not exist, ok is false and the
// hint is a witness of that instead: g = gcd(a, m) > 1, with a = g·qa and m = g·qm.
// It panics when m is 0.
func HintUint256InvMod(a, m *[4]uint64) (ok bool, invOrGcd, qa, qm [4]uint64) {
	nonZero(m, "modulus")
	hint(FcallUint256InvMod, concat(a[:], m[:])...)
	ok = FcallGet() == 1
	return ok, get4(), get4(), get4()
}

// HintBigintDiv hints a / b and a % b for little-endian limb arrays of any length.
// Each result is zero-padded to a multiple of 4 limbs (at least 4). It panics when
// b is 0 or there are more than FcallMaxParams-2 limbs in all.
func HintBigintDiv(a, b []uint64) (q, r []uint64) {
	zero := true
	for _, w := range b {
		zero = zero && w == 0
	}
	if zero {
		panic("zkvm: HintBigintDiv: the divisor is zero")
	}
	hint(FcallBigintDiv, concat([]uint64{uint64(len(a))}, a, []uint64{uint64(len(b))}, b)...)
	read := func(most int) []uint64 {
		n := FcallGet()
		if n > uint64(max(4, (most+3)/4*4)) {
			panic("zkvm: HintBigintDiv: too many limbs")
		}
		limbs := make([]uint64, n)
		FcallRead(limbs)
		return limbs
	}
	q = read(len(a))
	r = read(len(b))
	return
}

// HintEd25519FpSqrtRatio hints the even square root of u / v in the ed25519 base
// field. When u / v is not a square, isSquare is false and root is the even square
// root of 2u / v. When u is 0, root is 0 and isSquare true; when only v is 0, root
// is 0 and isSquare false.
func HintEd25519FpSqrtRatio(u, v *[4]uint64) (root [4]uint64, isSquare bool) {
	hint(FcallEd25519FpSqrtRatio, concat(u[:], v[:])...)
	isSquare = FcallGet() == 1
	return get4(), isSquare
}
