package zkvm

// The EVM word arithmetic of zkvm_u256.h (the EF standard, big-endian bytes) and of
// zkvm_u256_le.h (ZisK's little-endian variant, u64 limbs). Both have the EVM
// semantics: division by zero and a zero modulus give 0, shifts by 256 or more
// saturate, and the result may alias an input. None of them fails.

import "unsafe"

// U256 is an EVM word as zkvm_u256 keeps it: 32 bytes, most significant first.
type U256 [32]byte

// U256LE is an EVM word as zkvm_u256_le keeps it: four 64-bit limbs, least
// significant first, the layout of revm's ruint and evmone's intx.
type U256LE [4]uint64

// U256Div returns a / b, or 0 when b is 0.
func U256Div(a, b *U256) (q U256) {
	u256Div(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&q))
	return
}

// U256Mod returns a % b, or 0 when b is 0.
func U256Mod(a, b *U256) (r U256) {
	u256Mod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256DivMod returns a / b and a % b, or 0 and 0 when b is 0.
func U256DivMod(a, b *U256) (q, r U256) {
	u256DivMod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&q), unsafe.Pointer(&r))
	return
}

// U256SDiv returns a / b as two's-complement integers (EVM SDIV), or 0 when b is 0.
func U256SDiv(a, b *U256) (q U256) {
	u256SDiv(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&q))
	return
}

// U256SMod returns a % b as two's-complement integers, with the sign of a (EVM SMOD), or 0 when b is 0.
func U256SMod(a, b *U256) (r U256) {
	u256SMod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256SDivMod returns SDiv and SMod of a and b.
func U256SDivMod(a, b *U256) (q, r U256) {
	u256SDivMod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&q), unsafe.Pointer(&r))
	return
}

