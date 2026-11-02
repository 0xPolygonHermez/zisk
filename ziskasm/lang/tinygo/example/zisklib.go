package main

import (
	"slices"

	"github.com/0xPolygonHermez/zisk/ziskasm/lang/tinygo"
)

// checkZisklib calls every function of zkvm_zisklib.h, and a few of them on inputs
// they must reject.
func checkZisklib() {
	check("zisklib_add", zkvm.ZisklibAdd(1<<40, 5) == 1<<40+5)

	inv, ok := zkvm.Inv256(&zA)
	check("zisklib_inv256", ok && inv == zInv256)
	_, ok = zkvm.Inv256(&zAEven)
	check("zisklib_inv256 of an even value", !ok)
	ones, two, one := [4]uint64{^uint64(0), ^uint64(0), ^uint64(0), ^uint64(0)}, [4]uint64{2}, [4]uint64{1}
	sum, carry := zkvm.OverflowingAdd256(&ones, &two)
	check("zisklib_overflowing_add256", carry && sum == zAddWrap)
	diff, borrow := zkvm.OverflowingSub256(&one, &two)
	check("zisklib_overflowing_sub256", borrow && diff == zSubWrap)
	product, over := zkvm.OverflowingMul256(&z2p200, &z2p100)
	check("zisklib_overflowing_mul256", over && product == zMulOver)
	q, r := zkvm.DivRem256(&zA, &zB)
	check("zisklib_div_rem256", q == zQ && r == zR)
	check("zisklib_reduce_mod256", zkvm.ReduceMod256(&zA, &zM) == zReduce)
	check("zisklib_add_mod256", zkvm.AddMod256(&zA, &zB, &zM) == zAddMod)
	check("zisklib_mul_mod256", zkvm.MulMod256(&zA, &zB, &zM) == zMulMod)
	inv, ok = zkvm.InvMod256(&zA, &zM)
	check("zisklib_inv_mod256", ok && inv == zInvMod)
	six, nine := [4]uint64{6}, [4]uint64{9}
	_, ok = zkvm.InvMod256(&six, &nine)
	check("zisklib_inv_mod256 without an inverse", !ok)
	check("zisklib_pow_mod256", zkvm.PowMod256(&zA, &zB, &zM) == zPowMod)
	three, e100, e200 := [4]uint64{3}, [4]uint64{100}, [4]uint64{200}
	pow, over := zkvm.OverflowingPow256(&three, &e100)
	check("zisklib_overflowing_pow256", !over && pow == zPow3x100)
	pow, over = zkvm.OverflowingPow256(&three, &e200)
	check("zisklib_overflowing_pow256 overflow", over && pow == zPow3x200)

	check("zisklib_ecdsa_verify_secp256k1", zkvm.EcdsaVerifySecp256k1(&zK1Pub, &zK1Z, &zK1R, &zK1S))
	badZ := zK1Z
	badZ[0] ^= 1
	check("zisklib_ecdsa_verify_secp256k1 rejects", !zkvm.EcdsaVerifySecp256k1(&zK1Pub, &badZ, &zK1R, &zK1S))
	pk, err := zkvm.EcdsaRecoverSecp256k1(&zRecR, &zRecS, &zRecZ, 1)
	check("zisklib_ecdsa_recover_secp256k1", err == nil && pk == zRecPub)
	_, err = zkvm.EcdsaRecoverSecp256k1(&zRecR, &zRecS, &zRecZ, 2)
	check("zisklib_ecdsa_recover_secp256k1 recid 2", err == zkvm.StatusError(3))
	check("zisklib_schnorr_verify_secp256k1",
		zkvm.SchnorrVerifySecp256k1(&zSchnorrPk, &zSchnorrR, &zSchnorrS, zSchnorrMsg))
	check("zisklib_schnorr_verify_secp256k1 rejects",
		!zkvm.SchnorrVerifySecp256k1(&zSchnorrPk, &zSchnorrR, &zSchnorrS, zSchnorrMsg[1:]))
	check("zisklib_ecdsa_verify_secp256r1", zkvm.EcdsaVerifySecp256r1(&zR1Pub, &zR1Z, &zR1R, &zR1S))

	ok, err = zkvm.PairingCheckBn254(zBnValidG1, zBnValidG2)
	check("zisklib_pairing_check_bn254", ok && err == nil)
	ok, err = zkvm.PairingCheckBn254(zBnInvalidG1, zBnInvalidG2)
	check("zisklib_pairing_check_bn254 rejects", !ok && err == nil)
	ok, err = zkvm.PairingCheckBls12381(zBlsValidG1, zBlsValidG2)
	check("zisklib_pairing_check_bls12_381", ok && err == nil)
	ok, err = zkvm.PairingCheckBls12381(zBlsInvalidG1, zBlsInvalidG2)
	check("zisklib_pairing_check_bls12_381 rejects", !ok && err == nil)
	g1, err := zkvm.MapToCurveG1Bls12381(&zMapU1)
	check("zisklib_map_to_curve_g1_bls12_381", err == nil && g1 == zMapG1)
	g2, err := zkvm.MapToCurveG2Bls12381(&zMapU2)
	check("zisklib_map_to_curve_g2_bls12_381", err == nil && g2 == zMapG2)
	check("zisklib_hash_to_curve_g2_bls12_381", zkvm.HashToCurveG2Bls12381(zHashMsg, zHashDst) == zHashG2)
	check("zisklib_bls_verify_bls12_381", zkvm.BlsVerifyBls12381(&zBlsPk, zBlsMsg, &zBlsSig))
	check("zisklib_bls_verify_bls12_381 rejects", !zkvm.BlsVerifyBls12381(&zBlsPk, zBlsMsg[1:], &zBlsSig))
	check("zisklib_verify_kzg_proof_bls12_381", zkvm.VerifyKzgProofBls12381(&kzgZ, &kzgY, &kzgC, &kzgP))
	check("zisklib_verify_kzg_proof_bls12_381 rejects",
		!zkvm.VerifyKzgProofBls12381(&kzgZ, &kzgYB, &kzgC, &kzgP))
	check("zisklib_modexp_u64_c", slices.Equal(zkvm.ModexpU64([]uint64{3}, []uint64{2}, []uint64{5}),
		[]uint64{4, 0, 0, 0}))
	check("zisklib_modexp_u64_c 5 limbs", slices.Equal(zkvm.ModexpU64(zModexpBase, zModexpExp, zModexpMod),
		zModexpWant))
}
