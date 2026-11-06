//go:build tinygo && wasip1

package zkvm

// The functions of the C headers, called through cgo. TinyGo compiles them as
// undefined C functions, which the wasm linker turns into env.* imports for
// wasm2rom to implement.

// #cgo CFLAGS: -I../c/include
// #include "zkvm_accelerators.h"
// #include "zkvm_u256.h"
// #include "zkvm_u256_le.h"
// #include "zkvm_random.h"
// #include "zkvm_io.h"
// #include "zkvm_zisklib.h"
// #include "zkvm_fcall.h"
import "C"

import "unsafe"

func keccak256(data unsafe.Pointer, n uintptr, out unsafe.Pointer) int32 {
	return int32(C.zkvm_keccak256((*C.uint8_t)(data), C.size_t(n), (*C.zkvm_keccak256_hash)(out)))
}

func keccakF1600(state unsafe.Pointer) int32 {
	return int32(C.zkvm_keccak_f1600((*C.uint64_t)(state)))
}

func sha256(data unsafe.Pointer, n uintptr, out unsafe.Pointer) int32 {
	return int32(C.zkvm_sha256((*C.uint8_t)(data), C.size_t(n), (*C.zkvm_sha256_hash)(out)))
}

func ripemd160(data unsafe.Pointer, n uintptr, out unsafe.Pointer) int32 {
	return int32(C.zkvm_ripemd160((*C.uint8_t)(data), C.size_t(n), (*C.zkvm_ripemd160_hash)(out)))
}

func secp256k1Verify(msg, sig, pubkey, ok unsafe.Pointer) int32 {
	return int32(C.zkvm_secp256k1_verify((*C.zkvm_secp256k1_hash)(msg),
		(*C.zkvm_secp256k1_signature)(sig), (*C.zkvm_secp256k1_pubkey)(pubkey), (*C._Bool)(ok)))
}

func secp256k1Ecrecover(msg, sig unsafe.Pointer, recid uint8, pubkey unsafe.Pointer) int32 {
	return int32(C.zkvm_secp256k1_ecrecover((*C.zkvm_secp256k1_hash)(msg),
		(*C.zkvm_secp256k1_signature)(sig), C.uint8_t(recid), (*C.zkvm_secp256k1_pubkey)(pubkey)))
}

func secp256r1Verify(msg, sig, pubkey, ok unsafe.Pointer) int32 {
	return int32(C.zkvm_secp256r1_verify((*C.zkvm_secp256r1_hash)(msg),
		(*C.zkvm_secp256r1_signature)(sig), (*C.zkvm_secp256r1_pubkey)(pubkey), (*C._Bool)(ok)))
}

func modexp(base unsafe.Pointer, baseLen uintptr, exp unsafe.Pointer, expLen uintptr,
	modulus unsafe.Pointer, modLen uintptr, out unsafe.Pointer) int32 {
	return int32(C.zkvm_modexp((*C.uint8_t)(base), C.size_t(baseLen), (*C.uint8_t)(exp),
		C.size_t(expLen), (*C.uint8_t)(modulus), C.size_t(modLen), (*C.uint8_t)(out)))
}

func bn254G1Add(p1, p2, out unsafe.Pointer) int32 {
	return int32(C.zkvm_bn254_g1_add((*C.zkvm_bn254_g1_point)(p1), (*C.zkvm_bn254_g1_point)(p2),
		(*C.zkvm_bn254_g1_point)(out)))
}

func bn254G1Mul(point, scalar, out unsafe.Pointer) int32 {
	return int32(C.zkvm_bn254_g1_mul((*C.zkvm_bn254_g1_point)(point),
		(*C.zkvm_bn254_scalar)(scalar), (*C.zkvm_bn254_g1_point)(out)))
}

func bn254Pairing(pairs unsafe.Pointer, n uintptr, ok unsafe.Pointer) int32 {
	return int32(C.zkvm_bn254_pairing((*C.zkvm_bn254_pairing_pair)(pairs), C.size_t(n),
		(*C._Bool)(ok)))
}

func blake2f(rounds uint32, h, m, t unsafe.Pointer, f uint8) int32 {
	return int32(C.zkvm_blake2f(C.uint32_t(rounds), (*C.zkvm_blake2f_state)(h),
		(*C.zkvm_blake2f_message)(m), (*C.zkvm_blake2f_offset)(t), C.uint8_t(f)))
}