// U256Add returns a + b mod 2^256.
func U256Add(a, b *U256) (r U256) {
	u256Add(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256Sub returns a - b mod 2^256.
func U256Sub(a, b *U256) (r U256) {
	u256Sub(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256Mul returns a * b mod 2^256.
func U256Mul(a, b *U256) (r U256) {
	u256Mul(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256AddMod returns (a + b) mod n, without overflow, or 0 when n is 0.
func U256AddMod(a, b, n *U256) (r U256) {
	u256AddMod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(n), unsafe.Pointer(&r))
	return
}

// U256MulMod returns (a * b) mod n, without overflow, or 0 when n is 0.
func U256MulMod(a, b, n *U256) (r U256) {
	u256MulMod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(n), unsafe.Pointer(&r))
	return
}

// U256Exp returns base^exponent mod 2^256.
func U256Exp(base, exponent *U256) (r U256) {
	u256Exp(unsafe.Pointer(base), unsafe.Pointer(exponent), unsafe.Pointer(&r))
	return
}

// U256Lt returns 1 if a < b, else 0.
func U256Lt(a, b *U256) (r U256) {
	u256Lt(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256Gt returns 1 if a > b, else 0.
func U256Gt(a, b *U256) (r U256) {
	u256Gt(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256SLt returns 1 if a < b as two's-complement integers, else 0.
func U256SLt(a, b *U256) (r U256) {
	u256SLt(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256SGt returns 1 if a > b as two's-complement integers, else 0.
func U256SGt(a, b *U256) (r U256) {
	u256SGt(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256Eq returns 1 if a == b, else 0.
func U256Eq(a, b *U256) (r U256) {
	u256Eq(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256IsZero returns 1 if a is 0, else 0.
func U256IsZero(a *U256) (r U256) {
	u256IsZero(unsafe.Pointer(a), unsafe.Pointer(&r))
	return
}

// U256And returns a & b.
func U256And(a, b *U256) (r U256) {
	u256And(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256Or returns a | b.
func U256Or(a, b *U256) (r U256) {
	u256Or(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256Xor returns a ^ b.
func U256Xor(a, b *U256) (r U256) {
	u256Xor(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256Not returns ^a.
func U256Not(a *U256) (r U256) {
	u256Not(unsafe.Pointer(a), unsafe.Pointer(&r))
	return
}

// U256Byte returns byte i of a, counting from the most significant (EVM BYTE), or 0 when i >= 32.
func U256Byte(i, a *U256) (r U256) {
	u256Byte(unsafe.Pointer(i), unsafe.Pointer(a), unsafe.Pointer(&r))
	return
}

// U256Shl returns value << shift, or 0 when shift >= 256.
func U256Shl(shift, value *U256) (r U256) {
	u256Shl(unsafe.Pointer(shift), unsafe.Pointer(value), unsafe.Pointer(&r))
	return
}

// U256Shr returns value >> shift (logical), or 0 when shift >= 256.
func U256Shr(shift, value *U256) (r U256) {
	u256Shr(unsafe.Pointer(shift), unsafe.Pointer(value), unsafe.Pointer(&r))
	return
}

// U256Sar returns value >> shift (arithmetic).
func U256Sar(shift, value *U256) (r U256) {
	u256Sar(unsafe.Pointer(shift), unsafe.Pointer(value), unsafe.Pointer(&r))
	return
}

// U256SignExtend returns value sign-extended from byte b, counting from the least significant (EVM SIGNEXTEND); value itself when b >= 31.
func U256SignExtend(b, value *U256) (r U256) {
	u256SignExtend(unsafe.Pointer(b), unsafe.Pointer(value), unsafe.Pointer(&r))
	return
}

// U256LEDiv returns a / b, or 0 when b is 0.
func U256LEDiv(a, b *U256LE) (q U256LE) {
	u256leDiv(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&q))
	return
}

// U256LEMod returns a % b, or 0 when b is 0.
func U256LEMod(a, b *U256LE) (r U256LE) {
	u256leMod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LEDivMod returns a / b and a % b, or 0 and 0 when b is 0.
func U256LEDivMod(a, b *U256LE) (q, r U256LE) {
	u256leDivMod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&q), unsafe.Pointer(&r))
	return
}

// U256LESDiv returns a / b as two's-complement integers (EVM SDIV), or 0 when b is 0.
func U256LESDiv(a, b *U256LE) (q U256LE) {
	u256leSDiv(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&q))
	return
}

// U256LESMod returns a % b as two's-complement integers, with the sign of a (EVM SMOD), or 0 when b is 0.
func U256LESMod(a, b *U256LE) (r U256LE) {
	u256leSMod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LESDivMod returns SDiv and SMod of a and b.
func U256LESDivMod(a, b *U256LE) (q, r U256LE) {
	u256leSDivMod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&q), unsafe.Pointer(&r))
	return
}

// U256LEAdd returns a + b mod 2^256.
func U256LEAdd(a, b *U256LE) (r U256LE) {
	u256leAdd(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LESub returns a - b mod 2^256.
func U256LESub(a, b *U256LE) (r U256LE) {
	u256leSub(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LEMul returns a * b mod 2^256.
func U256LEMul(a, b *U256LE) (r U256LE) {
	u256leMul(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LEAddMod returns (a + b) mod n, without overflow, or 0 when n is 0.
func U256LEAddMod(a, b, n *U256LE) (r U256LE) {
	u256leAddMod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(n), unsafe.Pointer(&r))
	return
}

// U256LEMulMod returns (a * b) mod n, without overflow, or 0 when n is 0.
func U256LEMulMod(a, b, n *U256LE) (r U256LE) {
	u256leMulMod(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(n), unsafe.Pointer(&r))
	return
}

// U256LEExp returns base^exponent mod 2^256.
func U256LEExp(base, exponent *U256LE) (r U256LE) {
	u256leExp(unsafe.Pointer(base), unsafe.Pointer(exponent), unsafe.Pointer(&r))
	return
}

// U256LELt returns 1 if a < b, else 0.
func U256LELt(a, b *U256LE) (r U256LE) {
	u256leLt(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LEGt returns 1 if a > b, else 0.
func U256LEGt(a, b *U256LE) (r U256LE) {
	u256leGt(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LESLt returns 1 if a < b as two's-complement integers, else 0.
func U256LESLt(a, b *U256LE) (r U256LE) {
	u256leSLt(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LESGt returns 1 if a > b as two's-complement integers, else 0.
func U256LESGt(a, b *U256LE) (r U256LE) {
	u256leSGt(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LEEq returns 1 if a == b, else 0.
func U256LEEq(a, b *U256LE) (r U256LE) {
	u256leEq(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LEIsZero returns 1 if a is 0, else 0.
func U256LEIsZero(a *U256LE) (r U256LE) {
	u256leIsZero(unsafe.Pointer(a), unsafe.Pointer(&r))
	return
}

// U256LEAnd returns a & b.
func U256LEAnd(a, b *U256LE) (r U256LE) {
	u256leAnd(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LEOr returns a | b.
func U256LEOr(a, b *U256LE) (r U256LE) {
	u256leOr(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LEXor returns a ^ b.
func U256LEXor(a, b *U256LE) (r U256LE) {
	u256leXor(unsafe.Pointer(a), unsafe.Pointer(b), unsafe.Pointer(&r))
	return
}

// U256LENot returns ^a.
func U256LENot(a *U256LE) (r U256LE) {
	u256leNot(unsafe.Pointer(a), unsafe.Pointer(&r))
	return
}

// U256LEByte returns byte i of a, counting from the most significant (EVM BYTE), or 0 when i >= 32.
func U256LEByte(i, a *U256LE) (r U256LE) {
	u256leByte(unsafe.Pointer(i), unsafe.Pointer(a), unsafe.Pointer(&r))
	return
}

// U256LEShl returns value << shift, or 0 when shift >= 256.
func U256LEShl(shift, value *U256LE) (r U256LE) {
	u256leShl(unsafe.Pointer(shift), unsafe.Pointer(value), unsafe.Pointer(&r))
	return
}

// U256LEShr returns value >> shift (logical), or 0 when shift >= 256.
func U256LEShr(shift, value *U256LE) (r U256LE) {
	u256leShr(unsafe.Pointer(shift), unsafe.Pointer(value), unsafe.Pointer(&r))
	return
}

// U256LESar returns value >> shift (arithmetic).
func U256LESar(shift, value *U256LE) (r U256LE) {
	u256leSar(unsafe.Pointer(shift), unsafe.Pointer(value), unsafe.Pointer(&r))
	return
}

// U256LESignExtend returns value sign-extended from byte b, counting from the least significant (EVM SIGNEXTEND); value itself when b >= 31.
func U256LESignExtend(b, value *U256LE) (r U256LE) {
	u256leSignExtend(unsafe.Pointer(b), unsafe.Pointer(value), unsafe.Pointer(&r))
	return
}
