use std::{cell::UnsafeCell, io, thread::JoinHandle};

#[cfg(feature = "hints-metrics")]
use crate::hints::hint::HintKind;

pub const HINT_START: u32 = 0;
pub const HINT_END: u32 = 1;
// const HINT_CANCEL: u32 = 2;
// const HINT_ERROR: u32 = 3;
pub const HINTS_TYPE_RESULT: u32 = 4;
pub const HINTS_TYPE_ECRECOVER: u32 = 5;
pub const HINTS_TYPE_REDMOD256: u32 = 6;
pub const HINTS_TYPE_ADDMOD256: u32 = 7;
pub const HINTS_TYPE_MULMOD256: u32 = 8;
pub const HINTS_TYPE_DIVREM256: u32 = 9;
pub const HINTS_TYPE_WPOW256: u32 = 10;
pub const HINTS_TYPE_OMUL256: u32 = 11;
pub const HINTS_TYPE_WMUL256: u32 = 12;
pub const HINTS_TYPE_MODEXP: u32 = 13;
pub const HINTS_TYPE_IS_ON_CURVE_BN254: u32 = 14;
pub const HINTS_TYPE_TO_AFFINE_BN254: u32 = 15;
pub const HINTS_TYPE_ADD_BN254: u32 = 16;
pub const HINTS_TYPE_MUL_BN254: u32 = 17;
pub const HINTS_TYPE_TO_AFFINE_TWIST_BN254: u32 = 18;
pub const HINTS_TYPE_IS_ON_CURVE_TWIST_BN254: u32 = 19;
pub const HINTS_TYPE_IS_ON_SUBGROUP_TWIST_BN254: u32 = 20;
pub const HINTS_TYPE_PAIRING_BATCH_BN254: u32 = 21;

// BLS12-381 hint codes
pub const HINT_MUL_FP12_BLS12_381: u32 = 0x16;
pub const HINT_DECOMPRESS_BLS12_381: u32 = 0x17;
pub const HINT_IS_ON_CURVE_BLS12_381: u32 = 0x18;
pub const HINT_IS_ON_SUBGROUP_BLS12_381: u32 = 0x19;
pub const HINT_ADD_BLS12_381: u32 = 0x1A;
pub const HINT_SCALAR_MUL_BLS12_381: u32 = 0x1B;
pub const HINT_DECOMPRESS_TWIST_BLS12_381: u32 = 0x1C;
pub const HINT_IS_ON_CURVE_TWIST_BLS12_381: u32 = 0x1D;
pub const HINT_IS_ON_SUBGROUP_TWIST_BLS12_381: u32 = 0x1E;
pub const HINT_ADD_TWIST_BLS12_381: u32 = 0x1F;
pub const HINT_SCALAR_MUL_TWIST_BLS12_381: u32 = 0x20;
pub const HINT_MILLER_LOOP_BLS12_381: u32 = 0x21;
pub const HINT_FINAL_EXP_BLS12_381: u32 = 0x22;

pub const HINT_WRITE_BATCH: usize = 64;

pub struct HintFileWriterHandleCell {
    inner: UnsafeCell<Option<JoinHandle<io::Result<()>>>>,
}

unsafe impl Sync for HintFileWriterHandleCell {}

impl HintFileWriterHandleCell {
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(None),
        }
    }

    pub fn take(&self) -> Option<JoinHandle<io::Result<()>>> {
        unsafe { (*self.inner.get()).take() }
    }

    pub fn store(&self, handle: JoinHandle<io::Result<()>>) {
        // Safety: caller guarantees single-threaded access when mutating the handle.
        unsafe {
            *self.inner.get() = Some(handle);
        }
    }
}
pub trait HintData {
    fn header_and_payload(&self) -> ([u8; 8], &[u8]);
}

