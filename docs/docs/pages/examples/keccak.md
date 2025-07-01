---
description: Keccak Example
---

# Keccak Example

This example demonstrates how to compute Keccak-256 hashes using ZisK. The program takes a number `n` as input and performs Keccak-256 hashing `n` times sequentially, where each iteration hashes the result of the previous iteration.

## Overview

Keccak-256 is a cryptographic hash function that is part of the SHA-3 family. It's widely used in blockchain applications, particularly in Ethereum for generating addresses and transaction hashes.

This example showcases:
- Reading input data from ZisK
- Sequential cryptographic hashing operations
- Using the `tiny-keccak` crate for Keccak-256 computation
- Outputting hash results as multiple 32-bit values
- Iterative computation patterns in ZisK

## Program Code

### `main.rs`

```rust
// This example program takes a number `n` as input and computes the Keccak-256 hash `n` times sequentially.

// Mark the main function as the entry point for ZisK
#![no_main]
ziskos::entrypoint!(main);

use byteorder::ByteOrder;
use std::convert::TryInto;
use tiny_keccak::{Hasher, Keccak};
use ziskos::{read_input, set_output};

fn main() {
    // Read the input data as a byte array from ziskos
    let input: Vec<u8> = read_input();

    // Convert the input data to a u64 integer
    let n: u64 = match input.try_into() {
        Ok(input_bytes) => u64::from_le_bytes(input_bytes),
        Err(input) => panic!(
            "Invalid input length. Expected 8 bytes, got {}",
            input.len()
        ),
    };

    let mut hash = [0u8; 32];

    // Compute Keccak-256 hashing 'n' times
    for _ in 0..n {
        let mut hasher = Keccak::v256();
        hasher.update(&hash);
        hasher.finalize(&mut hash);
    }

    // Split 'hash' value into chunks of 32 bits and write them to ziskos output
    for i in 0..8 {
        let val = byteorder::BigEndian::read_u32(&mut hash[i * 4..i * 4 + 4]);
        set_output(i, val);
    }
}
```

### `Cargo.toml`

```toml
[package]
name = "keccak"
version = "0.1.0"
edition = "2021"
default-run = "keccak"

[dependencies]
byteorder = "1.5.0"
tiny-keccak = { version = "2.0.0", features = ["keccak"] }
ziskos = { git = "https://github.com/0xPolygonHermez/zisk.git" }
```

### `build.rs`

The `build.rs` script automatically generates the input file with a default value:

```rust
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

// Define constants for the directory and input file name
const OUTPUT_DIR: &str = "build/";
const FILE_NAME: &str = "input.bin";

fn main() -> io::Result<()> {
    let n: u64 = 20;

    // Ensure the output directory exists
    let output_dir = Path::new(OUTPUT_DIR);
    if !output_dir.exists() {
        // Create the directory and any necessary parent directories
        fs::create_dir_all(output_dir)?; 
    }

    // Create the file and write the 'n' value in little-endian format
    let file_path = output_dir.join(FILE_NAME);
    let mut file = File::create(&file_path)?;
    file.write_all(&n.to_le_bytes())?; 

    Ok(())
}
```

## Key Features

### Input Handling
- Reads an 8-byte input representing a `u64` value for the number of iterations `n`
- Uses little-endian byte order for input parsing
- Includes error handling for invalid input lengths

### Keccak-256 Computation
- Uses the `tiny-keccak` crate for efficient Keccak-256 hashing
- Performs sequential hashing where each iteration uses the previous hash as input
- Starts with an initial hash of all zeros (32 bytes)
- Each iteration creates a new hasher instance for clean computation

### Hash Chain Process
1. Initialize with a 32-byte array of zeros
2. For each iteration:
   - Create a new Keccak-256 hasher
   - Update the hasher with the current hash value
   - Finalize to get the new hash
3. After `n` iterations, output the final hash

### Output Format
- Splits the 256-bit (32-byte) hash into eight 32-bit chunks
- Uses big-endian byte order for output (standard for hash representations)
- Sets each chunk to outputs[0] through outputs[7]

