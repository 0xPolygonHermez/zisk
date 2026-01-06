#![warn(unused_imports)]

mod bn254;
mod bigint256;
mod hint;
mod keccakf;
mod modexp;
mod secp256k1;
mod sha256f;
mod types;
mod utils;

use crate::hints::{
    hint::HintQueue,
    types::{HINT_START, HINT_END, HINT_WRITE_BATCH, HintFileWriterHandleCell},
};

#[cfg(feature = "hints-metrics")]
use types::HintTotals;

use once_cell::sync::{Lazy, OnceCell};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::thread::{self, ThreadId};

#[cfg(feature = "hints-reference")]
use std::io::Read;

static HINT_QUEUE: Lazy<HintQueue> = Lazy::new(HintQueue::new);
static HINT_FILE_WRITER_HANDLE: Lazy<HintFileWriterHandleCell> = Lazy::new(HintFileWriterHandleCell::new);
static MAIN_TID: OnceCell<ThreadId> = OnceCell::new();

pub use keccakf::hint_keccakf;
pub use sha256f::hint_sha2;
pub use secp256k1::hint_ecrecover;
pub use bigint256::{
    hint_redmod256,
    hint_addmod256,
    hint_mulmod256,
    hint_divrem256,
    hint_wpow256,
    hint_omul256,
    hint_wmul256
};
pub use modexp::hint_modexp;
pub use bn254::{
    hint_is_on_curve_bn254,
    hint_to_affine_bn254,
    hint_add_bn254,
    hint_mul_bn254,
    hint_to_affine_twist_bn254,
    hint_is_on_curve_twist_bn254,
    hint_is_on_subgroup_twist_bn254,
    hint_pairing_batch_bn254,
};

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

pub fn is_hints_enabled() -> bool {
    HINT_QUEUE.is_open()
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

#[inline(always)]
fn check_main_thread() {
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
        println!("Comparing precompile hints against reference file {}", path);
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
