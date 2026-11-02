//go:build !(tinygo && wasip1)

package zkvm

// On other targets the functions exist only so that the package compiles (for
// go vet, editors and host-side code sharing types with a guest); calling one
// panics.

import "unsafe"

func onlyOnZisk() int32 {
	panic("zkvm: the accelerators only run on ZisK, in a TinyGo wasip1 guest")
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
