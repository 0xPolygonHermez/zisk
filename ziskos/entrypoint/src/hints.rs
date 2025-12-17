use once_cell::sync::{Lazy, OnceCell};
use std::cell::UnsafeCell;
use std::collections::{HashMap, VecDeque};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
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

static HINT_QUEUE: Lazy<HintQueue> = Lazy::new(HintQueue::new);
static HINT_FILE_WRITER_HANDLE: Lazy<HintFileWriterHandleCell> = Lazy::new(HintFileWriterHandleCell::new);
static KECCAKF_THREAD_COUNTS: Lazy<Mutex<HashMap<ThreadId, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static ECRECOVER_THREAD_COUNTS: Lazy<Mutex<HashMap<ThreadId, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static MAIN_TID: OnceCell<ThreadId> = OnceCell::new();
static REF_FILE: Lazy<Mutex<Option<std::fs::File>>> = Lazy::new(|| Mutex::new(None));
static KECCAKF_SEQ: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));
static ECRECOVER_SEQ: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));

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

enum Hint {
    KeccakF([u64; 25]),
    ECRecover(ECRecover),
}

pub struct ECRecover {
    pub pk: [u8; 33],
    pub z: [u8; 32],
    pub sig: [u8; 64],
}

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
    fn push_keccakf(&self, state: [u64; 25]) {
        if self.closed.load(Ordering::SeqCst) {
            return;
        }

        let mut states = self.states.lock().unwrap();
        states.push_back(Hint::KeccakF(state));
        self.condvar.notify_one();
    }

    #[inline(always)]
    fn push_ecrecover(&self, rec: ECRecover) {
        if self.closed.load(Ordering::SeqCst) {
            return;
        }

        let mut states = self.states.lock().unwrap();
        states.push_back(Hint::ECRecover(rec));
        self.condvar.notify_one();
    }

    fn pop(&self) -> Option<Hint> {
        let mut states = self.states.lock().unwrap();
        loop {
            if let Some(state) = states.pop_front() {
                return Some(state);
            }

            if self.closed.load(Ordering::SeqCst) {
                return None;
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
    // Open test hints file and consume initial HINT_START header if present
    {
        // Check if env variable HINTS_COMPARE_REF_FILE is set
        let compare_ref = std::env::var("HINTS_COMPARE_REF_FILE").is_ok();
        if compare_ref {
            println!("Comparing precompile hints against reference file {}", std::env::var("HINTS_COMPARE_REF_FILE").unwrap());
            let mut guard = REF_FILE.lock().unwrap();
            let compare_file = std::env::var("HINTS_COMPARE_REF_FILE").unwrap();
            *guard = Some(std::fs::File::open(compare_file)?);
            if let Some(f) = guard.as_mut() {
                let mut start = [0u8; 8];
                // Ignore content; just advance the cursor
                let _ = f.read_exact(&mut start);
            }
        }
    }
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

pub fn is_precompile_hints_enabled() -> bool {
    !HINT_QUEUE.closed.load(Ordering::SeqCst)
}

#[inline(always)]
pub fn hint_keccakf(state: &[u64; 25]) {
    if HINT_QUEUE.closed.load(Ordering::SeqCst) {
        return;
    }

    // Panic on calls from a different thread to quickly capture a stacktrace.
    let tid = std::thread::current().id();
    match MAIN_TID.get() {
        Some(main) => {
            if *main != tid {
                panic!(
                    "hint_keccakf llamado desde hilo no principal: main={:?} now={:?}",
                    main, tid
                );
            }
        }
        None => {
            // If not initialized yet, record the first caller thread as main
            let _ = MAIN_TID.set(tid);
        }
    }
    {
        let tid = std::thread::current().id();
        let mut map = KECCAKF_THREAD_COUNTS.lock().unwrap();
        *map.entry(tid).or_insert(0) += 1;
        // if *map.get(&tid).unwrap() == 53317 {
        //     panic!("hint_keccakf number 53317, thread id: {:?}", tid);
        // }
    }
    // Validate against reference file if available: read header (8 bytes) then 200 bytes payload
    {
        let compare_ref = std::env::var("HINTS_COMPARE_REF_FILE").is_ok();
        if compare_ref {
            let mut guard = REF_FILE.lock().unwrap();
            if let Some(f) = guard.as_mut() {
                // Current sequence number (starting at 0)
                let seq = KECCAKF_SEQ.fetch_add(1, Ordering::SeqCst);
                let mut header = [0u8; 8];
                if let Err(e) = f.read_exact(&mut header) {
                    println!("fallo leyendo cabecera keccakf de referencia en #{}: {:?}", seq, e);
                }
                // Assert header matches expected KeccakF header (type/result + length=25)
                let expected_header = (((HINTS_TYPE_RESULT as u64) << 32) | (25u64)).to_le_bytes();
                if header != expected_header {
                    println!("cabecera keccakf inesperada en #{}", seq);
                }
                let mut ref_payload = [0u8; 25 * std::mem::size_of::<u64>()];
                if let Err(e) = f.read_exact(&mut ref_payload) {
                    println!("fallo leyendo payload keccakf de referencia en #{}: {:?}", seq, e);
                }
                let state_bytes = unsafe {
                    std::slice::from_raw_parts(
                        state.as_ptr() as *const u8,
                        25 * std::mem::size_of::<u64>(),
                    )
                };
                if ref_payload.as_slice() != state_bytes {
                    panic!("discrepancia en keccakf #{}", seq);
                }
            }
        }
    }
    // Arrays up to length 32 implement `Copy`, so this is a fast memcpy without extra loops.
    HINT_QUEUE.push_keccakf(*state);
}

#[inline(always)]
pub fn hint_ecrecover(pk: &[u8; 33], z: &[u8; 32], sig: &[u8; 64]) {
    {
        let tid = std::thread::current().id();
        let mut map = ECRECOVER_THREAD_COUNTS.lock().unwrap();
        *map.entry(tid).or_insert(0) += 1;
        // if *map.get(&tid).unwrap() == 53317 {
        //     panic!("hint_keccakf number 53317, thread id: {:?}", tid);
        // }
    }

    let owned = ECRecover {
        pk: *pk,
        z: *z,
        sig: *sig,
    };
    HINT_QUEUE.push_ecrecover(owned);
}

pub fn close_precompile_hints() -> io::Result<()> {
    HINT_QUEUE.close();

    let handle = HINT_FILE_WRITER_HANDLE.take();
    if let Some(handle) = handle {
        match handle.join() {
            Ok(result) => {
                match result {
                    Ok(()) => {
                        // Print per-thread counts summary
                        let mut map = KECCAKF_THREAD_COUNTS.lock().unwrap();
                        if !map.is_empty() {
                            println!("hint_keccakf calls per thread:");
                            for (tid, count) in map.iter() {
                                println!("  {:?}: {}", tid, count);
                            }
                        }
                        map.clear();

                        let mut map = ECRECOVER_THREAD_COUNTS.lock().unwrap();
                        if !map.is_empty() {
                            println!("hint_ecrecover calls per thread:");
                            for (tid, count) in map.iter() {
                                println!("  {:?}: {}", tid, count);
                            }
                        }
                        map.clear();

                        KECCAKF_SEQ.store(0, Ordering::SeqCst);
                        ECRECOVER_SEQ.store(0, Ordering::SeqCst);

                        let mut rf = REF_FILE.lock().unwrap();
                        *rf = None;

                        Ok(())
                    },
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

    let mut file = std::fs::File::create(path)?;
    let mut total_keccakf = 0usize;
    let mut total_ecrecover = 0usize;

    const KECCAKF_LENGTH: usize = 25; // kecakkf length in u64s
    let header_keccakf = (((HINTS_TYPE_RESULT as u64) << 32) | (KECCAKF_LENGTH as u64)).to_le_bytes();

    const ECRECOVER_LENGTH: usize = 129; // ECRecover length in bytes: pk (33) + z (32) + sig (64)
    let header_ecrecover = (((HINTS_TYPE_ECRECOVER as u64) << 32) | (ECRECOVER_LENGTH as u64)).to_le_bytes();
    let mut buffer_ecrecover = [0u8; ECRECOVER_LENGTH];

    let disable_prefix = std::env::var("HINTS_DISABLE_PREFIX").unwrap_or_default() == "1";

    // Write HINT_START
    if !disable_prefix {
        let start_header: u64 = ((HINT_START as u64) << 32) | 0u64;
        let start_bytes = start_header.to_le_bytes();
        file.write_all(&start_bytes)?;
    }

    while let Some(item) = HINT_QUEUE.pop() {
        match item {
            Hint::KeccakF(state) => {
                // Safety: state is [u64; 25] and the target is little-endian
                let state_bytes = unsafe {
                    std::slice::from_raw_parts(
                        state.as_ptr() as *const u8,
                        KECCAKF_LENGTH * std::mem::size_of::<u64>(),
                    )
                };
                if !disable_prefix {
                    file.write_all(&header_keccakf)?;
                }
                file.write_all(state_bytes)?;
                total_keccakf += 1;
            }
            Hint::ECRecover(rec) => {
                let ecrecover_enabled = std::env::var("HINTS_ECRECOVER").unwrap_or_default() == "1";

                if ecrecover_enabled {
                    unsafe {
                        // Copy pk, z, sig into u8 buffer_ecrecover array
                        std::ptr::copy_nonoverlapping(rec.pk.as_ptr(), buffer_ecrecover.as_mut_ptr(), 33);
                        std::ptr::copy_nonoverlapping(rec.z.as_ptr(), buffer_ecrecover.as_mut_ptr().add(33), 32);
                        std::ptr::copy_nonoverlapping(rec.sig.as_ptr(), buffer_ecrecover.as_mut_ptr().add(65), 64);
                    }

                    // Safety: buf is [u64; 20] and the target is little-endian
                    let bytes = unsafe {
                        std::slice::from_raw_parts(
                            buffer_ecrecover.as_ptr(),
                            ECRECOVER_LENGTH * std::mem::size_of::<u64>(),
                        )
                    };

                    if !disable_prefix {
                        file.write_all(&header_ecrecover)?;
                    }
                    file.write_all(bytes)?;
                    total_ecrecover += 1;
                }
            }
        }
    }

    // Write HINT_END
    if !disable_prefix {
        let end_header: u64 = ((HINT_END as u64) << 32) | 0u64;
        let end_bytes = end_header.to_le_bytes();
        file.write_all(&end_bytes)?;
    }

    file.flush()?;

    println!("Total keccakf hints written: {}", total_keccakf);
    println!("Total ecrecover hints written: {}", total_ecrecover);

    Ok(())
}