//! Reads an input that can fill the whole 2GB input region, via zero-copy access, and commits
//! what it read so the host can check it (see examples/input-2gb/host).
//!
//! The input is a slice of u64. The first one is the read stride `s` (0 is taken as 1): the guest
//! reads every `s`-th u64, starting at index 0, plus always the last one, so with `s` = 1 it reads
//! them all, and with a larger `s` it still spans the whole region while touching fewer distinct
//! addresses.
//!
//! The public output is 4 little-endian u64 values: the number of u64 read, their wrapping sum,
//! and the first and the last u64 of the input, i.e. the ones at both ends of the input region.

#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let data_bytes = ziskos::io::read_slice();
    assert!(data_bytes.len() % 8 == 0, "input length must be a multiple of 8");
    assert!(!data_bytes.is_empty(), "input must hold at least the stride");

    // SAFETY: the input payload starts at INPUT_ADDR + 16, which is 8-byte aligned, and its length
    // is a multiple of 8 (checked above).
    let data: &[u64] = unsafe {
        core::slice::from_raw_parts(data_bytes.as_ptr() as *const u64, data_bytes.len() / 8)
    };

    let stride = data[0].max(1) as usize;
    let last_index = data.len() - 1;

    let mut count: u64 = 0;
    let mut sum: u64 = 0;
    for &value in data.iter().step_by(stride) {
        sum = sum.wrapping_add(value);
        count += 1;
    }
    if last_index % stride != 0 {
        sum = sum.wrapping_add(data[last_index]);
        count += 1;
    }

    let mut out = [0u8; 32];
    for (i, v) in [count, sum, data[0], data[last_index]].iter().enumerate() {
        out[i * 8..(i + 1) * 8].copy_from_slice(&v.to_le_bytes());
    }
    ziskos::io::commit_slice(&out);
}