#[cfg(feature = "hints-metrics")]
#[derive(Default, Debug)]
pub struct HintTotals {
    keccakf: u64,
    sha2: u64,
    ecrecover: u64,
    redmod256: u64,
    addmod256: u64,
    mulmod256: u64,
    divrem256: u64,
    wpow256: u64,
    omul256: u64,
    wmul256: u64,
    modexp: u64,
    is_on_curve_bn254: u64,
    to_affine_bn254: u64,
    add_bn254: u64,
    mul_bn254: u64,
    to_affine_twist_bn254: u64,
    is_on_curve_twist_bn254: u64,
    is_on_subgroup_twist_bn254: u64,
    pairing_batch_bn254: u64,
    mul_fp12_bls12_381: u64,
    is_on_curve_bls12_381: u64,
    is_on_subgroup_bls12_381: u64,
    add_bls12_381: u64,
    scalar_mul_bls12_381: u64,
    is_on_curve_twist_bls12_381: u64,
    is_on_subgroup_twist_bls12_381: u64,
    add_twist_bls12_381: u64,
    scalar_mul_twist_bls12_381: u64,
    miller_loop_bls12_381: u64,
    final_exp_bls12_381: u64,
    decompress_bls12_381: u64,
    decompress_twist_bls12_381: u64,
}

#[cfg(feature = "hints-metrics")]
impl HintTotals {
    #[inline]
    pub fn inc(&mut self, k: HintKind) {
        match k {
            HintKind::KeccakF => self.keccakf += 1,
            HintKind::Sha2 => self.sha2 += 1,
            HintKind::ECRecover => self.ecrecover += 1,
            // HintKind::ModExp => self.modexp += 1,
            HintKind::RedMod256 => self.redmod256 += 1,
            HintKind::AddMod256 => self.addmod256 += 1,
            HintKind::MulMod256 => self.mulmod256 += 1,
            HintKind::DivRem256 => self.divrem256 += 1,
            HintKind::WPow256 => self.wpow256 += 1,
            HintKind::OMul256 => self.omul256 += 1,
            HintKind::WMul256 => self.wmul256 += 1,
            HintKind::ModExp => self.modexp += 1,
            HintKind::IsOnCurveBN254 => self.is_on_curve_bn254 += 1,
            HintKind::ToAffineBN254 => self.to_affine_bn254 += 1,
            HintKind::AddBN254 => self.add_bn254 += 1,
            HintKind::MulBN254 => self.mul_bn254 += 1,
            HintKind::ToAffineTwistBN254 => self.to_affine_twist_bn254 += 1,
            HintKind::IsOnCurveTwistBN254 => self.is_on_curve_twist_bn254 += 1,
            HintKind::IsOnSubgroupTwistBN254 => self.is_on_subgroup_twist_bn254 += 1,
            HintKind::PairingBatchBN254 => self.pairing_batch_bn254 += 1,
            HintKind::MulFp12Bls12_381 => self.mul_fp12_bls12_381 += 1,
            HintKind::IsOnCurveBls12_381 => self.is_on_curve_bls12_381 += 1,
            HintKind::IsOnSubgroupBls12_381 => self.is_on_subgroup_bls12_381 += 1,
            HintKind::AddBls12_381 => self.add_bls12_381 += 1,
            HintKind::ScalarMulBls12_381 => self.scalar_mul_bls12_381 += 1,
            HintKind::IsOnCurveTwistBls12_381 => self.is_on_curve_twist_bls12_381 += 1,
            HintKind::IsOnSubgroupTwistBls12_381 => self.is_on_subgroup_twist_bls12_381 += 1,
            HintKind::AddTwistBls12_381 => self.add_twist_bls12_381 += 1,
            HintKind::ScalarMulTwistBls12_381 => self.scalar_mul_twist_bls12_381 += 1,
            HintKind::MillerLoopBls12_381 => self.miller_loop_bls12_381 += 1,
            HintKind::FinalExpBls12_381 => self.final_exp_bls12_381 += 1,
            HintKind::DecompressBls12_381 => self.decompress_bls12_381 += 1,
            HintKind::DecompressTwistBls12_381 => self.decompress_twist_bls12_381 += 1,
        }
    }

