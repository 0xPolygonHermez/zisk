package main

import (
	"bytes"
	"slices"

	"github.com/0xPolygonHermez/zisk/ziskasm/lang/tinygo"
)

// checkHints issues every hint of zkvm_fcall.h and compares it with the value the
// Python models computed (either root, for the Fp2 square root, which has no
// canonical one). A real guest would check each hint against its definition
// instead, since the prover chooses it.
func checkHints() {
	check("fcall rejects an executor ID", zkvm.Fcall(27, nil) == zkvm.ErrFcallRejected)
	check("fcall rejects too many parameters",
		zkvm.Fcall(zkvm.FcallBn254FpInv, make([]uint64, zkvm.FcallMaxParams+1)) == zkvm.ErrFcallRejected)

	check("hint secp256k1_fp_inv", zkvm.HintSecp256k1FpInv(&hX) == hK1FpInv)
	check("hint secp256k1_fn_inv", zkvm.HintSecp256k1FnInv(&hX) == hK1FnInv)
	root, isQR := zkvm.HintSecp256k1FpSqrt(&hK1Square, true)
	check("hint secp256k1_fp_sqrt", isQR && root == hK1OddRoot)
	three := [4]uint64{3}
	root, isQR = zkvm.HintSecp256k1FpSqrt(&three, false)
	check("hint secp256k1_fp_sqrt of a non-residue", !isQR && root == hK1NonResidueRoot)
	k1, k2, neg1, neg2 := zkvm.HintSecp256k1GlvDecompose(&hGlvK)
	check("hint secp256k1_glv_decompose", k1 == hGlvK1 && k2 == hGlvK2 && neg1 == hGlvNeg1 && neg2 == hGlvNeg2)
	check("hint secp256r1_fn_inv", zkvm.HintSecp256r1FnInv(&hX) == hR1FnInv)

	check("hint bn254_fp_inv", zkvm.HintBn254FpInv(&hX) == hBnFpInv)
	check("hint bn254_fp2_inv", zkvm.HintBn254Fp2Inv(&hBnFp2) == hBnFp2Inv)
	lambda, mu := zkvm.HintBn254TwistAddLineCoeffs(&hBnX1, &hBnY1, &hBnX2, &hBnY2)
	check("hint bn254_twist_add_line_coeffs", lambda == hBnAddLambda && mu == hBnAddMu)
	lambda, mu = zkvm.HintBn254TwistDblLineCoeffs(&hBnX1, &hBnY1)
	check("hint bn254_twist_dbl_line_coeffs", lambda == hBnDblLambda && mu == hBnDblMu)

	check("hint bls12_381_fp_inv", zkvm.HintBls12381FpInv(&hBlsX) == hBlsFpInv)
	root6, isQR := zkvm.HintBls12381FpSqrt(&hBlsSquare)
	check("hint bls12_381_fp_sqrt", isQR && root6 == hBlsRoot)
	check("hint bls12_381_fp2_inv", zkvm.HintBls12381Fp2Inv(&hBlsFp2) == hBlsFp2Inv)
	root12, isQR := zkvm.HintBls12381Fp2Sqrt(&hBlsFp2Square)
	check("hint bls12_381_fp2_sqrt", isQR && (root12 == hBlsFp2Root || root12 == hBlsFp2NegRoot))
	lambda12, mu12 := zkvm.HintBls12381TwistAddLineCoeffs(&hBlsX1, &hBlsY1, &hBlsX2, &hBlsY2)
	check("hint bls12_381_twist_add_line_coeffs", lambda12 == hBlsAddLambda && mu12 == hBlsAddMu)
	lambda12, mu12 = zkvm.HintBls12381TwistDblLineCoeffs(&hBlsX1, &hBlsY1)
	check("hint bls12_381_twist_dbl_line_coeffs", lambda12 == hBlsDblLambda && mu12 == hBlsDblMu)

	check("hint bin_decomp", bytes.Equal(zkvm.HintBinDecomp(hBinDecompX), hBinDecompBits))
	limb, bit := zkvm.HintMsbPos256(&hMsbA, &hMsbB)
	check("hint msb_pos_256", limb == 2 && bit == 2)
	limb, bit = zkvm.HintMsbPos384(&hMsb384X, &hMsb384Y)
	check("hint msb_pos_384", limb == 4 && bit == 44)
	q, r := zkvm.HintUint256Div(&zA, &zB)
	check("hint uint256_div", q == hDivQ && r == hDivR)
	inv, ok := zkvm.HintUint256Inv(&zA)
	check("hint uint256_inv", ok && inv == hInv)
	ok, inv, _, _ = zkvm.HintUint256InvMod(&zA, &zM)
	check("hint uint256_inv_mod", ok && inv == hInvMod)
	ok, gcd, qa, qm := zkvm.HintUint256InvMod(&hSix, &hNine)
	check("hint uint256_inv_mod witness", !ok && gcd == [4]uint64{3} && qa == [4]uint64{2} && qm == [4]uint64{3})
	bq, br := zkvm.HintBigintDiv(hBigA, hBigB)
	check("hint bigint_div", slices.Equal(bq, hBigQ) && slices.Equal(br, hBigR))
	root, isSquare := zkvm.HintEd25519FpSqrtRatio(&hEdNine, &hEdOne)
	check("hint ed25519_fp_sqrt_ratio", isSquare && root == hEdRoot9)
	root, isSquare = zkvm.HintEd25519FpSqrtRatio(&hEdTwo, &hEdOne)
	check("hint ed25519_fp_sqrt_ratio of a non-square", !isSquare && root == hEdRoot4)

	// Diagnostic text: printed by ziskemu --write-log, invisible to the output and the proof.
	// (RandomU64 is left out: a guest that draws random values needs the host's seed, and
	// cannot run with hints.)
	zkvm.WriteLog("zkvm: a diagnostic line, not part of the output\n")
}
