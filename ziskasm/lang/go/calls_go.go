//go:build !tinygo && wasip1

package zkvm

// The functions of the C headers for standard Go (GOOS=wasip1 GOARCH=wasm), which
// has no cgo: each is a //go:wasmimport of the env.* import TinyGo's cgo path
// produces, with the header's signature under the wasm32 C ABI (pointers and
// size_t are i32, uint64_t is i64). wasmimport takes no uint8 parameter, so those
// cross as uint32; a status the cgo wrapper discards is discarded here too.

import "unsafe"

//go:wasmimport env zkvm_keccak256
//go:noescape
func zkvm_keccak256(data unsafe.Pointer, n uintptr, out unsafe.Pointer) int32

func keccak256(data unsafe.Pointer, n uintptr, out unsafe.Pointer) int32 {
	return zkvm_keccak256(data, n, out)
}

//go:wasmimport env zkvm_keccak_f1600
//go:noescape
func zkvm_keccak_f1600(state unsafe.Pointer) int32

func keccakF1600(state unsafe.Pointer) int32 { return zkvm_keccak_f1600(state) }

//go:wasmimport env zkvm_sha256
//go:noescape
func zkvm_sha256(data unsafe.Pointer, n uintptr, out unsafe.Pointer) int32

func sha256(data unsafe.Pointer, n uintptr, out unsafe.Pointer) int32 {
	return zkvm_sha256(data, n, out)
}

//go:wasmimport env zkvm_ripemd160
//go:noescape
func zkvm_ripemd160(data unsafe.Pointer, n uintptr, out unsafe.Pointer) int32

func ripemd160(data unsafe.Pointer, n uintptr, out unsafe.Pointer) int32 {
	return zkvm_ripemd160(data, n, out)
}

//go:wasmimport env zkvm_secp256k1_verify
//go:noescape
func zkvm_secp256k1_verify(msg, sig, pubkey, ok unsafe.Pointer) int32

func secp256k1Verify(msg, sig, pubkey, ok unsafe.Pointer) int32 {
	return zkvm_secp256k1_verify(msg, sig, pubkey, ok)
}

//go:wasmimport env zkvm_secp256k1_ecrecover
//go:noescape
func zkvm_secp256k1_ecrecover(msg, sig unsafe.Pointer, recid uint32, pubkey unsafe.Pointer) int32

func secp256k1Ecrecover(msg, sig unsafe.Pointer, recid uint8, pubkey unsafe.Pointer) int32 {
	return zkvm_secp256k1_ecrecover(msg, sig, uint32(recid), pubkey)
}

//go:wasmimport env zkvm_secp256r1_verify
//go:noescape
func zkvm_secp256r1_verify(msg, sig, pubkey, ok unsafe.Pointer) int32

func secp256r1Verify(msg, sig, pubkey, ok unsafe.Pointer) int32 {
	return zkvm_secp256r1_verify(msg, sig, pubkey, ok)
}

//go:wasmimport env zkvm_modexp
//go:noescape
func zkvm_modexp(base unsafe.Pointer, baseLen uintptr, exp unsafe.Pointer, expLen uintptr, modulus unsafe.Pointer, modLen uintptr, out unsafe.Pointer) int32

func modexp(base unsafe.Pointer, baseLen uintptr, exp unsafe.Pointer, expLen uintptr, modulus unsafe.Pointer, modLen uintptr, out unsafe.Pointer) int32 {
	return zkvm_modexp(base, baseLen, exp, expLen, modulus, modLen, out)
}

//go:wasmimport env zkvm_bn254_g1_add
//go:noescape
func zkvm_bn254_g1_add(p1, p2, out unsafe.Pointer) int32

func bn254G1Add(p1, p2, out unsafe.Pointer) int32 { return zkvm_bn254_g1_add(p1, p2, out) }

//go:wasmimport env zkvm_bn254_g1_mul
//go:noescape
func zkvm_bn254_g1_mul(point, scalar, out unsafe.Pointer) int32

