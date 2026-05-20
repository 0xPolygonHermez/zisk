//! Processes a large u64 input slice (250MB - 1GB+) via zero-copy access
//! and commits the wrapping sum.

#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let data_bytes = ziskos::io::read_input_slice();

    let data: &[u64] = unsafe {
        core::slice::from_raw_parts(data_bytes.as_ptr() as *const u64, data_bytes.len() / 8)
    };

    let mut sum: u64 = 0;
    for &value in data {
        sum = sum.wrapping_add(value);
    }

    ziskos::io::commit(&sum);
}
