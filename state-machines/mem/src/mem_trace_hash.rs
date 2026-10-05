//! Content hash of a `Mem` instance, written per segment when `ZISK_MEM_TRACE_HASH_DIR` is set.
//! Two runs of the same block (CPU fill, device fill) can then be compared bit for bit: the hash
//! covers every word of the trace rows and the scalars the air values are built from.

use rayon::prelude::*;
use std::io::Write;

use crate::mem_sm::{MemFillOutput, MemPreviousSegment};

fn mix(mut h: u64, v: u64) -> u64 {
    h ^= v;
    h = h.wrapping_mul(0x0000_0100_0000_01B3); // FNV-1a prime, on words
    h ^= h >> 29;
    h
}

/// FNV-style hash over words, in 1M-word chunks hashed in parallel and folded in order.
fn hash_words(words: &[u64]) -> u64 {
    let chunk = 1 << 20;
    let partials: Vec<u64> = words
        .par_chunks(chunk)
        .map(|c| c.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &w| mix(h, w)))
        .collect();
    partials.iter().fold(0x9E37_79B9_7F4A_7C15u64, |h, &p| mix(h, p))
}

/// Writes `<dir>/<name>_<segment>.hash` when the env var is set. `rows` is the trace as raw words.
pub(crate) fn dump(
    name: &str,
    segment: usize,
    is_last_segment: bool,
    rows: &[u64],
    previous_segment: &MemPreviousSegment,
    out: &MemFillOutput,
) {
    let Ok(dir) = std::env::var("ZISK_MEM_TRACE_HASH_DIR") else {
        return;
    };
    let h_rows = hash_words(rows);
    let scalars = [
        segment as u64,
        is_last_segment as u64,
        previous_segment.addr as u64,
        previous_segment.step,
        previous_segment.value,
        out.last_addr as u64,
        out.last_step,
        out.last_value[0] as u64,
        out.last_value[1] as u64,
        out.distance_base[0] as u64,
        out.distance_base[1] as u64,
        out.distance_end[0] as u64,
        out.distance_end[1] as u64,
        out.padding_size as u64,
    ];
    let h_scalars = scalars.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &w| mix(h, w));
    let path = format!("{dir}/{name}_{segment}.hash");
    match std::fs::File::create(&path) {
        Ok(mut f) => {
            let _ = writeln!(f, "rows {h_rows:016x} scalars {h_scalars:016x} words {}", rows.len());
        }
        Err(e) => tracing::warn!("cannot write {path}: {e}"),
    }
}

/// The rows of a trace as raw words. Row types are `#[repr(C)]` and made of `u64` words (packed)
/// or field elements of 8 bytes (unpacked), so the byte image is the content.
/// Like [`hash_words`], leaving out the bits of each row's last word beyond `used_bits`.
fn hash_words_masked(words: &[u64], words_per_row: usize, used_bits: usize) -> u64 {
    let tail_bits = used_bits - 64 * (words_per_row - 1);
    let tail_mask = if tail_bits >= 64 { u64::MAX } else { (1u64 << tail_bits) - 1 };
    let chunk = (1 << 20) / words_per_row * words_per_row;
    let partials: Vec<u64> = words
        .par_chunks(chunk)
        .map(|c| {
            c.chunks(words_per_row).fold(0xcbf2_9ce4_8422_2325u64, |h, row| {
                let (tail, head) = row.split_last().unwrap();
                mix(head.iter().fold(h, |h, &w| mix(h, w)), tail & tail_mask)
            })
        })
        .collect();
    partials.iter().fold(0x9E37_79B9_7F4A_7C15u64, |h, &p| mix(h, p))
}

/// Like [`dump`], for a memory whose air values are the given scalars. The bits of each row's
/// last word beyond `used_bits` are left out: a fill that does not zero its buffer leaves them
/// unset, and the prover never reads them.
pub(crate) fn dump_scalars(
    name: &str,
    segment: usize,
    rows: &[u64],
    scalars: &[u64],
    words_per_row: usize,
    used_bits: usize,
) {
    let Ok(dir) = std::env::var("ZISK_MEM_TRACE_HASH_DIR") else {
        return;
    };
    let h_rows = hash_words_masked(rows, words_per_row, used_bits);
    let h_scalars = scalars.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &w| mix(h, w));
    let path = format!("{dir}/{name}_{segment}.hash");
    match std::fs::File::create(&path) {
        Ok(mut f) => {
            let _ = writeln!(f, "rows {h_rows:016x} scalars {h_scalars:016x} words {}", rows.len());
        }
        Err(e) => tracing::warn!("mem trace hash: cannot write {path}: {e}"),
    }
}

/// The rows' memory as words, for a packed row type the device writes directly.
pub(crate) fn rows_as_words_mut<R>(rows: &mut [R]) -> &mut [u64] {
    let bytes = std::mem::size_of_val(rows);
    assert_eq!(bytes % 8, 0);
    assert_eq!(std::mem::align_of::<R>() % 8, 0);
    // SAFETY: `rows` is a contiguous, 8-byte-aligned slice of `#[repr(C)]` packed rows whose fields
    // are all 8-byte words; the slice covers precisely the rows' memory and inherits their
    // exclusive borrow.
    unsafe { std::slice::from_raw_parts_mut(rows.as_mut_ptr() as *mut u64, bytes / 8) }
}

pub(crate) fn rows_as_words<R>(rows: &[R]) -> &[u64] {
    let bytes = std::mem::size_of_val(rows);
    assert_eq!(bytes % 8, 0);
    // SAFETY: `rows` is a contiguous, initialised, 8-byte-aligned slice of `#[repr(C)]` rows whose
    // fields are all 8-byte words; reinterpreting its bytes as `u64` reads no padding.
    unsafe { std::slice::from_raw_parts(rows.as_ptr() as *const u64, bytes / 8) }
}