## Running the Example

### Build and Execute

1. **Build the program:**
   ```bash
   cargo-zisk build --release
   ```

2. **Run with the default input (n=20):**
   ```bash
   cargo-zisk run --release -i build/input.bin
   ```

3. **Create custom input:**
   ```bash
   # Create input for n=5
   python3 -c "import struct; open('custom_input.bin', 'wb').write(struct.pack('<Q', 5))"
   
   # Run with custom input
   cargo-zisk run --release -i custom_input.bin
   ```

### Expected Results

The output will be eight 32-bit values representing the final Keccak-256 hash after `n` iterations. For example:

- **n=1**: Keccak-256 of 32 zero bytes
- **n=2**: Keccak-256 of the result from n=1
- **n=20**: Keccak-256 applied 20 times sequentially

Each run will produce different hash values due to the iterative nature of the computation.

## Use Cases

This example is particularly useful for:

### Blockchain Applications
- Transaction hash computation
- Block hash calculations
- Merkle tree construction
- Address generation

### Proof of Work Simulations
- Demonstrating iterative hashing patterns
- Mining algorithm prototypes
- Hash-based puzzles

### Cryptographic Research
- Hash chain analysis
- Performance benchmarking of cryptographic operations
- Side-channel analysis in controlled environments

## Performance Considerations

### Computational Complexity
- Time complexity: O(n) where n is the number of iterations
- Each Keccak-256 operation has fixed computational cost
- Memory usage remains constant regardless of iteration count

### ZisK-Specific Optimizations
- The `tiny-keccak` crate is optimized for performance
- Sequential hashing allows for predictable execution patterns
- Large values of `n` may require increasing the `--max-steps` parameter

### Scaling Considerations
```bash
# For large iteration counts, increase max steps
ziskemu -e target/riscv64ima-zisk-zkvm-elf/release/keccak -i build/input.bin -n 50000000
```

## Generate Proof

Follow the standard ZisK proof generation process:

1. **Program setup:**
   ```bash
   cargo-zisk rom-setup -e target/riscv64ima-zisk-zkvm-elf/release/keccak -k $HOME/.zisk/provingKey
   ```

2. **Verify constraints:**
   ```bash
   cargo-zisk verify-constraints -e target/riscv64ima-zisk-zkvm-elf/release/keccak -i build/input.bin
   ```

3. **Generate proof:**
   ```bash
   cargo-zisk prove -e target/riscv64ima-zisk-zkvm-elf/release/keccak -i build/input.bin -o proof -a -y
   ```

## Advanced Usage

### Custom Input Generation

Create more sophisticated input files for testing:

```python
import struct

def create_keccak_input(n, filename):
    """Create binary input file for keccak example"""
    with open(filename, 'wb') as f:
        f.write(struct.pack('<Q', n))

# Create inputs for different iteration counts
create_keccak_input(1, 'input_1.bin')      # Single hash
create_keccak_input(100, 'input_100.bin')  # 100 iterations
create_keccak_input(1000, 'input_1k.bin')  # 1000 iterations
```

### Verification of Results

You can verify the results by running equivalent computations in other environments:

```python
from Crypto.Hash import keccak

def verify_keccak_chain(n):
    """Verify the Keccak chain computation"""
    hash_val = b'\x00' * 32  # Start with 32 zero bytes
    
    for _ in range(n):
        hasher = keccak.new(digest_bits=256)
        hasher.update(hash_val)
        hash_val = hasher.digest()
    
    # Convert to 32-bit chunks (big-endian)
    chunks = []
    for i in range(0, 32, 4):
        chunk = int.from_bytes(hash_val[i:i+4], 'big')
        chunks.append(chunk)
    
    return chunks

# Verify results
result = verify_keccak_chain(20)
print("Expected output chunks:", result)
```

This example demonstrates the power of ZisK for cryptographic computations and provides a foundation for more complex blockchain-related zero-knowledge applications.