func bn254G1Mul(point, scalar, out unsafe.Pointer) int32 {
	return zkvm_bn254_g1_mul(point, scalar, out)
}

//go:wasmimport env zkvm_bn254_pairing
//go:noescape
func zkvm_bn254_pairing(pairs unsafe.Pointer, n uintptr, ok unsafe.Pointer) int32

func bn254Pairing(pairs unsafe.Pointer, n uintptr, ok unsafe.Pointer) int32 {
	return zkvm_bn254_pairing(pairs, n, ok)
}

//go:wasmimport env zkvm_blake2f
//go:noescape
func zkvm_blake2f(rounds uint32, h, m, t unsafe.Pointer, f uint32) int32

func blake2f(rounds uint32, h, m, t unsafe.Pointer, f uint8) int32 {
	return zkvm_blake2f(rounds, h, m, t, uint32(f))
}

//go:wasmimport env zkvm_kzg_point_eval
//go:noescape
func zkvm_kzg_point_eval(commitment, z, y, proof, ok unsafe.Pointer) int32

func kzgPointEval(commitment, z, y, proof, ok unsafe.Pointer) int32 {
	return zkvm_kzg_point_eval(commitment, z, y, proof, ok)
}

//go:wasmimport env zkvm_bls12_g1_add
//go:noescape
func zkvm_bls12_g1_add(p1, p2, out unsafe.Pointer) int32

func bls12G1Add(p1, p2, out unsafe.Pointer) int32 { return zkvm_bls12_g1_add(p1, p2, out) }

//go:wasmimport env zkvm_bls12_g1_msm
//go:noescape
func zkvm_bls12_g1_msm(pairs unsafe.Pointer, n uintptr, out unsafe.Pointer) int32

func bls12G1Msm(pairs unsafe.Pointer, n uintptr, out unsafe.Pointer) int32 {
	return zkvm_bls12_g1_msm(pairs, n, out)
}

//go:wasmimport env zkvm_bls12_g2_add
//go:noescape
func zkvm_bls12_g2_add(p1, p2, out unsafe.Pointer) int32

func bls12G2Add(p1, p2, out unsafe.Pointer) int32 { return zkvm_bls12_g2_add(p1, p2, out) }

//go:wasmimport env zkvm_bls12_g2_msm
//go:noescape
func zkvm_bls12_g2_msm(pairs unsafe.Pointer, n uintptr, out unsafe.Pointer) int32

func bls12G2Msm(pairs unsafe.Pointer, n uintptr, out unsafe.Pointer) int32 {
	return zkvm_bls12_g2_msm(pairs, n, out)
}

//go:wasmimport env zkvm_bls12_pairing
//go:noescape
func zkvm_bls12_pairing(pairs unsafe.Pointer, n uintptr, ok unsafe.Pointer) int32

func bls12Pairing(pairs unsafe.Pointer, n uintptr, ok unsafe.Pointer) int32 {
	return zkvm_bls12_pairing(pairs, n, ok)
}

//go:wasmimport env zkvm_bls12_map_fp_to_g1
//go:noescape
func zkvm_bls12_map_fp_to_g1(element, out unsafe.Pointer) int32

func bls12MapFpToG1(element, out unsafe.Pointer) int32 { return zkvm_bls12_map_fp_to_g1(element, out) }

//go:wasmimport env zkvm_bls12_map_fp2_to_g2
//go:noescape
func zkvm_bls12_map_fp2_to_g2(element, out unsafe.Pointer) int32

func bls12MapFp2ToG2(element, out unsafe.Pointer) int32 {
	return zkvm_bls12_map_fp2_to_g2(element, out)
}

//go:wasmimport env zkvm_u256_div
//go:noescape
func zkvm_u256_div(a, b, c unsafe.Pointer) int32

func u256Div(a, b, c unsafe.Pointer) { zkvm_u256_div(a, b, c) }

//go:wasmimport env zkvm_u256_mod
//go:noescape
func zkvm_u256_mod(a, b, c unsafe.Pointer) int32

func u256Mod(a, b, c unsafe.Pointer) { zkvm_u256_mod(a, b, c) }

