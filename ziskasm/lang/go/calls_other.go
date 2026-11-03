//go:build !(tinygo && wasip1)

package zkvm

// On other targets the functions exist only so that the package compiles (for
// go vet, editors and host-side code sharing types with a guest); calling one
// panics.

import "unsafe"

func onlyOnZisk() int32 {
	panic("zkvm: the ZisK functions only run on ZisK, in a TinyGo wasip1 guest")
}

func keccak256(unsafe.Pointer, uintptr, unsafe.Pointer) int32 { return onlyOnZisk() }
func keccakF1600(unsafe.Pointer) int32                        { return onlyOnZisk() }
func sha256(unsafe.Pointer, uintptr, unsafe.Pointer) int32    { return onlyOnZisk() }
func ripemd160(unsafe.Pointer, uintptr, unsafe.Pointer) int32 { return onlyOnZisk() }
func secp256k1Verify(_, _, _, _ unsafe.Pointer) int32         { return onlyOnZisk() }
func secp256k1Ecrecover(_, _ unsafe.Pointer, _ uint8, _ unsafe.Pointer) int32 {
	return onlyOnZisk()
}
func secp256r1Verify(_, _, _, _ unsafe.Pointer) int32 { return onlyOnZisk() }
func modexp(_ unsafe.Pointer, _ uintptr, _ unsafe.Pointer, _ uintptr, _ unsafe.Pointer, _ uintptr,
	_ unsafe.Pointer) int32 {
	return onlyOnZisk()
}
func bn254G1Add(_, _, _ unsafe.Pointer) int32                    { return onlyOnZisk() }
func bn254G1Mul(_, _, _ unsafe.Pointer) int32                    { return onlyOnZisk() }
func bn254Pairing(unsafe.Pointer, uintptr, unsafe.Pointer) int32 { return onlyOnZisk() }
func blake2f(_ uint32, _, _, _ unsafe.Pointer, _ uint8) int32    { return onlyOnZisk() }
func kzgPointEval(_, _, _, _, _ unsafe.Pointer) int32            { return onlyOnZisk() }
func bls12G1Add(_, _, _ unsafe.Pointer) int32                    { return onlyOnZisk() }
func bls12G1Msm(unsafe.Pointer, uintptr, unsafe.Pointer) int32   { return onlyOnZisk() }
func bls12G2Add(_, _, _ unsafe.Pointer) int32                    { return onlyOnZisk() }
func bls12G2Msm(unsafe.Pointer, uintptr, unsafe.Pointer) int32   { return onlyOnZisk() }
func bls12Pairing(unsafe.Pointer, uintptr, unsafe.Pointer) int32 { return onlyOnZisk() }
func bls12MapFpToG1(_, _ unsafe.Pointer) int32                   { return onlyOnZisk() }
func bls12MapFp2ToG2(_, _ unsafe.Pointer) int32                  { return onlyOnZisk() }

