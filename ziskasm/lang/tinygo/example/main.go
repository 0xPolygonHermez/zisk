// A TinyGo guest that calls every accelerator of zkvm_accelerators.h through the
// zkvm package and checks each result against a known answer (see vectors.go). It
// prints "zkvm: N checks passed" and exits 0, or names each failed check and exits 1.
//
// Build and run (see ../README.md):
//
//	tinygo build -target=wasip1 -no-debug -opt=z -scheduler=none -panic=trap -o zkvm-example.wasm .
//	ziskemu -e zkvm-example.wasm -c
//
// It avoids fmt, whose float formatting would need a float build of ZisK.
package main

import (
	"bytes"
	"os"
	"strconv"
	"unsafe"

	"github.com/0xPolygonHermez/zisk/ziskasm/lang/tinygo"
)

var passed, failed int

func check(name string, ok bool) {
	if ok {
		passed++
		return
	}
	failed++
	os.Stdout.WriteString("FAIL " + name + "\n")
}

func hexEq(got []byte, want string) bool {
	const digits = "0123456789abcdef"
	if len(want) != 2*len(got) {
		return false
	}
	for i, b := range got {
		if want[2*i] != digits[b>>4] || want[2*i+1] != digits[b&15] {
			return false
		}
	}
	return true
}

// pairs reinterprets a flat byte vector as a slice of n-byte pair structs.
func pairs[T any](flat []byte) []T {
	var zero T
	size := int(unsafe.Sizeof(zero))
	out := make([]T, len(flat)/size)
	copy(unsafe.Slice((*byte)(unsafe.Pointer(unsafe.SliceData(out))), len(flat)), flat)
	return out
}

func main() {
	abc := []byte("xabc")[1:] // starts at an odd offset: no alignment is required

	// Hashes and keccak-f.
	k := zkvm.Keccak256(abc)
	check("keccak256", hexEq(k[:], "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45"))
	s := zkvm.Sha256(abc)
	check("sha256", hexEq(s[:], "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"))
	r := zkvm.Ripemd160(abc)
	check("ripemd160", hexEq(r[:], "8eb208f7e05d987a9b044a8e98c6b087f15a0bfc"))
	e := zkvm.Keccak256(nil)
	check("keccak256 empty", hexEq(e[:], "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"))
	var state [25]uint64
	zkvm.KeccakF1600(&state)
	check("keccak_f1600", state[0] == 0xF1258F7940E1DDE7)

	// secp256k1 and secp256r1.
	ok, err := zkvm.Secp256k1Verify(&k1Msg, &k1Sig, &k1Pub)
	check("secp256k1_verify", ok && err == nil)
	badSig := k1Sig
	badSig[40] ^= 1
	ok, err = zkvm.Secp256k1Verify(&k1Msg, &badSig, &k1Pub)
	check("secp256k1_verify rejects a bad signature", !ok || err != nil)
	pub, err := zkvm.Secp256k1Ecrecover(&recMsg, &recSig, 1)
	check("secp256k1_ecrecover", err == nil && pub == recPub)
	_, err = zkvm.Secp256k1Ecrecover(&recMsg, &recSig, 2)
	check("secp256k1_ecrecover rejects recid 2", err == zkvm.ErrFailed)
	ok, err = zkvm.Secp256r1Verify(&r1Msg, &r1Sig, &r1Pub)
	check("secp256r1_verify", ok && err == nil)

	// modexp.
	out, err := zkvm.Modexp([]byte{3}, []byte{2}, []byte{5})
	check("modexp 3^2 mod 5", err == nil && bytes.Equal(out, []byte{4}))
	out, err = zkvm.Modexp(modV2Base, modV2Exp, modV2Mod)
	check("modexp 32-byte", err == nil && bytes.Equal(out, modV2Want))
	out, err = zkvm.Modexp(modV3Base, modV3Exp, modV3Mod)
	check("modexp 48-byte", err == nil && bytes.Equal(out, modV3Want))
	out, err = zkvm.Modexp(modV4Base, modV4Exp, modV4Mod)
	check("modexp 40-byte", err == nil && bytes.Equal(out, modV4Want))
	out, err = zkvm.Modexp([]byte{3}, []byte{2}, nil)
	check("modexp empty modulus", err == nil && len(out) == 0)

	// BN254.
	sum, err := zkvm.Bn254G1Add(&bnAddP1, &bnAddP2)
	check("bn254_g1_add", err == nil && sum == bnAddSum)
	product, err := zkvm.Bn254G1Mul(&bnMulP, &bnMulK)
	check("bn254_g1_mul", err == nil && product == bnMulProduct)
	ok, err = zkvm.Bn254Pairing(pairs[zkvm.Bn254PairingPair](bnPairsValid[:]))
	check("bn254_pairing valid", ok && err == nil)
	ok, err = zkvm.Bn254Pairing(pairs[zkvm.Bn254PairingPair](bnPairsInvalid[:]))
	check("bn254_pairing invalid", !ok && err == nil)

	// BLAKE2b compression (EIP-152 vector: one final block of "abc").
	h := blakeH
	err = zkvm.Blake2f(12, &h, &blakeM, &blakeT, true)
	check("blake2f", err == nil && h == blakeWant)

	// BLS12-381 and KZG.
	g1, err := zkvm.Bls12G1Add(&blsA1, &blsB1)
	check("bls12_g1_add", err == nil && g1 == blsEr1)
	g2, err := zkvm.Bls12G2Add(&blsA2, &blsB2)
	check("bls12_g2_add", err == nil && g2 == blsEr2)
	g1, err = zkvm.Bls12G1Msm(pairs[zkvm.Bls12G1MsmPair](blsMsm1[:]))
	check("bls12_g1_msm", err == nil && g1 == blsErm1)
	g2, err = zkvm.Bls12G2Msm(pairs[zkvm.Bls12G2MsmPair](blsMsm2[:]))
	check("bls12_g2_msm", err == nil && g2 == blsErm2)
	ok, err = zkvm.Bls12Pairing(pairs[zkvm.Bls12PairingPair](blsPairA[:]))
	check("bls12_pairing valid", ok && err == nil)
	ok, err = zkvm.Bls12Pairing(pairs[zkvm.Bls12PairingPair](blsPairR[:]))
	check("bls12_pairing invalid", !ok && err == nil)
	g1, err = zkvm.Bls12MapFpToG1(&mapU1)
	check("bls12_map_fp_to_g1", err == nil && g1 == mapEg1)
	g2, err = zkvm.Bls12MapFp2ToG2(&mapU2)
	check("bls12_map_fp2_to_g2", err == nil && g2 == mapEg2)
	ok, err = zkvm.KzgPointEval(&kzgC, &kzgZ, &kzgY, &kzgP)
	check("kzg_point_eval valid", ok && err == nil)
	ok, err = zkvm.KzgPointEval(&kzgC, &kzgZ, &kzgYB, &kzgP)
	check("kzg_point_eval invalid", !ok)

	if failed > 0 {
		os.Stdout.WriteString("zkvm: " + strconv.Itoa(failed) + " checks failed\n")
		os.Exit(1)
	}
	os.Stdout.WriteString("zkvm: " + strconv.Itoa(passed) + " checks passed\n")
}