//go:wasmimport env zkvm_u256_divmod
//go:noescape
func zkvm_u256_divmod(a, b, c, d unsafe.Pointer) int32

func u256DivMod(a, b, c, d unsafe.Pointer) { zkvm_u256_divmod(a, b, c, d) }

//go:wasmimport env zkvm_u256_sdiv
//go:noescape
func zkvm_u256_sdiv(a, b, c unsafe.Pointer) int32

func u256SDiv(a, b, c unsafe.Pointer) { zkvm_u256_sdiv(a, b, c) }

//go:wasmimport env zkvm_u256_smod
//go:noescape
func zkvm_u256_smod(a, b, c unsafe.Pointer) int32

func u256SMod(a, b, c unsafe.Pointer) { zkvm_u256_smod(a, b, c) }

//go:wasmimport env zkvm_u256_sdivmod
//go:noescape
func zkvm_u256_sdivmod(a, b, c, d unsafe.Pointer) int32

func u256SDivMod(a, b, c, d unsafe.Pointer) { zkvm_u256_sdivmod(a, b, c, d) }

//go:wasmimport env zkvm_u256_add
//go:noescape
func zkvm_u256_add(a, b, c unsafe.Pointer) int32

func u256Add(a, b, c unsafe.Pointer) { zkvm_u256_add(a, b, c) }

//go:wasmimport env zkvm_u256_sub
//go:noescape
func zkvm_u256_sub(a, b, c unsafe.Pointer) int32

func u256Sub(a, b, c unsafe.Pointer) { zkvm_u256_sub(a, b, c) }

//go:wasmimport env zkvm_u256_mul
//go:noescape
func zkvm_u256_mul(a, b, c unsafe.Pointer) int32

func u256Mul(a, b, c unsafe.Pointer) { zkvm_u256_mul(a, b, c) }

//go:wasmimport env zkvm_u256_addmod
//go:noescape
func zkvm_u256_addmod(a, b, c, d unsafe.Pointer) int32

func u256AddMod(a, b, c, d unsafe.Pointer) { zkvm_u256_addmod(a, b, c, d) }

//go:wasmimport env zkvm_u256_mulmod
//go:noescape
func zkvm_u256_mulmod(a, b, c, d unsafe.Pointer) int32

func u256MulMod(a, b, c, d unsafe.Pointer) { zkvm_u256_mulmod(a, b, c, d) }

//go:wasmimport env zkvm_u256_exp
//go:noescape
func zkvm_u256_exp(a, b, c unsafe.Pointer) int32

func u256Exp(a, b, c unsafe.Pointer) { zkvm_u256_exp(a, b, c) }

//go:wasmimport env zkvm_u256_lt
//go:noescape
func zkvm_u256_lt(a, b, c unsafe.Pointer) int32

func u256Lt(a, b, c unsafe.Pointer) { zkvm_u256_lt(a, b, c) }

//go:wasmimport env zkvm_u256_gt
//go:noescape
func zkvm_u256_gt(a, b, c unsafe.Pointer) int32

func u256Gt(a, b, c unsafe.Pointer) { zkvm_u256_gt(a, b, c) }

//go:wasmimport env zkvm_u256_slt
//go:noescape
func zkvm_u256_slt(a, b, c unsafe.Pointer) int32

func u256SLt(a, b, c unsafe.Pointer) { zkvm_u256_slt(a, b, c) }

//go:wasmimport env zkvm_u256_sgt
//go:noescape
func zkvm_u256_sgt(a, b, c unsafe.Pointer) int32

func u256SGt(a, b, c unsafe.Pointer) { zkvm_u256_sgt(a, b, c) }

//go:wasmimport env zkvm_u256_eq
//go:noescape
func zkvm_u256_eq(a, b, c unsafe.Pointer) int32

func u256Eq(a, b, c unsafe.Pointer) { zkvm_u256_eq(a, b, c) }

//go:wasmimport env zkvm_u256_iszero
//go:noescape
func zkvm_u256_iszero(a, b unsafe.Pointer) int32