    pub fn print_summary(&self) {
        println!("Precompile hints summary:");
        if self.keccakf != 0 {
            println!("  KeccakF: {}", self.keccakf);
        }
        if self.sha2 != 0 {
            println!("  SHA2: {}", self.sha2);
        }
        if self.ecrecover != 0 {
            println!("  ECRecover: {}", self.ecrecover);
        }
        if self.redmod256 != 0 {
            println!("  RedMod256: {}", self.redmod256);
        }
        if self.addmod256 != 0 {
            println!("  AddMod256: {}", self.addmod256);
        }
        if self.mulmod256 != 0 {
            println!("  MulMod256: {}", self.mulmod256);
        }
        if self.divrem256 != 0 {
            println!("  DivRem256: {}", self.divrem256);
        }
        if self.wpow256 != 0 {
            println!("  WPow256: {}", self.wpow256);
        }
        if self.omul256 != 0 {
            println!("  OMul256: {}", self.omul256);
        }
        if self.wmul256 != 0 {
            println!("  WMul256: {}", self.wmul256);
        }
        if self.modexp != 0 {
            println!("  ModExp: {}", self.modexp);
        }
        if self.is_on_curve_bn254 != 0 {
            println!("  IsOnCurveBN254: {}", self.is_on_curve_bn254);
        }
        if self.to_affine_bn254 != 0 {
            println!("  ToAffineBN254: {}", self.to_affine_bn254);
        }
        if self.add_bn254 != 0 {
            println!("  AddBN254: {}", self.add_bn254);
        }
        if self.mul_bn254 != 0 {
            println!("  MulBN254: {}", self.mul_bn254);
        }
        if self.to_affine_twist_bn254 != 0 {
            println!("  ToAffineTwistBN254: {}", self.to_affine_twist_bn254);
        }
        if self.is_on_curve_twist_bn254 != 0 {
            println!("  IsOnCurveTwistBN254: {}", self.is_on_curve_twist_bn254);
        }
        if self.is_on_subgroup_twist_bn254 != 0 {
            println!("  IsOnSubgroupTwistBN254: {}", self.is_on_subgroup_twist_bn254);
        }
        if self.pairing_batch_bn254 != 0 {
            println!("  PairingBatchBN254: {}", self.pairing_batch_bn254);
        }
        if self.mul_fp12_bls12_381 != 0 {
            println!("  MulFp12Bls12_381: {}", self.mul_fp12_bls12_381);
        }
        if self.is_on_curve_bls12_381 != 0 {
            println!("  IsOnCurveBls12_381: {}", self.is_on_curve_bls12_381);
        }
        if self.is_on_subgroup_bls12_381 != 0 {
            println!("  IsOnSubgroupBls12_381: {}", self.is_on_subgroup_bls12_381);
        }
        if self.add_bls12_381 != 0 {
            println!("  AddBls12_381: {}", self.add_bls12_381);
        }
        if self.scalar_mul_bls12_381 != 0 {
            println!("  ScalarMulBls12_381: {}", self.scalar_mul_bls12_381);
        }
        if self.is_on_curve_twist_bls12_381 != 0 {
            println!("  IsOnCurveTwistBls12_381: {}", self.is_on_curve_twist_bls12_381);
        }
        if self.is_on_subgroup_twist_bls12_381 != 0 {
            println!("  IsOnSubgroupTwistBls12_381: {}", self.is_on_subgroup_twist_bls12_381);
        }
        if self.add_twist_bls12_381 != 0 {
            println!("  AddTwistBls12_381: {}", self.add_twist_bls12_381);
        }
        if self.scalar_mul_twist_bls12_381 != 0 {
            println!("  ScalarMulTwistBls12_381: {}", self.scalar_mul_twist_bls12_381);
        }
        if self.miller_loop_bls12_381 != 0 {
            println!("  MillerLoopBls12_381: {}", self.miller_loop_bls12_381);
        }
        if self.final_exp_bls12_381 != 0 {
            println!("  FinalExpBls12_381: {}", self.final_exp_bls12_381);
        }
        if self.decompress_bls12_381 != 0 {
            println!("  DecompressBls12_381: {}", self.decompress_bls12_381);
        }
        if self.decompress_twist_bls12_381 != 0 {
            println!("  DecompressTwistBls12_381: {}", self.decompress_twist_bls12_381);
        }
    }
}