func u256Div(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256Mod(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256DivMod(_, _, _, _ unsafe.Pointer)  { onlyOnZisk() }
func u256SDiv(_, _, _ unsafe.Pointer)       { onlyOnZisk() }
func u256SMod(_, _, _ unsafe.Pointer)       { onlyOnZisk() }
func u256SDivMod(_, _, _, _ unsafe.Pointer) { onlyOnZisk() }
func u256Add(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256Sub(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256Mul(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256AddMod(_, _, _, _ unsafe.Pointer)  { onlyOnZisk() }
func u256MulMod(_, _, _, _ unsafe.Pointer)  { onlyOnZisk() }
func u256Exp(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256Lt(_, _, _ unsafe.Pointer)         { onlyOnZisk() }
func u256Gt(_, _, _ unsafe.Pointer)         { onlyOnZisk() }
func u256SLt(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256SGt(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256Eq(_, _, _ unsafe.Pointer)         { onlyOnZisk() }
func u256IsZero(_, _ unsafe.Pointer)        { onlyOnZisk() }
func u256And(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256Or(_, _, _ unsafe.Pointer)         { onlyOnZisk() }
func u256Xor(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256Not(_, _ unsafe.Pointer)           { onlyOnZisk() }
func u256Byte(_, _, _ unsafe.Pointer)       { onlyOnZisk() }
func u256Shl(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256Shr(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256Sar(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256SignExtend(_, _, _ unsafe.Pointer) { onlyOnZisk() }

func u256leDiv(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leMod(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leDivMod(_, _, _, _ unsafe.Pointer)  { onlyOnZisk() }
func u256leSDiv(_, _, _ unsafe.Pointer)       { onlyOnZisk() }
func u256leSMod(_, _, _ unsafe.Pointer)       { onlyOnZisk() }
func u256leSDivMod(_, _, _, _ unsafe.Pointer) { onlyOnZisk() }
func u256leAdd(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leSub(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leMul(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leAddMod(_, _, _, _ unsafe.Pointer)  { onlyOnZisk() }
func u256leMulMod(_, _, _, _ unsafe.Pointer)  { onlyOnZisk() }
func u256leExp(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leLt(_, _, _ unsafe.Pointer)         { onlyOnZisk() }
func u256leGt(_, _, _ unsafe.Pointer)         { onlyOnZisk() }
func u256leSLt(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leSGt(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leEq(_, _, _ unsafe.Pointer)         { onlyOnZisk() }
func u256leIsZero(_, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leAnd(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leOr(_, _, _ unsafe.Pointer)         { onlyOnZisk() }
func u256leXor(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leNot(_, _ unsafe.Pointer)           { onlyOnZisk() }
func u256leByte(_, _, _ unsafe.Pointer)       { onlyOnZisk() }
func u256leShl(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leShr(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leSar(_, _, _ unsafe.Pointer)        { onlyOnZisk() }
func u256leSignExtend(_, _, _ unsafe.Pointer) { onlyOnZisk() }

func randomU64() uint64                                            { onlyOnZisk(); return 0 }
func writeLog(unsafe.Pointer, uintptr)                             { onlyOnZisk() }
func zisklibAdd(_, _ uint64) uint64                                { onlyOnZisk(); return 0 }
func zisklibInv256(_, _ unsafe.Pointer) uint64                     { onlyOnZisk(); return 0 }
func zisklibOverflowingAdd256(_, _, _ unsafe.Pointer) uint64       { onlyOnZisk(); return 0 }
func zisklibOverflowingSub256(_, _, _ unsafe.Pointer) uint64       { onlyOnZisk(); return 0 }
func zisklibOverflowingMul256(_, _, _ unsafe.Pointer) uint64       { onlyOnZisk(); return 0 }
func zisklibDivRem256(_, _, _, _ unsafe.Pointer)                   { onlyOnZisk() }
func zisklibReduceMod256(_, _, _ unsafe.Pointer)                   { onlyOnZisk() }
func zisklibAddMod256(_, _, _, _ unsafe.Pointer)                   { onlyOnZisk() }
func zisklibMulMod256(_, _, _, _ unsafe.Pointer)                   { onlyOnZisk() }
func zisklibInvMod256(_, _, _ unsafe.Pointer) uint64               { onlyOnZisk(); return 0 }
func zisklibPowMod256(_, _, _, _ unsafe.Pointer)                   { onlyOnZisk() }
func zisklibOverflowingPow256(_, _, _ unsafe.Pointer) uint64       { onlyOnZisk(); return 0 }
func zisklibEcdsaVerifySecp256k1(_, _, _, _ unsafe.Pointer) uint64 { onlyOnZisk(); return 0 }
func zisklibEcdsaRecoverSecp256k1(_, _, _ unsafe.Pointer, _ uint64, _ unsafe.Pointer) uint64 {
	onlyOnZisk()
	return 0
}
func zisklibSchnorrVerifySecp256k1(_, _, _, _ unsafe.Pointer, _ uint64) uint64 {
	onlyOnZisk()
	return 0
}
func zisklibEcdsaVerifySecp256r1(_, _, _, _ unsafe.Pointer) uint64  { onlyOnZisk(); return 0 }
func zisklibPairingCheckBn254(_, _ unsafe.Pointer, _ uint64) uint64 { onlyOnZisk(); return 0 }
func zisklibPairingCheckBls12381(_, _ unsafe.Pointer, _ uint64) uint64 {
	onlyOnZisk()
	return 0
}
func zisklibMapToCurveG1Bls12381(_, _ unsafe.Pointer) uint64 { onlyOnZisk(); return 0 }
func zisklibMapToCurveG2Bls12381(_, _ unsafe.Pointer) uint64 { onlyOnZisk(); return 0 }
func zisklibHashToCurveG2Bls12381(_ unsafe.Pointer, _ uint64, _ unsafe.Pointer, _ uint64,
	_ unsafe.Pointer) {
	onlyOnZisk()
}
func zisklibBlsVerifyBls12381(_, _ unsafe.Pointer, _ uint64, _ unsafe.Pointer) uint64 {
	onlyOnZisk()
	return 0
}
func zisklibVerifyKzgProofBls12381(_, _, _, _ unsafe.Pointer) uint64 { onlyOnZisk(); return 0 }
func zisklibModexpU64(_ unsafe.Pointer, _ uintptr, _ unsafe.Pointer, _ uintptr, _ unsafe.Pointer,
	_ uintptr, _ unsafe.Pointer) uintptr {
	onlyOnZisk()
	return 0
}
func fcall(uint32, unsafe.Pointer, uintptr) int32 { return onlyOnZisk() }
func fcallGet() uint64                            { onlyOnZisk(); return 0 }
func fcallRead(unsafe.Pointer, uintptr)           { onlyOnZisk() }