func u256IsZero(a, b unsafe.Pointer) { zkvm_u256_iszero(a, b) }

//go:wasmimport env zkvm_u256_and
//go:noescape
func zkvm_u256_and(a, b, c unsafe.Pointer) int32

func u256And(a, b, c unsafe.Pointer) { zkvm_u256_and(a, b, c) }

//go:wasmimport env zkvm_u256_or
//go:noescape
func zkvm_u256_or(a, b, c unsafe.Pointer) int32

func u256Or(a, b, c unsafe.Pointer) { zkvm_u256_or(a, b, c) }

//go:wasmimport env zkvm_u256_xor
//go:noescape
func zkvm_u256_xor(a, b, c unsafe.Pointer) int32

func u256Xor(a, b, c unsafe.Pointer) { zkvm_u256_xor(a, b, c) }

//go:wasmimport env zkvm_u256_not
//go:noescape
func zkvm_u256_not(a, b unsafe.Pointer) int32

func u256Not(a, b unsafe.Pointer) { zkvm_u256_not(a, b) }

//go:wasmimport env zkvm_u256_byte
//go:noescape
func zkvm_u256_byte(a, b, c unsafe.Pointer) int32

func u256Byte(a, b, c unsafe.Pointer) { zkvm_u256_byte(a, b, c) }

//go:wasmimport env zkvm_u256_shl
//go:noescape
func zkvm_u256_shl(a, b, c unsafe.Pointer) int32

func u256Shl(a, b, c unsafe.Pointer) { zkvm_u256_shl(a, b, c) }

//go:wasmimport env zkvm_u256_shr
//go:noescape
func zkvm_u256_shr(a, b, c unsafe.Pointer) int32

func u256Shr(a, b, c unsafe.Pointer) { zkvm_u256_shr(a, b, c) }

//go:wasmimport env zkvm_u256_sar
//go:noescape
func zkvm_u256_sar(a, b, c unsafe.Pointer) int32

func u256Sar(a, b, c unsafe.Pointer) { zkvm_u256_sar(a, b, c) }

//go:wasmimport env zkvm_u256_signextend
//go:noescape
func zkvm_u256_signextend(a, b, c unsafe.Pointer) int32

func u256SignExtend(a, b, c unsafe.Pointer) { zkvm_u256_signextend(a, b, c) }

//go:wasmimport env zkvm_u256_le_div
//go:noescape
func zkvm_u256_le_div(a, b, c unsafe.Pointer) int32

func u256leDiv(a, b, c unsafe.Pointer) { zkvm_u256_le_div(a, b, c) }

//go:wasmimport env zkvm_u256_le_mod
//go:noescape
func zkvm_u256_le_mod(a, b, c unsafe.Pointer) int32

func u256leMod(a, b, c unsafe.Pointer) { zkvm_u256_le_mod(a, b, c) }

//go:wasmimport env zkvm_u256_le_divmod
//go:noescape
func zkvm_u256_le_divmod(a, b, c, d unsafe.Pointer) int32

func u256leDivMod(a, b, c, d unsafe.Pointer) { zkvm_u256_le_divmod(a, b, c, d) }

//go:wasmimport env zkvm_u256_le_sdiv
//go:noescape
func zkvm_u256_le_sdiv(a, b, c unsafe.Pointer) int32

func u256leSDiv(a, b, c unsafe.Pointer) { zkvm_u256_le_sdiv(a, b, c) }

//go:wasmimport env zkvm_u256_le_smod
//go:noescape
func zkvm_u256_le_smod(a, b, c unsafe.Pointer) int32

func u256leSMod(a, b, c unsafe.Pointer) { zkvm_u256_le_smod(a, b, c) }

//go:wasmimport env zkvm_u256_le_sdivmod
//go:noescape
func zkvm_u256_le_sdivmod(a, b, c, d unsafe.Pointer) int32

func u256leSDivMod(a, b, c, d unsafe.Pointer) { zkvm_u256_le_sdivmod(a, b, c, d) }

