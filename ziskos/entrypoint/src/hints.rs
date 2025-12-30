use log::info;
use once_cell::sync::{Lazy, OnceCell};
use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::io::{self, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Condvar, Mutex,
};
use std::thread::{self, JoinHandle};
use std::thread::ThreadId;

const HINT_START: u32 = 0;
const HINT_END: u32 = 1;
// const HINT_CANCEL: u32 = 2;
// const HINT_ERROR: u32 = 3;
const HINTS_TYPE_RESULT: u32 = 4;
const HINTS_TYPE_ECRECOVER: u32 = 5;
const HINT_WRITE_BATCH: usize = 64;

// KeccakF
const KECCAKF_LEN_U64: u64 = 25;
const KECCAKF_BYTES: usize = (KECCAKF_LEN_U64 as usize) * core::mem::size_of::<u64>();
const HEADER_KECCAKF: [u8; 8] =
    (((HINTS_TYPE_RESULT as u64) << 32) | KECCAKF_LEN_U64).to_le_bytes();

// SHA2
const SHA2_LEN_U64: u64 = 4;
const SHA2_BYTES: usize = core::mem::size_of::<[u32; 8]>();
const HEADER_SHA2: [u8; 8] =
    (((HINTS_TYPE_RESULT as u64) << 32) | SHA2_LEN_U64).to_le_bytes();

// ECRecover
const ECRECOVER_BYTES: usize = core::mem::size_of::<ECRecover>();
const _: () = {
    if ECRECOVER_BYTES % 8 != 0 {
        panic!("ECRecover size must be multiple of 8");
    }
};
const ECRECOVER_LEN_U64: u64 = (ECRECOVER_BYTES as u64) / 8;
const HEADER_ECRECOVER: [u8; 8] =
    (((HINTS_TYPE_ECRECOVER as u64) << 32) | ECRECOVER_LEN_U64).to_le_bytes();

static HINT_QUEUE: Lazy<HintQueue> = Lazy::new(HintQueue::new);
static HINT_FILE_WRITER_HANDLE: Lazy<HintFileWriterHandleCell> = Lazy::new(HintFileWriterHandleCell::new);
static MAIN_TID: OnceCell<ThreadId> = OnceCell::new();

struct HintFileWriterHandleCell {
    inner: UnsafeCell<Option<JoinHandle<io::Result<()>>>>,
}

unsafe impl Sync for HintFileWriterHandleCell {}

