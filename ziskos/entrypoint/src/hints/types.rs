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
    modexp: u64,
    redmod256: u64,
    addmod256: u64,
    mulmod256: u64,
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
        }
    }

    pub fn print_summary(&self) {
        use log::info;

        info!("Precompile hints summary:");
        if self.keccakf != 0 {
            info!("  KeccakF: {}", self.keccakf);
        }
        if self.sha2 != 0 {
            info!("  SHA2: {}", self.sha2);
        }
        if self.ecrecover != 0 {
            info!("  ECRecover: {}", self.ecrecover);
        }
        if self.modexp != 0 {
            info!("  ModExp: {}", self.modexp);
        }
        if self.redmod256 != 0 {
            info!("  RedMod256: {}", self.redmod256);
        }
        if self.addmod256 != 0 {
            info!("  AddMod256: {}", self.addmod256);
        }
        if self.mulmod256 != 0 {
            info!("  MulMod256: {}", self.mulmod256);
        }
    }
}