//go:wasmimport env zkvm_u256_le_add
//go:noescape
func zkvm_u256_le_add(a, b, c unsafe.Pointer) int32

func u256leAdd(a, b, c unsafe.Pointer) { zkvm_u256_le_add(a, b, c) }

//go:wasmimport env zkvm_u256_le_sub
//go:noescape
func zkvm_u256_le_sub(a, b, c unsafe.Pointer) int32

func u256leSub(a, b, c unsafe.Pointer) { zkvm_u256_le_sub(a, b, c) }

//go:wasmimport env zkvm_u256_le_mul
//go:noescape
func zkvm_u256_le_mul(a, b, c unsafe.Pointer) int32

func u256leMul(a, b, c unsafe.Pointer) { zkvm_u256_le_mul(a, b, c) }

//go:wasmimport env zkvm_u256_le_addmod
//go:noescape
func zkvm_u256_le_addmod(a, b, c, d unsafe.Pointer) int32

func u256leAddMod(a, b, c, d unsafe.Pointer) { zkvm_u256_le_addmod(a, b, c, d) }

//go:wasmimport env zkvm_u256_le_mulmod
//go:noescape
func zkvm_u256_le_mulmod(a, b, c, d unsafe.Pointer) int32

func u256leMulMod(a, b, c, d unsafe.Pointer) { zkvm_u256_le_mulmod(a, b, c, d) }

//go:wasmimport env zkvm_u256_le_exp
//go:noescape
func zkvm_u256_le_exp(a, b, c unsafe.Pointer) int32

func u256leExp(a, b, c unsafe.Pointer) { zkvm_u256_le_exp(a, b, c) }

//go:wasmimport env zkvm_u256_le_lt
//go:noescape
func zkvm_u256_le_lt(a, b, c unsafe.Pointer) int32

func u256leLt(a, b, c unsafe.Pointer) { zkvm_u256_le_lt(a, b, c) }

//go:wasmimport env zkvm_u256_le_gt
//go:noescape
func zkvm_u256_le_gt(a, b, c unsafe.Pointer) int32

func u256leGt(a, b, c unsafe.Pointer) { zkvm_u256_le_gt(a, b, c) }

//go:wasmimport env zkvm_u256_le_slt
//go:noescape
func zkvm_u256_le_slt(a, b, c unsafe.Pointer) int32

func u256leSLt(a, b, c unsafe.Pointer) { zkvm_u256_le_slt(a, b, c) }

//go:wasmimport env zkvm_u256_le_sgt
//go:noescape
func zkvm_u256_le_sgt(a, b, c unsafe.Pointer) int32

func u256leSGt(a, b, c unsafe.Pointer) { zkvm_u256_le_sgt(a, b, c) }

//go:wasmimport env zkvm_u256_le_eq
//go:noescape
func zkvm_u256_le_eq(a, b, c unsafe.Pointer) int32

func u256leEq(a, b, c unsafe.Pointer) { zkvm_u256_le_eq(a, b, c) }

//go:wasmimport env zkvm_u256_le_iszero
//go:noescape
func zkvm_u256_le_iszero(a, b unsafe.Pointer) int32

func u256leIsZero(a, b unsafe.Pointer) { zkvm_u256_le_iszero(a, b) }

//go:wasmimport env zkvm_u256_le_and
//go:noescape
func zkvm_u256_le_and(a, b, c unsafe.Pointer) int32

func u256leAnd(a, b, c unsafe.Pointer) { zkvm_u256_le_and(a, b, c) }

//go:wasmimport env zkvm_u256_le_or
//go:noescape
func zkvm_u256_le_or(a, b, c unsafe.Pointer) int32

func u256leOr(a, b, c unsafe.Pointer) { zkvm_u256_le_or(a, b, c) }

//go:wasmimport env zkvm_u256_le_xor
//go:noescape
func zkvm_u256_le_xor(a, b, c unsafe.Pointer) int32

func u256leXor(a, b, c unsafe.Pointer) { zkvm_u256_le_xor(a, b, c) }

