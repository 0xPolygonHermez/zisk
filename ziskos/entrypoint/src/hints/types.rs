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
    }
}