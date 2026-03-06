use std::path::PathBuf;
use zisk_sdk::{build_program, ZiskIO, ZiskStdin};

fn main() {
    build_program("../guest");

    // Read input size from environment variable (in MB), default to 250MB
    let size_mb: usize =
        std::env::var("INPUT_SIZE_MB").ok().and_then(|s| s.parse().ok()).unwrap_or(250);

    // Make the size available to main.rs at compile time
    println!("cargo:rustc-env=INPUT_SIZE_MB={}", size_mb);
    println!("cargo:rerun-if-env-changed=INPUT_SIZE_MB");

    // Calculate number of u64 values
    // 1MB = 1,048,576 bytes = 131,072 u64 values
    const BYTES_PER_MB: usize = 1024 * 1024;
    const NUM_U64_PER_MB: usize = BYTES_PER_MB / 8;
    let num_u64 = size_mb * NUM_U64_PER_MB;

    println!("Generating {} u64 values (~{}MB of data)...", num_u64, size_mb);

    let mut data = Vec::with_capacity(num_u64);
    for i in 0..num_u64 {
        // Generate pseudo-random but deterministic data
        data.push((i as u64).wrapping_mul(1103515245).wrapping_add(12345));
    }

    println!("Writing input data...");
    let stdin_save = ZiskStdin::new();
    stdin_save.write(&data);

    // Save to file
    let path = PathBuf::from(format!("tmp/big_program_input_{}mb.bin", size_mb));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    stdin_save.save(&path).unwrap();

    println!("Input data saved to: {}", path.display());
    let file_size_mb = std::fs::metadata(&path).unwrap().len() / 1024 / 1024;
    println!("File size: {}MB", file_size_mb);
    println!("\nConfigured size: {}MB", size_mb);
    println!("To change: INPUT_SIZE_MB=512 cargo build --release");
}
