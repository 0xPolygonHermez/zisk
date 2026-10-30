/-
  Every operation of sail-zisk's `zisk_exec_mem` but the three that write
  the fcall registers leaves them as they are (`Keep.exec_mem`).
-/
import Equiv.ZKeep

set_option maxRecDepth 1000000

namespace Equiv.Zisk

open _root_.Zisk

set_option maxHeartbeats 0 in
theorem Keep.exec_dma_memcpy (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_DMA_MEMCPY ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_dma_xmemcpy (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_DMA_XMEMCPY ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_dma_memcmp (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_DMA_MEMCMP ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_dma_xmemcmp (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_DMA_XMEMCMP ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_dma_xmemset (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_DMA_XMEMSET ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_fcall_param (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_FCALL_PARAM ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_jump_dest (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_JUMP_DEST ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_add256 (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_ADD256 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_arith256 (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_ARITH256 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_arith256_mod (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_ARITH256_MOD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_arith384_mod (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_ARITH384_MOD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_secp256k1_add (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_SECP256K1_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_secp256k1_dbl (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_SECP256K1_DBL ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_secp256r1_add (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_SECP256R1_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_secp256r1_dbl (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_SECP256R1_DBL ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_bn254_curve_add (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BN254_CURVE_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_bn254_curve_dbl (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BN254_CURVE_DBL ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_bn254_complex_add (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BN254_COMPLEX_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_bn254_complex_sub (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BN254_COMPLEX_SUB ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_bn254_complex_mul (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BN254_COMPLEX_MUL ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_bls12_381_curve_add (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BLS12_381_CURVE_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_bls12_381_curve_dbl (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BLS12_381_CURVE_DBL ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_bls12_381_complex_add (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BLS12_381_COMPLEX_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_bls12_381_complex_sub (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BLS12_381_COMPLEX_SUB ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_bls12_381_complex_mul (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BLS12_381_COMPLEX_MUL ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_ed25519_add (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_ED25519_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_babyjubjub_add (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BABYJUBJUB_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_keccak (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_KECCAK ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_sha256 (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_SHA256 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_blake2b (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BLAKE2B ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_blake2s (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BLAKE2S ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_blake3 (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_BLAKE3 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_poseidon2 (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_POSEIDON2 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_poseidon1 (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_POSEIDON1 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

set_option maxHeartbeats 0 in
theorem Keep.exec_poseidon2_koalabear (a b ext : BitVec 64) : Keep (Functions.zisk_exec_mem (.ZOP_POSEIDON2_KOALABEAR ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  keep

theorem Keep.exec_mem (op : zisk_op) (h : writesF op = false) (a b ext : BitVec 64) :
    Keep (Functions.zisk_exec_mem op a b ext) := by
  cases op
  all_goals first
    | with_reducible exact Keep.exec_dma_memcpy a b ext
    | with_reducible exact Keep.exec_dma_xmemcpy a b ext
    | with_reducible exact Keep.exec_dma_memcmp a b ext
    | with_reducible exact Keep.exec_dma_xmemcmp a b ext
    | with_reducible exact Keep.exec_dma_xmemset a b ext
    | with_reducible exact Keep.exec_fcall_param a b ext
    | with_reducible exact Keep.exec_jump_dest a b ext
    | with_reducible exact Keep.exec_add256 a b ext
    | with_reducible exact Keep.exec_arith256 a b ext
    | with_reducible exact Keep.exec_arith256_mod a b ext
    | with_reducible exact Keep.exec_arith384_mod a b ext
    | with_reducible exact Keep.exec_secp256k1_add a b ext
    | with_reducible exact Keep.exec_secp256k1_dbl a b ext
    | with_reducible exact Keep.exec_secp256r1_add a b ext
    | with_reducible exact Keep.exec_secp256r1_dbl a b ext
    | with_reducible exact Keep.exec_bn254_curve_add a b ext
    | with_reducible exact Keep.exec_bn254_curve_dbl a b ext
    | with_reducible exact Keep.exec_bn254_complex_add a b ext
    | with_reducible exact Keep.exec_bn254_complex_sub a b ext
    | with_reducible exact Keep.exec_bn254_complex_mul a b ext
    | with_reducible exact Keep.exec_bls12_381_curve_add a b ext
    | with_reducible exact Keep.exec_bls12_381_curve_dbl a b ext
    | with_reducible exact Keep.exec_bls12_381_complex_add a b ext
    | with_reducible exact Keep.exec_bls12_381_complex_sub a b ext
    | with_reducible exact Keep.exec_bls12_381_complex_mul a b ext
    | with_reducible exact Keep.exec_ed25519_add a b ext
    | with_reducible exact Keep.exec_babyjubjub_add a b ext
    | with_reducible exact Keep.exec_keccak a b ext
    | with_reducible exact Keep.exec_sha256 a b ext
    | with_reducible exact Keep.exec_blake2b a b ext
    | with_reducible exact Keep.exec_blake2s a b ext
    | with_reducible exact Keep.exec_blake3 a b ext
    | with_reducible exact Keep.exec_poseidon2 a b ext
    | with_reducible exact Keep.exec_poseidon1 a b ext
    | with_reducible exact Keep.exec_poseidon2_koalabear a b ext
    | (simp [writesF] at h; done)
    | (simp only [Functions.zisk_exec_mem]; exact Keep.pure _)

set_option maxHeartbeats 0 in
/-- A ZisK instruction whose operation does not write the fcall registers
    leaves them as they are. -/
theorem Keep.execute (inst : zisk_inst) (h : writesF inst.op = false) : Keep (Functions.zisk_execute inst) := by
  unfold Functions.zisk_execute
  refine Keep.bind (by unfold Functions.source_a; keep) (fun a => ?_)
  refine Keep.bind (by unfold Functions.source_b; keep) (fun b => ?_)
  dsimp only
  split
  · refine Keep.bind (by unfold Functions.precompiled_prelude Functions.blake2b_chain; keep) (fun _ => ?_)
    refine Keep.bind (Keep.bind (Keep.exec_mem _ h _ _ _) (fun r => by keep)) (fun cf => ?_)
    keep
  · refine Keep.bind (Keep.pure _) (fun _ => ?_)
    refine Keep.bind (Keep.bind (Keep.exec_mem _ h _ _ _) (fun r => by keep)) (fun cf => ?_)
    keep

end Equiv.Zisk
