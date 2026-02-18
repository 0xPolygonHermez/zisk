mod bls12_381;
mod bn254;
mod hint_buffer;
mod keccak256;
mod kzg;
mod macros;
mod modexp;
mod secp256k1;
mod secp256r1;
mod sha256f;

#[cfg(zisk_hints_metrics)]
mod metrics;

use crate::hints::hint_buffer::{build_hint_buffer, HintBuffer};
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use std::cell::UnsafeCell;
use std::path::PathBuf;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::{ffi::CStr, os::raw::c_char};
use std::{
    io::{self, BufWriter, Write},
    sync::Arc,
};
use tokio::sync::oneshot;
use zisk_common::io::{StreamWrite, UnixSocketStreamWriter};

#[cfg(zisk_hints_single_thread)]
use std::thread::ThreadId;

pub use bls12_381::*;
pub use bn254::*;
pub use keccak256::*;
pub use kzg::*;
pub use modexp::*;
pub use secp256k1::*;
pub use secp256r1::*;
pub use sha256f::*;

pub const CLIENT_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
pub const WAIT_FOR_CLIENT_RETRY_DELAY: Duration = Duration::from_millis(5);

static HINT_BUFFER: Lazy<Arc<HintBuffer>> = Lazy::new(|| build_hint_buffer());
static HINT_WRITER_HANDLE: Lazy<HintFileWriterHandleCell> =
    Lazy::new(HintFileWriterHandleCell::new);

pub struct HintFileWriterHandleCell {
    inner: UnsafeCell<Option<JoinHandle<io::Result<()>>>>,
}

unsafe impl Sync for HintFileWriterHandleCell {}

