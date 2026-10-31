/-
  Every operation of sail-zisk's `zisk_exec_mem` depends on memory and the
  fcall registers alone (`Frame`): those with memory, each in its module of
  Equiv/Frame/; the others, none.
-/
import Equiv.Frame.Dma
import Equiv.Frame.Fcall
import Equiv.Frame.Arith
import Equiv.Frame.Secp
import Equiv.Frame.Bn254
import Equiv.Frame.Bls
import Equiv.Frame.Edwards
import Equiv.Frame.Hash

namespace Equiv.Zisk

open _root_.Zisk

theorem Frame.exec_mem (op : zisk_op) (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem op a b ext) := by
  cases op
  all_goals first
    | with_reducible exact Frame.exec_dma_memcpy a b ext
    | with_reducible exact Frame.exec_dma_xmemcpy a b ext
    | with_reducible exact Frame.exec_dma_memcmp a b ext
    | with_reducible exact Frame.exec_dma_xmemcmp a b ext
    | with_reducible exact Frame.exec_dma_xmemset a b ext
    | with_reducible exact Frame.exec_fcall_param a b ext
    | with_reducible exact Frame.exec_fcall a b ext
    | with_reducible exact Frame.exec_fcall_get a b ext
    | with_reducible exact Frame.exec_dma_inputcpy a b ext
    | with_reducible exact Frame.exec_jump_dest a b ext
    | with_reducible exact Frame.exec_add256 a b ext
    | with_reducible exact Frame.exec_arith256 a b ext
    | with_reducible exact Frame.exec_arith256_mod a b ext
    | with_reducible exact Frame.exec_arith384_mod a b ext
    | with_reducible exact Frame.exec_secp256k1_add a b ext
    | with_reducible exact Frame.exec_secp256k1_dbl a b ext
    | with_reducible exact Frame.exec_secp256r1_add a b ext
    | with_reducible exact Frame.exec_secp256r1_dbl a b ext
    | with_reducible exact Frame.exec_bn254_curve_add a b ext
    | with_reducible exact Frame.exec_bn254_curve_dbl a b ext
    | with_reducible exact Frame.exec_bn254_complex_add a b ext
    | with_reducible exact Frame.exec_bn254_complex_sub a b ext
    | with_reducible exact Frame.exec_bn254_complex_mul a b ext
    | with_reducible exact Frame.exec_bls12_381_curve_add a b ext
    | with_reducible exact Frame.exec_bls12_381_curve_dbl a b ext
    | with_reducible exact Frame.exec_bls12_381_complex_add a b ext
    | with_reducible exact Frame.exec_bls12_381_complex_sub a b ext
    | with_reducible exact Frame.exec_bls12_381_complex_mul a b ext
    | with_reducible exact Frame.exec_ed25519_add a b ext
    | with_reducible exact Frame.exec_babyjubjub_add a b ext
    | with_reducible exact Frame.exec_keccak a b ext
    | with_reducible exact Frame.exec_sha256 a b ext
    | with_reducible exact Frame.exec_blake2b a b ext
    | with_reducible exact Frame.exec_blake2s a b ext
    | with_reducible exact Frame.exec_blake3 a b ext
    | with_reducible exact Frame.exec_poseidon2 a b ext
    | with_reducible exact Frame.exec_poseidon1 a b ext
    | with_reducible exact Frame.exec_poseidon2_koalabear a b ext
    | (simp only [Functions.zisk_exec_mem]; exact Frame.pure _)

end Equiv.Zisk