func kzgPointEval(commitment, z, y, proof, ok unsafe.Pointer) int32 {
	return int32(C.zkvm_kzg_point_eval((*C.zkvm_kzg_commitment)(commitment),
		(*C.zkvm_kzg_field_element)(z), (*C.zkvm_kzg_field_element)(y),
		(*C.zkvm_kzg_proof)(proof), (*C._Bool)(ok)))
}

func bls12G1Add(p1, p2, out unsafe.Pointer) int32 {
	return int32(C.zkvm_bls12_g1_add((*C.zkvm_bls12_381_g1_point)(p1),
		(*C.zkvm_bls12_381_g1_point)(p2), (*C.zkvm_bls12_381_g1_point)(out)))
}

func bls12G1Msm(pairs unsafe.Pointer, n uintptr, out unsafe.Pointer) int32 {
	return int32(C.zkvm_bls12_g1_msm((*C.zkvm_bls12_381_g1_msm_pair)(pairs), C.size_t(n),
		(*C.zkvm_bls12_381_g1_point)(out)))
}

func bls12G2Add(p1, p2, out unsafe.Pointer) int32 {
	return int32(C.zkvm_bls12_g2_add((*C.zkvm_bls12_381_g2_point)(p1),
		(*C.zkvm_bls12_381_g2_point)(p2), (*C.zkvm_bls12_381_g2_point)(out)))
}

func bls12G2Msm(pairs unsafe.Pointer, n uintptr, out unsafe.Pointer) int32 {
	return int32(C.zkvm_bls12_g2_msm((*C.zkvm_bls12_381_g2_msm_pair)(pairs), C.size_t(n),
		(*C.zkvm_bls12_381_g2_point)(out)))
}

func bls12Pairing(pairs unsafe.Pointer, n uintptr, ok unsafe.Pointer) int32 {
	return int32(C.zkvm_bls12_pairing((*C.zkvm_bls12_381_pairing_pair)(pairs), C.size_t(n),
		(*C._Bool)(ok)))
}

func bls12MapFpToG1(element, out unsafe.Pointer) int32 {
	return int32(C.zkvm_bls12_map_fp_to_g1((*C.zkvm_bls12_381_fp)(element),
		(*C.zkvm_bls12_381_g1_point)(out)))
}

func bls12MapFp2ToG2(element, out unsafe.Pointer) int32 {
	return int32(C.zkvm_bls12_map_fp2_to_g2((*C.zkvm_bls12_381_fp2)(element),
		(*C.zkvm_bls12_381_g2_point)(out)))
}

// ---- zkvm_u256.h and zkvm_u256_le.h: every call returns ZKVM_EOK ----