impl HintFileWriterHandleCell {
    const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(None),
        }
    }

    fn take(&self) -> Option<JoinHandle<io::Result<()>>> {
        unsafe { (*self.inner.get()).take() }
    }

    fn store(&self, handle: JoinHandle<io::Result<()>>) {
        // Safety: caller guarantees single-threaded access when mutating the handle.
        unsafe {
            *self.inner.get() = Some(handle);
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum HintKind {
    KeccakF,
    Sha2,
    ECRecover,
    ModExp,
}

#[derive(Clone, Debug)]
enum Hint {
    KeccakF([u64; 25]),
    SHA2([u32; 8]),
    ECRecover(ECRecover),
    ModExp(Vec<u8>),
}

impl Hint {
    #[inline]
    #[allow(unused)]
    fn kind(&self) -> HintKind {
        match self {
            Hint::KeccakF(_) => HintKind::KeccakF,
            Hint::SHA2(_) => HintKind::Sha2,
            Hint::ECRecover(_) => HintKind::ECRecover,
            Hint::ModExp(_) => HintKind::ModExp,
        }
    }

    #[allow(unused)]
    fn read_from(&self, file: &mut std::fs::File, disable_prefix: bool) -> Result<(), String> {
        let kind = self.kind();

        let (expected_header, expected_payload): ([u8; 8], &[u8]) = match self {
            Hint::KeccakF(state) => {
                let bytes = unsafe {
                    core::slice::from_raw_parts(state.as_ptr() as *const u8, KECCAKF_BYTES)
                };
                (HEADER_KECCAKF, bytes)
            }
            Hint::SHA2(state) => {
                let bytes = unsafe {
                    core::slice::from_raw_parts(state.as_ptr() as *const u8, SHA2_BYTES)
                };
                (HEADER_SHA2, bytes)
            }
            Hint::ECRecover(rec) => {
                let bytes = unsafe {
                    core::slice::from_raw_parts(
                        (rec as *const ECRecover).cast::<u8>(),
                        ECRECOVER_BYTES,
                    )
                };
                (HEADER_ECRECOVER, bytes)
            }
            Hint::ModExp(buf) => {
                let len = buf.len() as u64;
                let header = (((HINTS_TYPE_RESULT as u64) << 32) | len).to_le_bytes();
                (header, buf.as_slice())
            }
        };

        if !disable_prefix {
            let mut header = [0u8; 8];
            if let Err(e) = file.read_exact(&mut header) {
                return Err(format!("Failed to read {:?} header, error: {}", kind, e));
            }

            if header != expected_header {
                return Err(format!("Unexpected {:?} header: expected {:?}, got {:?}", kind, expected_header, header));
            }
        }

        let mut payload = vec![0u8; expected_payload.len()];
        if let Err(e) = file.read_exact(&mut payload) {
            return Err(format!("Failed to read {:?} payload, error: {}", kind, e));
        }

        if payload.as_slice() != expected_payload {
            return Err(format!("{:?} value mismatch", kind));
        }

        Ok(())
    }

    #[inline]
    fn write_to<W: std::io::Write>(&self, w: &mut W, disable_prefix: bool) -> io::Result<()> {
        debug_assert!(cfg!(target_endian = "little"));

        match self {
            Hint::KeccakF(state) => {
                if !disable_prefix {
                    w.write_all(&HEADER_KECCAKF)?;
                }
                let bytes = unsafe {
                    core::slice::from_raw_parts(state.as_ptr() as *const u8, KECCAKF_BYTES)
                };
                w.write_all(bytes)?;
            }

            Hint::SHA2(state) => {
                if !disable_prefix {
                    w.write_all(&HEADER_SHA2)?;
                }
                let bytes =
                    unsafe { core::slice::from_raw_parts(state.as_ptr() as *const u8, SHA2_BYTES) };
                w.write_all(bytes)?;
            }

            Hint::ECRecover(rec) => {
                if !disable_prefix {
                    w.write_all(&HEADER_ECRECOVER)?;
                }
                let bytes = unsafe {
                    core::slice::from_raw_parts(
                        (rec as *const ECRecover).cast::<u8>(),
                        ECRECOVER_BYTES,
                    )
                };
                w.write_all(bytes)?;
            }

            Hint::ModExp(buf) => {
                let len = buf.len() as u64;
                let header = (((HINTS_TYPE_RESULT as u64) << 32) | len).to_le_bytes();
                if !disable_prefix {
                    w.write_all(&header)?;
                }
                w.write_all(buf)?;
            }
        }

        Ok(())
    }
}

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ECRecover {
    pub pk: [u8; 33],
    pub z: [u8; 32],
    pub sig: [u8; 64],
}

impl Default for ECRecover {
    fn default() -> Self {
        Self {
            pk: [0u8; 33],
            z: [0u8; 32],
            sig: [0u8; 64],
        }
    }
}

#[cfg(feature = "hints-metrics")]
#[derive(Default, Debug)]
struct HintTotals {
    keccakf: u64,
    sha2: u64,
    ecrecover: u64,
    modexp: u64,
}

#[cfg(feature = "hints-metrics")]
impl HintTotals {
    #[inline]
    fn inc(&mut self, k: HintKind) {
        match k {
            HintKind::KeccakF => self.keccakf += 1,
            HintKind::Sha2 => self.sha2 += 1,
            HintKind::ECRecover => self.ecrecover += 1,
            HintKind::ModExp => self.modexp += 1,
        }
    }

    fn print_summary(&self) {
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
    }
}

#[derive(Debug)]
struct HintQueue {
    states: Mutex<VecDeque<Hint>>,
    condvar: Condvar,
    closed: AtomicBool,
}

impl HintQueue {
    const fn new() -> Self {
        Self {
            states: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            closed: AtomicBool::new(true),
        }
    }

    fn reset(&self) {
        let mut states = self.states.lock().unwrap();
        states.clear();
        self.closed.store(false, Ordering::SeqCst);
    }

    #[inline(always)]
    fn push(&self, hint: Hint) {
        let mut states = self.states.lock().unwrap();
        states.push_back(hint);
        self.condvar.notify_one();
    }

    fn pop_batch(&self, out: &mut Vec<Hint>, max_batch: usize) -> bool {
        let mut states = self.states.lock().unwrap();
        loop {
            if !states.is_empty() {
                let take = max_batch.min(states.len());
                for _ in 0..take {
                    if let Some(hint) = states.pop_front() {
                        out.push(hint);
                    }
                }
                return true;
            }

            if self.closed.load(Ordering::SeqCst) {
                return false;
            }

            states = self.condvar.wait(states).unwrap();
        }
    }

    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.condvar.notify_all();
    }
}

pub fn init_precompile_hints(hints_file_path: PathBuf) -> io::Result<()> {
    // Record the main thread id to validate single-threaded calls later
    let _ = MAIN_TID.set(std::thread::current().id());

    if let Some(handle) = HINT_FILE_WRITER_HANDLE.take() {
        HINT_QUEUE.close();
        match handle.join() {
            Ok(result) => {
                if let Err(err) = result {
                    return Err(err);
                }
            }
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed precompile hints writer thread, error: {:?}", e),
                ))
            }
        }
    }

    HINT_QUEUE.reset();

    let handle = thread::spawn(move || write_precompile_hints(hints_file_path));
    HINT_FILE_WRITER_HANDLE.store(handle);

    Ok(())
}