//go:wasmimport env zkvm_u256_le_not
//go:noescape
func zkvm_u256_le_not(a, b unsafe.Pointer) int32

func u256leNot(a, b unsafe.Pointer) { zkvm_u256_le_not(a, b) }

//go:wasmimport env zkvm_u256_le_byte
//go:noescape
func zkvm_u256_le_byte(a, b, c unsafe.Pointer) int32

func u256leByte(a, b, c unsafe.Pointer) { zkvm_u256_le_byte(a, b, c) }

//go:wasmimport env zkvm_u256_le_shl
//go:noescape
func zkvm_u256_le_shl(a, b, c unsafe.Pointer) int32

func u256leShl(a, b, c unsafe.Pointer) { zkvm_u256_le_shl(a, b, c) }

//go:wasmimport env zkvm_u256_le_shr
//go:noescape
func zkvm_u256_le_shr(a, b, c unsafe.Pointer) int32

func u256leShr(a, b, c unsafe.Pointer) { zkvm_u256_le_shr(a, b, c) }

//go:wasmimport env zkvm_u256_le_sar
//go:noescape
func zkvm_u256_le_sar(a, b, c unsafe.Pointer) int32

func u256leSar(a, b, c unsafe.Pointer) { zkvm_u256_le_sar(a, b, c) }

//go:wasmimport env zkvm_u256_le_signextend
//go:noescape
func zkvm_u256_le_signextend(a, b, c unsafe.Pointer) int32

func u256leSignExtend(a, b, c unsafe.Pointer) { zkvm_u256_le_signextend(a, b, c) }

//go:wasmimport env zkvm_random_u64
func zkvm_random_u64() uint64

func randomU64() uint64 { return zkvm_random_u64() }

//go:wasmimport env write_log
//go:noescape
func write_log(text unsafe.Pointer, n uintptr)

func writeLog(text unsafe.Pointer, n uintptr) { write_log(text, n) }

//go:wasmimport env zkvm_zisklib_add
func zkvm_zisklib_add(a, b uint64) uint64

func zisklibAdd(a, b uint64) uint64 { return zkvm_zisklib_add(a, b) }

//go:wasmimport env zkvm_zisklib_inv256
//go:noescape
func zkvm_zisklib_inv256(a, r unsafe.Pointer) uint64

func zisklibInv256(a, r unsafe.Pointer) uint64 { return zkvm_zisklib_inv256(a, r) }

//go:wasmimport env zkvm_zisklib_overflowing_add256
//go:noescape
func zkvm_zisklib_overflowing_add256(a, b, r unsafe.Pointer) uint64

func zisklibOverflowingAdd256(a, b, r unsafe.Pointer) uint64 {
	return zkvm_zisklib_overflowing_add256(a, b, r)
}

//go:wasmimport env zkvm_zisklib_overflowing_sub256
//go:noescape
func zkvm_zisklib_overflowing_sub256(a, b, r unsafe.Pointer) uint64

func zisklibOverflowingSub256(a, b, r unsafe.Pointer) uint64 {
	return zkvm_zisklib_overflowing_sub256(a, b, r)
}

//go:wasmimport env zkvm_zisklib_overflowing_mul256
//go:noescape
func zkvm_zisklib_overflowing_mul256(a, b, r unsafe.Pointer) uint64

func zisklibOverflowingMul256(a, b, r unsafe.Pointer) uint64 {
	return zkvm_zisklib_overflowing_mul256(a, b, r)
}

//go:wasmimport env zkvm_zisklib_div_rem256
//go:noescape
func zkvm_zisklib_div_rem256(a, b, q, r unsafe.Pointer)

func zisklibDivRem256(a, b, q, r unsafe.Pointer) { zkvm_zisklib_div_rem256(a, b, q, r) }

//go:wasmimport env zkvm_zisklib_reduce_mod256
//go:noescape
func zkvm_zisklib_reduce_mod256(a, m, r unsafe.Pointer)

func zisklibReduceMod256(a, m, r unsafe.Pointer) { zkvm_zisklib_reduce_mod256(a, m, r) }

