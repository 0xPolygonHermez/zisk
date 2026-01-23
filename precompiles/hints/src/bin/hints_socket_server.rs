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

    println!("Client connected! Starting data transfer...");

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

    // File structure:
    // - First 8 bytes: header
    // - Middle: batches of hints (each hint = 26 * 8 = 208 bytes)
    // - Last 8 bytes: footer

    const HINT_SIZE: usize = 26 * 8; // 208 bytes per hint
    const HINTS_PER_BATCH: usize = 100;
    const BATCH_SIZE: usize = HINTS_PER_BATCH * HINT_SIZE; // 20,800 bytes

    if file_data.len() < 16 {
        eprintln!("Error: File too small (need at least 16 bytes for header+footer)");
        return Ok(());
    }

    let mut offset = 0;
    let mut message_num = 0;

    loop {
        if shutdown.load(Ordering::Relaxed) {
            println!("\nShutdown requested, exiting...");
            break;
        }

        if offset >= file_data.len() {
            // All data sent
            println!("All data sent successfully!");
            println!("Connection active. Press '0' to close...");
            while !shutdown.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
            }
            break;
        }

        // Determine what to send in this message
        let (start, end) = if offset == 0 {
            // First message: 8 bytes header
            (0, 8)
        } else if offset + 8 >= file_data.len() {
            // Last message: final 8 bytes
            (file_data.len() - 8, file_data.len())
        } else {
            // Middle messages: batches of hints
            let data_end = file_data.len() - 8; // Before footer
            let remaining_data = data_end - offset;
            let batch_size = std::cmp::min(BATCH_SIZE, remaining_data);
            (offset, offset + batch_size)
        };

        let chunk = &file_data[start..end];

        match writer.write(chunk) {
            Ok(_) => {
                message_num += 1;
                println!(
                    "Message {}: Sent {} bytes (offset {}-{})",
                    message_num,
                    chunk.len(),
                    start,
                    end
                );
                offset = end;
            }
            Err(e) => {
                eprintln!("Error writing to Unix socket: {}", e);
                break;
            }
        }
    }

    println!("Closing connection...");
    let _ = writer.close();
    println!("Server shutting down...");
    Ok(())
}