#[inline(always)]
pub fn check_main_thread() {
    // Panic on calls from a different thread
    let tid = std::thread::current().id();
    match MAIN_TID.get() {
        Some(main) => {
            if *main != tid {
                panic!(
                    "Precompile hint function called from non-main thread, main={:?}, current={:?}",
                    main, tid
                );
            }
        }
        None => {
            // If not initialized yet, record the first caller thread as main
            let _ = MAIN_TID.set(tid);
        }
    }
}

#[inline(always)]
pub fn hint_keccakf(state: &[u64; 25]) {
    check_main_thread();

    let hint = Hint::KeccakF(*state);
    HINT_QUEUE.push(hint);
}

#[inline(always)]
pub fn hint_sha2(state: &[u32; 8]) {
    check_main_thread();

    let hint = Hint::SHA2(*state);
    HINT_QUEUE.push(hint);
}

#[inline(always)]
pub fn hint_ecrecover(pk: &[u8; 33], z: &[u8; 32], sig: &[u8; 64]) {
    check_main_thread();

    let hint = Hint::ECRecover(ECRecover {
        pk: *pk,
        z: *z,
        sig: *sig,
    });
    HINT_QUEUE.push(hint);
}

#[inline(always)]
pub fn hint_modexp(data: Vec<u8>) {
    check_main_thread();

    let hint = Hint::ModExp(data);
    HINT_QUEUE.push(hint);
}

pub fn close_precompile_hints() -> io::Result<()> {
    HINT_QUEUE.close();

    let handle = HINT_FILE_WRITER_HANDLE.take();
    if let Some(handle) = handle {
        match handle.join() {
            Ok(result) => {
                match result {
                    Ok(()) => Ok(()),
                    Err(e) => return Err(e),
                }
            }
            Err(e) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed precompile hints writer thread, error: {:?}", e),
            )),
        }
    } else {
        Ok(())
    }
}

fn write_precompile_hints(path: PathBuf) -> io::Result<()> {
    debug_assert!(cfg!(target_endian = "little"));

    let file = std::fs::File::create(path)?;
    let mut writer = BufWriter::with_capacity(1 << 20, file);
    let disable_prefix = std::env::var("HINTS_DISABLE_PREFIX").unwrap_or_default() == "1";

    #[cfg(feature = "hints-reference")]
    let mut ref_file: Option<std::fs::File> = None;
    #[cfg(feature = "hints-reference")]
    let mut ref_idx: usize = 0;
    #[cfg(feature = "hints-reference")]
    if let Ok(path) = std::env::var("HINTS_REF_FILE") {
        info!("Comparing precompile hints against reference file {}", path);
        let mut f = std::fs::File::open(path)?;
        let mut start = [0u8; 8];
        let _ = f.read_exact(&mut start);
        ref_file = Some(f);
    }

    // Write HINT_START
    if !disable_prefix {
        let start_header: u64 = ((HINT_START as u64) << 32) | 0u64;
        let start_bytes = start_header.to_le_bytes();
        writer.write_all(&start_bytes)?;
    }

    #[cfg(feature = "hints-metrics")]
    let mut totals = HintTotals::default();

    let mut batch = Vec::with_capacity(HINT_WRITE_BATCH);
    loop {
        batch.clear();
        if !HINT_QUEUE.pop_batch(&mut batch, HINT_WRITE_BATCH) {
            break;
        }

        for hint in batch.drain(..) {
            #[cfg(feature = "hints-reference")]
            if let Some(file) = ref_file.as_mut() {
                if let Err(err) = hint.read_from(file, disable_prefix) {
                    panic!("Reference comparison failed at hint #{}: {}", ref_idx, err);
                }
                ref_idx += 1;
            }

            #[cfg(feature = "hints-metrics")]
            totals.inc(hint.kind());

            hint.write_to(&mut writer, disable_prefix)?;
        }
    }

    // Write HINT_END
    if !disable_prefix {
        let end_header: u64 = ((HINT_END as u64) << 32) | 0u64;
        let end_bytes = end_header.to_le_bytes();
        writer.write_all(&end_bytes)?;
    }

    writer.flush()?;

    #[cfg(feature = "hints-metrics")]
    totals.print_summary();

    Ok(())
}
