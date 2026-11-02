//go:build tinygo && wasip1

package zkvm

// The functions of the standard header, called through cgo. TinyGo compiles them as
// undefined C functions, which the wasm linker turns into env.zkvm_* imports for
// wasm2rom to implement.

// #cgo CFLAGS: -I../c/include
// #include "zkvm_accelerators.h"
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