//go:wasmimport env zkvm_zisklib_add_mod256
//go:noescape
func zkvm_zisklib_add_mod256(a, b, m, r unsafe.Pointer)

func zisklibAddMod256(a, b, m, r unsafe.Pointer) { zkvm_zisklib_add_mod256(a, b, m, r) }

//go:wasmimport env zkvm_zisklib_mul_mod256
//go:noescape
func zkvm_zisklib_mul_mod256(a, b, m, r unsafe.Pointer)

func zisklibMulMod256(a, b, m, r unsafe.Pointer) { zkvm_zisklib_mul_mod256(a, b, m, r) }

//go:wasmimport env zkvm_zisklib_inv_mod256
//go:noescape
func zkvm_zisklib_inv_mod256(a, m, r unsafe.Pointer) uint64

func zisklibInvMod256(a, m, r unsafe.Pointer) uint64 { return zkvm_zisklib_inv_mod256(a, m, r) }

//go:wasmimport env zkvm_zisklib_pow_mod256
//go:noescape
func zkvm_zisklib_pow_mod256(base, exp, m, r unsafe.Pointer)

func zisklibPowMod256(base, exp, m, r unsafe.Pointer) { zkvm_zisklib_pow_mod256(base, exp, m, r) }

//go:wasmimport env zkvm_zisklib_overflowing_pow256
//go:noescape
func zkvm_zisklib_overflowing_pow256(base, exp, r unsafe.Pointer) uint64

func zisklibOverflowingPow256(base, exp, r unsafe.Pointer) uint64 {
	return zkvm_zisklib_overflowing_pow256(base, exp, r)
}

//go:wasmimport env zkvm_zisklib_ecdsa_verify_secp256k1
//go:noescape
func zkvm_zisklib_ecdsa_verify_secp256k1(pk, z, r, s unsafe.Pointer) uint64

func zisklibEcdsaVerifySecp256k1(pk, z, r, s unsafe.Pointer) uint64 {
	return zkvm_zisklib_ecdsa_verify_secp256k1(pk, z, r, s)
}

//go:wasmimport env zkvm_zisklib_ecdsa_recover_secp256k1
//go:noescape
func zkvm_zisklib_ecdsa_recover_secp256k1(r, s, z unsafe.Pointer, recid uint64, pk unsafe.Pointer) uint64

func zisklibEcdsaRecoverSecp256k1(r, s, z unsafe.Pointer, recid uint64, pk unsafe.Pointer) uint64 {
	return zkvm_zisklib_ecdsa_recover_secp256k1(r, s, z, recid, pk)
}

//go:wasmimport env zkvm_zisklib_schnorr_verify_secp256k1
//go:noescape
func zkvm_zisklib_schnorr_verify_secp256k1(pkX, r, s, msg unsafe.Pointer, n uint64) uint64

func zisklibSchnorrVerifySecp256k1(pkX, r, s, msg unsafe.Pointer, n uint64) uint64 {
	return zkvm_zisklib_schnorr_verify_secp256k1(pkX, r, s, msg, n)
}

//go:wasmimport env zkvm_zisklib_ecdsa_verify_secp256r1
//go:noescape
func zkvm_zisklib_ecdsa_verify_secp256r1(pk, z, r, s unsafe.Pointer) uint64

func zisklibEcdsaVerifySecp256r1(pk, z, r, s unsafe.Pointer) uint64 {
	return zkvm_zisklib_ecdsa_verify_secp256r1(pk, z, r, s)
}

//go:wasmimport env zkvm_zisklib_pairing_check_bn254
//go:noescape
func zkvm_zisklib_pairing_check_bn254(g1, g2 unsafe.Pointer, n uint64) uint64

func zisklibPairingCheckBn254(g1, g2 unsafe.Pointer, n uint64) uint64 {
	return zkvm_zisklib_pairing_check_bn254(g1, g2, n)
}

