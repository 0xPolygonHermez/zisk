use anyhow::Result;
use std::io::{Cursor, Write};
use std::{fs, path::PathBuf};
use tracing::info;
use zstd::Encoder;

/// Saves a proof data to disk.
///
/// Creates a unique filename to avoid overwriting existing proof files by appending
/// a counter suffix (_2, _3, etc.) if the initial filename already exists.
///
/// # Arguments
///
/// * `id` - A unique identifier for the proof
/// * `proof_folder` - The folder where proofs will be saved
/// * `proof_data` - The proof data as a vector of u64 values
/// * `with_zip` - Whether to also save a compressed version of the proof
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `CoordinatorError` on failure
pub fn save_proof(
    id: &str,
    proof_folder: PathBuf,
    proof_data: &[u64],
    with_zip: bool,
) -> Result<()> {
    // Ensure the proofs directory exists
    fs::create_dir_all(&proof_folder)?;

    // Generate unique filename to avoid overwriting existing files
    let mut raw_path = proof_folder.join(format!("proof_{}.fri", id));
    let mut zip_path = raw_path.with_extension("fri.compressed");
    let mut counter = 2;

    while fs::exists(&raw_path)? || (with_zip && fs::exists(&zip_path)?) {
        raw_path = proof_folder.join(format!("proof_{}_{}.fri", id, counter));
        zip_path = raw_path.with_extension("fri.compressed");
        counter += 1;
    }

    // Convert Vec<u64> to bytes safely
    let proof_bytes = bytemuck::cast_slice::<u64, u8>(proof_data);

    // Write raw proof file
    fs::write(&raw_path, proof_bytes)?;

    // Calculate compression statistics
    let raw_size = proof_bytes.len();

    info!("Final proof compression completed:");
    info!("  Raw: {} ({} bytes)", raw_path.display(), raw_size);

    if with_zip {
        // Compress proof data and write to file
        let zip_size = save_zip_proof(proof_bytes, &zip_path, 1)?;

        let ratio = zip_size as f64 / raw_size as f64;

        info!("  Compressed: {} ({} bytes, ratio: {:.2}x)", zip_path.display(), zip_size, ratio);
    }

    Ok(())
}

/// Compresses data using zstd and writes it to a file.
///
/// # Arguments
///
/// * `data` - The raw data to compress
/// * `zip_path` - Path where the compressed file will be written
/// * `compression_level` - Compression level (1 = fastest, 22 = best compression)
///
/// # Returns
///
/// Returns the compressed size in bytes
pub fn save_zip_proof(
    data: &[u8],
    zip_path: &std::path::Path,
    compression_level: i32,
) -> Result<usize> {
    // Compress data in memory using zstd
    let mut compressed_buffer = Cursor::new(Vec::new());

    let mut encoder = Encoder::new(&mut compressed_buffer, compression_level)?;
    encoder.write_all(data)?;
    encoder.finish()?;

    // Extract compressed data and get size
    let compressed_data = compressed_buffer.into_inner();
    let compressed_size = compressed_data.len();

    // Write compressed data to file
    fs::write(zip_path, &compressed_data)?;

    Ok(compressed_size)
}
