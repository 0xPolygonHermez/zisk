use std::{cell::UnsafeCell, io, thread::JoinHandle};

pub const HINT_START: u32 = 0;
pub const HINT_END: u32 = 1;
// const HINT_CANCEL: u32 = 2;
// const HINT_ERROR: u32 = 3;
// pub const HINTS_TYPE_RESULT: u32 = 4;
// pub const HINTS_TYPE_REDMOD256: u32 = 6;
// pub const HINTS_TYPE_ADDMOD256: u32 = 7;
// pub const HINTS_TYPE_MULMOD256: u32 = 8;
// pub const HINTS_TYPE_DIVREM256: u32 = 9;
// pub const HINTS_TYPE_WPOW256: u32 = 10;
// pub const HINTS_TYPE_OMUL256: u32 = 11;
// pub const HINTS_TYPE_WMUL256: u32 = 12;
// pub const HINTS_TYPE_MODEXP: u32 = 13;
// pub const HINTS_TYPE_IS_ON_CURVE_BN254: u32 = 14;
// pub const HINTS_TYPE_TO_AFFINE_BN254: u32 = 15;
// pub const HINTS_TYPE_ADD_BN254: u32 = 16;
// pub const HINTS_TYPE_MUL_BN254: u32 = 17;
// pub const HINTS_TYPE_TO_AFFINE_TWIST_BN254: u32 = 18;
// pub const HINTS_TYPE_IS_ON_CURVE_TWIST_BN254: u32 = 19;
// pub const HINTS_TYPE_IS_ON_SUBGROUP_TWIST_BN254: u32 = 20;
// pub const HINTS_TYPE_PAIRING_BATCH_BN254: u32 = 21;

// BLS12-381 hint codes
// pub const HINT_MUL_FP12_BLS12_381: u32 = 0x16;
// pub const HINT_DECOMPRESS_BLS12_381: u32 = 0x17;
// pub const HINT_IS_ON_CURVE_BLS12_381: u32 = 0x18;
// pub const HINT_IS_ON_SUBGROUP_BLS12_381: u32 = 0x19;
// pub const HINT_ADD_BLS12_381: u32 = 0x1A;
// pub const HINT_SCALAR_MUL_BLS12_381: u32 = 0x1B;
// pub const HINT_DECOMPRESS_TWIST_BLS12_381: u32 = 0x1C;
// pub const HINT_IS_ON_CURVE_TWIST_BLS12_381: u32 = 0x1D;
// pub const HINT_IS_ON_SUBGROUP_TWIST_BLS12_381: u32 = 0x1E;
// pub const HINT_ADD_TWIST_BLS12_381: u32 = 0x1F;
// pub const HINT_SCALAR_MUL_TWIST_BLS12_381: u32 = 0x20;
// pub const HINT_MILLER_LOOP_BLS12_381: u32 = 0x21;
// pub const HINT_FINAL_EXP_BLS12_381: u32 = 0x22;

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

#[derive(Clone, Debug)]
pub struct HintRegisterInfo {
    pub hint_name: String,
    pub count: u64,
}