//go:wasmimport env zkvm_zisklib_pairing_check_bls12_381
//go:noescape
func zkvm_zisklib_pairing_check_bls12_381(g1, g2 unsafe.Pointer, n uint64) uint64

func zisklibPairingCheckBls12381(g1, g2 unsafe.Pointer, n uint64) uint64 {
	return zkvm_zisklib_pairing_check_bls12_381(g1, g2, n)
}

//go:wasmimport env zkvm_zisklib_map_to_curve_g1_bls12_381
//go:noescape
func zkvm_zisklib_map_to_curve_g1_bls12_381(u, r unsafe.Pointer) uint64

func zisklibMapToCurveG1Bls12381(u, r unsafe.Pointer) uint64 {
	return zkvm_zisklib_map_to_curve_g1_bls12_381(u, r)
}

//go:wasmimport env zkvm_zisklib_map_to_curve_g2_bls12_381
//go:noescape
func zkvm_zisklib_map_to_curve_g2_bls12_381(u, r unsafe.Pointer) uint64

func zisklibMapToCurveG2Bls12381(u, r unsafe.Pointer) uint64 {
	return zkvm_zisklib_map_to_curve_g2_bls12_381(u, r)
}

//go:wasmimport env zkvm_zisklib_hash_to_curve_g2_bls12_381
//go:noescape
func zkvm_zisklib_hash_to_curve_g2_bls12_381(msg unsafe.Pointer, msgLen uint64, dst unsafe.Pointer, dstLen uint64, r unsafe.Pointer)

func zisklibHashToCurveG2Bls12381(msg unsafe.Pointer, msgLen uint64, dst unsafe.Pointer, dstLen uint64, r unsafe.Pointer) {
	zkvm_zisklib_hash_to_curve_g2_bls12_381(msg, msgLen, dst, dstLen, r)
}

//go:wasmimport env zkvm_zisklib_bls_verify_bls12_381
//go:noescape
func zkvm_zisklib_bls_verify_bls12_381(pk, msg unsafe.Pointer, n uint64, sig unsafe.Pointer) uint64

func zisklibBlsVerifyBls12381(pk, msg unsafe.Pointer, n uint64, sig unsafe.Pointer) uint64 {
	return zkvm_zisklib_bls_verify_bls12_381(pk, msg, n, sig)
}

//go:wasmimport env zkvm_zisklib_verify_kzg_proof_bls12_381
//go:noescape
func zkvm_zisklib_verify_kzg_proof_bls12_381(z, y, commitment, proof unsafe.Pointer) uint64

func zisklibVerifyKzgProofBls12381(z, y, commitment, proof unsafe.Pointer) uint64 {
	return zkvm_zisklib_verify_kzg_proof_bls12_381(z, y, commitment, proof)
}

//go:wasmimport env zkvm_zisklib_modexp_u64_c
//go:noescape
func zkvm_zisklib_modexp_u64_c(base unsafe.Pointer, baseLen uintptr, exp unsafe.Pointer, expLen uintptr, modulus unsafe.Pointer, modLen uintptr, r unsafe.Pointer) uintptr

func zisklibModexpU64(base unsafe.Pointer, baseLen uintptr, exp unsafe.Pointer, expLen uintptr, modulus unsafe.Pointer, modLen uintptr, r unsafe.Pointer) uintptr {
	return zkvm_zisklib_modexp_u64_c(base, baseLen, exp, expLen, modulus, modLen, r)
}

//go:wasmimport env zkvm_fcall
//go:noescape
func zkvm_fcall(id uint32, params unsafe.Pointer, n uintptr) int32

func fcall(id uint32, params unsafe.Pointer, n uintptr) int32 { return zkvm_fcall(id, params, n) }

//go:wasmimport env zkvm_fcall_get
func zkvm_fcall_get() uint64

func fcallGet() uint64 { return zkvm_fcall_get() }

//go:wasmimport env zkvm_fcall_read
//go:noescape
func zkvm_fcall_read(results unsafe.Pointer, n uintptr)

func fcallRead(results unsafe.Pointer, n uintptr) { zkvm_fcall_read(results, n) }
