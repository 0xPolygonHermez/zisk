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
use anyhow::Result;
use once_cell::sync::Lazy;
use std::cell::UnsafeCell;
use std::path::PathBuf;
use std::thread::{self, JoinHandle};
use std::{ffi::CStr, os::raw::c_char};
use std::{
    io::{self, BufWriter, Write},
    sync::Arc,
};
use zisk_common::io::{StreamWrite, UnixSocketStreamWriter};

#[cfg(zisk_hints_single_thread)]
use std::thread::ThreadId;

#[cfg(zisk_hints_single_thread)]
use once_cell::sync::OnceCell;

pub use bls12_381::*;
pub use bn254::*;
pub use keccak256::*;
pub use kzg::*;
pub use modexp::*;
pub use secp256k1::*;
pub use secp256r1::*;
pub use sha256f::*;

pub const HINT_START: u32 = 0;
pub const HINT_END: u32 = 1;

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

pub fn init_hints() -> io::Result<()> {
    // Initialize the main thread ID for single-threaded assert (if enabled)
    #[cfg(zisk_hints_single_thread)]
    let _ = MAIN_TID.set(None); // Placeholder value to mark uninitialized

    if let Some(handle) = HINT_WRITER_HANDLE.take() {
        HINT_BUFFER.close();
        match handle.join() {
            Ok(result) => {
                if let Err(err) = result {
                    return Err(err);
                }
            }
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed hints writer thread, error: {:?}", e),
                ))
            }
        }
    }

    HINT_BUFFER.reset();

    Ok(())
}

pub fn init_hints_file(hints_file_path: PathBuf) -> io::Result<()> {
    init_hints()?;

    let handle = thread::spawn(move || write_hints_to_file(hints_file_path));
    HINT_WRITER_HANDLE.store(handle);

    Ok(())
}

pub fn init_hints_socket(socket_path: PathBuf) -> io::Result<()> {
    init_hints()?;

    // Create the Unix socket writer (server)
    let mut socket_writer = UnixSocketWriter::new(&socket_path).map_err(io::Error::other)?;

    // Open the connection (waits for client to connect)
    // TODO: Implement open timeout
    socket_writer.open().map_err(io::Error::other)?;
    println!("Client connected to hints socket! Starting hint data transfer...");

    let handle = thread::spawn(move || write_hints_to_socket(socket_writer));
    HINT_WRITER_HANDLE.store(handle);

    Ok(())
}
pub fn close_hints() -> io::Result<()> {
    HINT_BUFFER.close();

    let handle = HINT_WRITER_HANDLE.take();
    if let Some(handle) = handle {
        match handle.join() {
            Ok(result) => match result {
                Ok(()) => Ok(()),
                Err(e) => return Err(e),
            },
            Err(e) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed hints writer thread, error: {:?}", e),
            )),
        }
    } else {
        Ok(())
    }
}

pub fn write_hints<W: Write>(writer: &mut W) -> io::Result<()> {
    let disable_prefix = std::env::var("HINTS_DISABLE_PREFIX").unwrap_or_default() == "1";

    // Write HINT_START
    if !disable_prefix {
        let start_header: u64 = ((HINT_START as u64) << 32) | 0u64;
        let start_bytes = start_header.to_le_bytes();
        writer.write_all(&start_bytes)?;
    }

    // Write hints from the buffer
    HINT_BUFFER.drain_to_writer(writer)?;
    // Write HINT_END
    if !disable_prefix {
        let end_header: u64 = ((HINT_END as u64) << 32) | 0u64;
        let end_bytes = end_header.to_le_bytes();
        writer.write_all(&end_bytes)?;
    }

    writer.flush()?;

    #[cfg(zisk_hints_metrics)]
    crate::hints::metrics::print_metrics();

    Ok(())
}

fn write_hints_to_file(path: PathBuf) -> io::Result<()> {
    debug_assert!(cfg!(target_endian = "little"));

    let file = std::fs::File::create(path)?;
    let mut file_writer = BufWriter::with_capacity(1 << 20, file);

    write_hints(&mut file_writer)?;

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
}

impl Write for UnixSocketWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}

fn write_hints_to_socket(mut socket_writer: UnixSocketWriter) -> io::Result<()> {
    debug_assert!(cfg!(target_endian = "little"));

    write_hints(&mut socket_writer)?;

    Ok(())
}

#[cfg(zisk_hints_single_thread)]
static MAIN_TID: OnceCell<Option<ThreadId>> = OnceCell::new();

#[cfg(zisk_hints_single_thread)]
#[inline(always)]
pub(crate) fn check_main_thread() {
    // Panic on calls from a different thread
    let tid = std::thread::current().id();
    match MAIN_TID.get() {
        Some(main) => {
            match main {
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
                    let _ = MAIN_TID.set(Some(tid));
                }
            }
        }
        None => {
            // If not initialized yet, record the first caller thread as main
            let _ = MAIN_TID.set(Some(tid));
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