func u256Div(a, b, c unsafe.Pointer) {
	C.zkvm_u256_div((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Mod(a, b, c unsafe.Pointer) {
	C.zkvm_u256_mod((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256DivMod(a, b, c, d unsafe.Pointer) {
	C.zkvm_u256_divmod((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c), (*C.zkvm_u256)(d))
}
func u256SDiv(a, b, c unsafe.Pointer) {
	C.zkvm_u256_sdiv((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256SMod(a, b, c unsafe.Pointer) {
	C.zkvm_u256_smod((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256SDivMod(a, b, c, d unsafe.Pointer) {
	C.zkvm_u256_sdivmod((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c), (*C.zkvm_u256)(d))
}
func u256Add(a, b, c unsafe.Pointer) {
	C.zkvm_u256_add((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Sub(a, b, c unsafe.Pointer) {
	C.zkvm_u256_sub((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Mul(a, b, c unsafe.Pointer) {
	C.zkvm_u256_mul((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256AddMod(a, b, c, d unsafe.Pointer) {
	C.zkvm_u256_addmod((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c), (*C.zkvm_u256)(d))
}
func u256MulMod(a, b, c, d unsafe.Pointer) {
	C.zkvm_u256_mulmod((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c), (*C.zkvm_u256)(d))
}
func u256Exp(a, b, c unsafe.Pointer) {
	C.zkvm_u256_exp((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Lt(a, b, c unsafe.Pointer) {
	C.zkvm_u256_lt((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Gt(a, b, c unsafe.Pointer) {
	C.zkvm_u256_gt((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256SLt(a, b, c unsafe.Pointer) {
	C.zkvm_u256_slt((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256SGt(a, b, c unsafe.Pointer) {
	C.zkvm_u256_sgt((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Eq(a, b, c unsafe.Pointer) {
	C.zkvm_u256_eq((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256IsZero(a, b unsafe.Pointer) { C.zkvm_u256_iszero((*C.zkvm_u256)(a), (*C.zkvm_u256)(b)) }
func u256And(a, b, c unsafe.Pointer) {
	C.zkvm_u256_and((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Or(a, b, c unsafe.Pointer) {
	C.zkvm_u256_or((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Xor(a, b, c unsafe.Pointer) {
	C.zkvm_u256_xor((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Not(a, b unsafe.Pointer) { C.zkvm_u256_not((*C.zkvm_u256)(a), (*C.zkvm_u256)(b)) }
func u256Byte(a, b, c unsafe.Pointer) {
	C.zkvm_u256_byte((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Shl(a, b, c unsafe.Pointer) {
	C.zkvm_u256_shl((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Shr(a, b, c unsafe.Pointer) {
	C.zkvm_u256_shr((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256Sar(a, b, c unsafe.Pointer) {
	C.zkvm_u256_sar((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}
func u256SignExtend(a, b, c unsafe.Pointer) {
	C.zkvm_u256_signextend((*C.zkvm_u256)(a), (*C.zkvm_u256)(b), (*C.zkvm_u256)(c))
}

func u256leDiv(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_div((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leMod(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_mod((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leDivMod(a, b, c, d unsafe.Pointer) {
	C.zkvm_u256_le_divmod((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c), (*C.zkvm_u256_le)(d))
}
func u256leSDiv(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_sdiv((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leSMod(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_smod((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leSDivMod(a, b, c, d unsafe.Pointer) {
	C.zkvm_u256_le_sdivmod((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c), (*C.zkvm_u256_le)(d))
}
func u256leAdd(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_add((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leSub(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_sub((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leMul(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_mul((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leAddMod(a, b, c, d unsafe.Pointer) {
	C.zkvm_u256_le_addmod((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c), (*C.zkvm_u256_le)(d))
}
func u256leMulMod(a, b, c, d unsafe.Pointer) {
	C.zkvm_u256_le_mulmod((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c), (*C.zkvm_u256_le)(d))
}
func u256leExp(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_exp((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leLt(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_lt((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leGt(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_gt((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leSLt(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_slt((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leSGt(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_sgt((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leEq(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_eq((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leIsZero(a, b unsafe.Pointer) {
	C.zkvm_u256_le_iszero((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b))
}
func u256leAnd(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_and((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leOr(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_or((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leXor(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_xor((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leNot(a, b unsafe.Pointer) { C.zkvm_u256_le_not((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b)) }
func u256leByte(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_byte((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leShl(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_shl((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leShr(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_shr((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leSar(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_sar((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}
func u256leSignExtend(a, b, c unsafe.Pointer) {
	C.zkvm_u256_le_signextend((*C.zkvm_u256_le)(a), (*C.zkvm_u256_le)(b), (*C.zkvm_u256_le)(c))
}

// ---- zkvm_random.h and zkvm_io.h ----

func randomU64() uint64 { return uint64(C.zkvm_random_u64()) }

func writeLog(text unsafe.Pointer, n uintptr) { C.write_log((*C.uint8_t)(text), C.size_t(n)) }

// ---- zkvm_zisklib.h ----

func zisklibAdd(a, b uint64) uint64 { return uint64(C.zkvm_zisklib_add(C.uint64_t(a), C.uint64_t(b))) }

func u64s(p unsafe.Pointer) *C.uint64_t { return (*C.uint64_t)(p) }

func zisklibInv256(a, r unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_inv256(u64s(a), u64s(r)))
}

func zisklibOverflowingAdd256(a, b, r unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_overflowing_add256(u64s(a), u64s(b), u64s(r)))
}

func zisklibOverflowingSub256(a, b, r unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_overflowing_sub256(u64s(a), u64s(b), u64s(r)))
}

func zisklibOverflowingMul256(a, b, r unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_overflowing_mul256(u64s(a), u64s(b), u64s(r)))
}

func zisklibDivRem256(a, b, q, r unsafe.Pointer) {
	C.zkvm_zisklib_div_rem256(u64s(a), u64s(b), u64s(q), u64s(r))
}

func zisklibReduceMod256(a, m, r unsafe.Pointer) {
	C.zkvm_zisklib_reduce_mod256(u64s(a), u64s(m), u64s(r))
}

func zisklibAddMod256(a, b, m, r unsafe.Pointer) {
	C.zkvm_zisklib_add_mod256(u64s(a), u64s(b), u64s(m), u64s(r))
}

func zisklibMulMod256(a, b, m, r unsafe.Pointer) {
	C.zkvm_zisklib_mul_mod256(u64s(a), u64s(b), u64s(m), u64s(r))
}

func zisklibInvMod256(a, m, r unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_inv_mod256(u64s(a), u64s(m), u64s(r)))
}

func zisklibPowMod256(base, exp, m, r unsafe.Pointer) {
	C.zkvm_zisklib_pow_mod256(u64s(base), u64s(exp), u64s(m), u64s(r))
}

func zisklibOverflowingPow256(base, exp, r unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_overflowing_pow256(u64s(base), u64s(exp), u64s(r)))
}

func zisklibEcdsaVerifySecp256k1(pk, z, r, s unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_ecdsa_verify_secp256k1(u64s(pk), u64s(z), u64s(r), u64s(s)))
}

func zisklibEcdsaRecoverSecp256k1(r, s, z unsafe.Pointer, recid uint64, pk unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_ecdsa_recover_secp256k1(u64s(r), u64s(s), u64s(z),
		C.uint64_t(recid), u64s(pk)))
}

func zisklibSchnorrVerifySecp256k1(pkX, r, s, msg unsafe.Pointer, n uint64) uint64 {
	return uint64(C.zkvm_zisklib_schnorr_verify_secp256k1(u64s(pkX), u64s(r), u64s(s),
		(*C.uint8_t)(msg), C.uint64_t(n)))
}

func zisklibEcdsaVerifySecp256r1(pk, z, r, s unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_ecdsa_verify_secp256r1(u64s(pk), u64s(z), u64s(r), u64s(s)))
}

func zisklibPairingCheckBn254(g1, g2 unsafe.Pointer, n uint64) uint64 {
	return uint64(C.zkvm_zisklib_pairing_check_bn254(u64s(g1), u64s(g2), C.uint64_t(n)))
}

func zisklibPairingCheckBls12381(g1, g2 unsafe.Pointer, n uint64) uint64 {
	return uint64(C.zkvm_zisklib_pairing_check_bls12_381(u64s(g1), u64s(g2), C.uint64_t(n)))
}

func zisklibMapToCurveG1Bls12381(u, r unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_map_to_curve_g1_bls12_381(u64s(u), u64s(r)))
}

func zisklibMapToCurveG2Bls12381(u, r unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_map_to_curve_g2_bls12_381(u64s(u), u64s(r)))
}

func zisklibHashToCurveG2Bls12381(msg unsafe.Pointer, msgLen uint64, dst unsafe.Pointer,
	dstLen uint64, r unsafe.Pointer) {
	C.zkvm_zisklib_hash_to_curve_g2_bls12_381((*C.uint8_t)(msg), C.uint64_t(msgLen),
		(*C.uint8_t)(dst), C.uint64_t(dstLen), u64s(r))
}

func zisklibBlsVerifyBls12381(pk, msg unsafe.Pointer, n uint64, sig unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_bls_verify_bls12_381((*C.uint8_t)(pk), (*C.uint8_t)(msg),
		C.uint64_t(n), (*C.uint8_t)(sig)))
}

func zisklibVerifyKzgProofBls12381(z, y, commitment, proof unsafe.Pointer) uint64 {
	return uint64(C.zkvm_zisklib_verify_kzg_proof_bls12_381((*C.uint8_t)(z), (*C.uint8_t)(y),
		(*C.uint8_t)(commitment), (*C.uint8_t)(proof)))
}

func zisklibModexpU64(base unsafe.Pointer, baseLen uintptr, exp unsafe.Pointer, expLen uintptr,
	modulus unsafe.Pointer, modLen uintptr, r unsafe.Pointer) uintptr {
	return uintptr(C.zkvm_zisklib_modexp_u64_c(u64s(base), C.size_t(baseLen), u64s(exp),
		C.size_t(expLen), u64s(modulus), C.size_t(modLen), u64s(r)))
}

// ---- zkvm_fcall.h ----

func fcall(id uint32, params unsafe.Pointer, n uintptr) int32 {
	return int32(C.zkvm_fcall(C.uint32_t(id), u64s(params), C.size_t(n)))
}

func fcallGet() uint64 { return uint64(C.zkvm_fcall_get()) }

func fcallRead(results unsafe.Pointer, n uintptr) { C.zkvm_fcall_read(u64s(results), C.size_t(n)) }
