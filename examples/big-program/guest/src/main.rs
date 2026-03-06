// This example program processes large u64 input data (250MB - 1GB+)
// Input size is controlled by INPUT_SIZE_MB environment variable in host build.rs

#![no_main]
ziskos::entrypoint!(main);

fn main() {
    // Get zero-copy slice directly from INPUT_ADDR (no RAM allocation!)
    let data_bytes = ziskos::read_input_slice();

    // Reinterpret bytes as &[u64] - still zero-copy
    let data: &[u64] = unsafe {
        core::slice::from_raw_parts(data_bytes.as_ptr() as *const u64, data_bytes.len() / 8)
    };

    // Sum all values - no heap allocation needed
    let mut sum: u64 = 0;
    for &value in data {
        sum = sum.wrapping_add(value);
    }

    // Commit the result
    ziskos::io::commit(&sum);
}
