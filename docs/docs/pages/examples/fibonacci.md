---
description: Fibonacci Example
---

# Fibonacci Example

This example demonstrates how to compute the nth Fibonacci number using ZisK. The program takes a number `n` as input and returns the nth Fibonacci number using an iterative approach with overflow handling.

## Overview

The Fibonacci sequence is a series of numbers where each number is the sum of the two preceding ones: 0, 1, 1, 2, 3, 5, 8, 13, 21, 34, ...

This example showcases:
- Reading input data from ZisK
- Iterative computation with wrapping arithmetic
- Outputting 64-bit results as two 32-bit values
- Basic ZisK program structure

## Program Code

### `main.rs`

```rust
// This example program takes a number `n` as input and computes the nth Fibonacci number.

// Mark the main function as the entry point for ZisK
#![no_main]
ziskos::entrypoint!(main);

use std::convert::TryInto;
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

    // Compute the nth Fibonacci number
    let fib_result = fibonacci(n);

    // Output the Fibonacci result as two 32-bit values (low and high parts)
    let low = (fib_result & 0xFFFFFFFF) as u32;
    let high = ((fib_result >> 32) & 0xFFFFFFFF) as u32;

    set_output(0, low);
    set_output(1, high);

    // Set remaining outputs to 0
    for i in 2..8 {
        set_output(i, 0);
    }
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }

    let mut a = 0u64;
    let mut b = 1u64;

    for _ in 2..=n {
        let temp = a.wrapping_add(b);
        a = b;
        b = temp;
    }

    b
}
```

### `Cargo.toml`

```toml
[package]
name = "fibonacci"
version = "0.1.0"
edition = "2021"
default-run = "fibonacci"

[dependencies]
byteorder = "1.5.0"
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
- Reads an 8-byte input representing a `u64` value for `n`
- Uses little-endian byte order for input parsing
- Includes error handling for invalid input lengths

### Fibonacci Computation
- Uses an iterative approach for efficiency
- Handles edge cases (n ≤ 1)
- Uses `wrapping_add()` to handle potential overflow gracefully
- Time complexity: O(n), Space complexity: O(1)

### Output Format
- Splits the 64-bit Fibonacci result into two 32-bit parts
- Sets the low 32 bits to output[0] and high 32 bits to output[1]
- Initializes remaining output slots (2-7) to zero

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
   # Create input for n=30
   python3 -c "import struct; open('custom_input.bin', 'wb').write(struct.pack('<Q', 30))"
   
   # Run with custom input
   cargo-zisk run --release -i custom_input.bin
   ```

### Expected Results

For n=20, the 20th Fibonacci number is 6765:
- Output[0]: 6765 (low 32 bits)
- Output[1]: 0 (high 32 bits)
- Output[2-7]: 0

For n=50, the 50th Fibonacci number is 12586269025:
- Output[0]: 3832890881 (low 32 bits) 
- Output[1]: 2 (high 32 bits)
- Output[2-7]: 0

## Performance Considerations

- The iterative approach is more efficient than recursive implementation
- Large values of `n` will require more computation steps
- For very large `n` values, consider using the `--max-steps` flag when running

## Generate Proof

Follow the standard ZisK proof generation process:

1. **Program setup:**
   ```bash
   cargo-zisk rom-setup -e target/riscv64ima-zisk-zkvm-elf/release/fibonacci -k $HOME/.zisk/provingKey
   ```

2. **Verify constraints:**
   ```bash
   cargo-zisk verify-constraints -e target/riscv64ima-zisk-zkvm-elf/release/fibonacci -i build/input.bin
   ```

3. **Generate proof:**
   ```bash
   cargo-zisk prove -e target/riscv64ima-zisk-zkvm-elf/release/fibonacci -i build/input.bin -o proof -a -y
   ```

This example provides a solid foundation for understanding ZisK program structure and demonstrates efficient iterative computation with proper input/output handling.