impl HintFileWriterHandleCell {
    pub const fn new() -> Self {
        Self { inner: UnsafeCell::new(None) }
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

fn wait_for_hints_writer() -> Result<()> {
    if let Some(handle) = HINT_WRITER_HANDLE.take() {
        HINT_BUFFER.close();
        match handle.join() {
            Ok(result) => {
                if let Err(err) = result {
                    return Err(anyhow!(
                        "Failed previous hints writer thread result, error: {}",
                        err
                    ));
                }
            }
            Err(e) => {
                return Err(anyhow!("Failed previous hints writer thread, error: {:?}", e));
            }
        }
    }

    Ok(())
}
pub fn init_hints() {
    // Initialize the main thread ID for single-threaded assert (if enabled)
    #[cfg(zisk_hints_single_thread)]
    {
        let tid = std::thread::current().id();
        *MAIN_TID.lock().unwrap() = Some(tid);
    }

    #[cfg(zisk_hints_metrics)]
    crate::hints::metrics::reset_metrics();

    HINT_BUFFER.reset();

    // Write HINT_START
    HINT_BUFFER.write_hint_start();
}

pub fn init_hints_file(hints_file_path: PathBuf, ready: Option<oneshot::Sender<()>>) -> Result<()> {
    wait_for_hints_writer()?;

    if let Some(tx) = ready {
        let _ = tx.send(());
    }

    init_hints();

    let handle = thread::spawn(move || write_hints_to_file(hints_file_path));
    HINT_WRITER_HANDLE.store(handle);

    Ok(())
}

pub fn init_hints_socket(
    socket_path: PathBuf,
    debug_file: Option<PathBuf>,
    ready: Option<oneshot::Sender<()>>,
) -> Result<()> {
    wait_for_hints_writer()?;

    // Create the Unix socket writer (server)
    let mut socket_writer = UnixSocketWriter::new(&socket_path)?;

    // Open the connection
    socket_writer.open()?;

    // Notify that socket is ready
    if let Some(tx) = ready {
        let _ = tx.send(());
    }

    // Wait for client to connect with a timeout
    if let Err(e) = socket_writer.wait_for_client(CLIENT_CONNECT_TIMEOUT) {
        return Err(anyhow!("Failed to wait for client to connect to hints socket, error: {}", e));
    }

    init_hints();

    let handle = thread::spawn(move || write_hints_to_socket(socket_writer, debug_file));
    HINT_WRITER_HANDLE.store(handle);

    Ok(())
}

pub fn close_hints() -> Result<()> {
    #[cfg(zisk_hints_single_thread)]
    {
        *MAIN_TID.lock().unwrap() = None;
    }

    // Write HINT_END
    HINT_BUFFER.write_hint_end();

    // Close the hint buffer to signal the writer thread to finish
    HINT_BUFFER.close();

    // Wait for the writer thread to finish and check for errors
    let handle = HINT_WRITER_HANDLE.take();
    if let Some(handle) = handle {
        match handle.join() {
            Ok(result) => match result {
                Ok(()) => Ok(()),
                Err(e) => return Err(anyhow!("Failed hints writer thread result, error: {}", e)),
            },
            Err(e) => Err(anyhow!("Failed hints writer thread, error: {:?}", e)),
        }
    } else {
        Ok(())
    }
}

pub fn write_hints<W: Write + ?Sized>(
    writer: &mut W,
    debug_writer: Option<&mut dyn Write>,
) -> io::Result<()> {
    // Write hints from the buffer
    HINT_BUFFER.drain_to_writer(writer, debug_writer)?;

    #[cfg(zisk_hints_metrics)]
    crate::hints::metrics::print_metrics();

    Ok(())
}

fn write_hints_to_file(path: PathBuf) -> io::Result<()> {
    debug_assert!(cfg!(target_endian = "little"));

    let file = std::fs::File::create(path)?;
    let mut file_writer = BufWriter::with_capacity(1 << 20, file);

    write_hints(&mut file_writer, None)?;

    Ok(())
}

struct UnixSocketWriter {
    inner: UnixSocketStreamWriter,
}

impl UnixSocketWriter {
    pub fn new(path: &PathBuf) -> Result<Self> {
        let writer = UnixSocketStreamWriter::new(path)?;
        Ok(Self { inner: writer })
    }

    pub fn open(&mut self) -> Result<()> {
        self.inner.open()
    }

    pub fn wait_for_client(&mut self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        while !self.inner.is_client_connected() {
            if start.elapsed() >= timeout {
                return Err(anyhow!("Timeout waiting for client to connect to socket"));
            }
            thread::sleep(WAIT_FOR_CLIENT_RETRY_DELAY);
        }

        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {
        self.inner.close()
    }
}

impl Write for UnixSocketWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}

fn write_hints_to_socket(
    mut socket_writer: UnixSocketWriter,
    debug_file: Option<PathBuf>,
) -> io::Result<()> {
    debug_assert!(cfg!(target_endian = "little"));

    if let Some(path) = debug_file {
        let file = std::fs::File::create(path)?;
        let mut debug_writer = BufWriter::with_capacity(1 << 20, file); // 1 MiB buffer
        write_hints(&mut socket_writer, Some(&mut debug_writer as &mut dyn Write))?;
    } else {
        write_hints(&mut socket_writer, None)?;
    }

    socket_writer.close().map_err(io::Error::other)?;

    Ok(())
}

#[cfg(zisk_hints_single_thread)]
static MAIN_TID: Mutex<Option<ThreadId>> = Mutex::new(None);

#[cfg(zisk_hints_single_thread)]
#[inline(always)]
pub(crate) fn check_main_thread() -> bool {
    let tid = std::thread::current().id();
    let guard = MAIN_TID.lock().unwrap();

    match *guard {
        Some(main_tid) => {
            if main_tid != tid {
                println!("Warning: trying to write hint from thread {:?} but MAIN_TID is {:?}. Ignoring...", tid, main_tid);
                return false;
            }
            true
        }
        None => {
            println!("Warning: trying to write hint from thread {:?} before MAIN_TID is initialized. Ignoring...", tid);
            false
        }
    }
}

// Logs hint message; gated by `hints_enabled()` on non-Zisk targets and always-on for Zisk
#[inline(always)]
pub fn hint_log<S: AsRef<str>>(msg: S) {
    // We check if hints are enable only for non-zisk targets, since in zisk targets hints are not used
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    if !HINT_BUFFER.is_enabled() {
        return;
    }

    println!("{}", msg.as_ref());
}

// Extern functions for C interface

#[no_mangle]
pub extern "C" fn pause_hints() -> bool {
    let already_paused = HINT_BUFFER.is_paused();
    HINT_BUFFER.pause();
    already_paused
}

#[no_mangle]
pub extern "C" fn resume_hints() {
    HINT_BUFFER.resume();
}

#[no_mangle]
pub unsafe extern "C" fn hint_log_c(msg: *const c_char) {
    if msg.is_null() {
        return;
    }

    let c_str = unsafe { CStr::from_ptr(msg) };

    match c_str.to_str() {
        Ok(s) => hint_log(s),
        Err(_) => return,
    }
}
