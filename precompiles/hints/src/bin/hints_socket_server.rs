//! Hints Unix Socket Server
//!
//! A development tool that opens a Unix domain socket, writes binary file contents to it,
//! and waits for the user to press '0' to close.
//!
//! Usage: hints-socket-server <binary_file> <socket_path>
//!   Example: hints-socket-server hints.bin /tmp/hints.sock

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use zisk_common::io::{StreamWrite, UnixSocketStreamWriter};

/// Reads binary file and returns its contents
fn read_binary_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <binary_file> <socket_path>", args[0]);
        eprintln!("Example: {} hints.bin /tmp/hints.sock", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    let socket_path = &args[2];

    // Read the binary file
    let file_data = read_binary_file(file_path)?;
    println!("Read {} bytes from: {}", file_data.len(), file_path);

    println!("========================================");
    println!("Hints Unix Socket Server");
    println!("========================================");
    println!("Binary file: {}", file_path);
    println!("Socket path: {}", socket_path);
    println!();

    // Create the Unix socket writer (server)
    let mut writer = UnixSocketStreamWriter::new(socket_path).map_err(io::Error::other)?;

    println!("Unix socket server created successfully");
    println!("Waiting for client connection...");

    // Open the connection (waits for client to connect)
    writer.open().map_err(io::Error::other)?;

    println!("Client connected! Starting hint data transfer...");

    let shutdown = Arc::new(AtomicBool::new(false));

    // Spawn shutdown listener thread
    let shutdown_clone = Arc::clone(&shutdown);
    thread::spawn(move || {
        println!("Press '0' + Enter to close at any time");
        let stdin = io::stdin();
        let mut buffer = String::new();
        loop {
            buffer.clear();
            if stdin.read_line(&mut buffer).is_ok() && buffer.trim() == "0" {
                println!("Shutdown signal received!");
                shutdown_clone.store(true, Ordering::Relaxed);
                break;
            }
        }
    });

    // Sleep 500ms
    thread::sleep(Duration::from_millis(500));

    let mut offset = 0;
    let mut hint_count = 0;
    let mut first_hint = true;

    let start_time = std::time::Instant::now();
    loop {
        if shutdown.load(Ordering::Relaxed) {
            println!("\nShutdown requested, exiting...");
            break;
        }

        if offset >= file_data.len() {
            panic!("Reached end of file data unexpectedly!");
        }

        let mut hint_total_len = 8;

        let hint_header = u64::from_le_bytes(file_data[offset..offset + 8].try_into().unwrap());
        let hint_id = (hint_header >> 32) as u32 & 0x7FFF_FFFF;

        if first_hint {
            // HINT_START
            assert!(hint_id == 0, "Invalid hint file format: first hint must be START");
            println!("Received START hint");
            first_hint = false;
        }

        let hint_data_len = hint_header & 0x_FFFF_FFFF;
        let pad = (8 - (hint_data_len % 8)) % 8; // Padding to align to 8 bytes
        let data_len_with_pad = hint_data_len + pad as u64;

        hint_total_len += data_len_with_pad;

        if (offset + hint_total_len as usize) > file_data.len() {
            eprintln!("Error: Hint data length exceeds file ends");
            return Ok(());
        }

        let data_with_pad = &file_data[offset..offset + hint_total_len as usize];
        match writer.write(data_with_pad) {
            Ok(_) => {
                if hint_count % 100 == 0 && hint_id != 0 && hint_id != 1 {
                    println!("#{} Hint id: 0x{:x}, sent: {} bytes, offset: {}",
                        hint_count,
                        hint_id,
                        hint_total_len,
                        offset
                    );
                }
            }
            Err(e) => {
                eprintln!("Error writing to Unix socket: {}", e);
                break;
            }
        }
        offset += hint_total_len as usize;

        if hint_id != 1 && hint_id != 0 {
            hint_count += 1;
        }

        if hint_id == 1 {
            // HINT_END
            println!("Received END hint. All hints sent, total: {}, time elapsed: {:?}", hint_count, start_time.elapsed());
            break;
        }

    }

    println!("Closing connection...");
    let _ = writer.close();
    println!("Server shutting down...");
    Ok(())
}