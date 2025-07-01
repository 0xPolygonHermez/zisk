# Fibonacci Number Computation in ZisK

This example demonstrates how to compute Fibonacci numbers within the ZisK zero-knowledge virtual machine.

## Overview

This ZisK program takes a number `n` as input and computes the nth Fibonacci number using an efficient iterative algorithm. The program is designed to run entirely within the ZisK zero-knowledge virtual machine, making it suitable for generating zero-knowledge proofs of Fibonacci number computations.

## Key Features

- **Efficient Computation**: Uses iterative algorithm with O(n) time complexity
- **Overflow Handling**: Uses wrapping arithmetic to handle large Fibonacci numbers
- **ZisK Integration**: Runs natively in the ZisK zero-knowledge virtual machine
- **64-bit Output**: Supports Fibonacci numbers up to 64-bit precision
- **Automatic Input Generation**: Includes build script to generate test input

## Prerequisites

- ZisK toolchain installed
- `cargo-zisk` build tool
- `ziskemu` emulator

## Building the Example

To build the example, run:

```bash
cargo-zisk build --release
```

This will:
1. Execute the build script (`build.rs`) to generate the input file
2. Compile the program for the ZisK RISC-V target (`riscv64ima-zisk-zkvm-elf`)

## Running the Example

To execute the program:

```bash
ziskemu -e target/riscv64ima-zisk-zkvm-elf/release/fibonacci -i build/input.bin
```

### Input Configuration

The input is configured in `build.rs`. By default, it computes the 20th Fibonacci number:

```rust
let n: u64 = 20;
```

To compute a different Fibonacci number:
1. Edit the value of `n` in `build.rs`
2. Rebuild the project with `cargo-zisk build --release`

### Input Format

The input file (`build/input.bin`) contains:
- An 8-byte little-endian encoded `u64` representing the position `n` in the Fibonacci sequence

## Output Format

The program outputs the nth Fibonacci number as two 32-bit values:
- `output[0]`: Low 32 bits of the result
- `output[1]`: High 32 bits of the result
- `output[2-7]`: Set to 0 (unused)

To reconstruct the full 64-bit result:
```
result = (output[1] as u64) << 32 | (output[0] as u64)
```

## Technical Implementation

### Algorithm

The program uses an iterative approach to compute Fibonacci numbers:

```rust
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

### Key Features

- **Base Cases**: Handles F(0) = 0 and F(1) = 1 correctly
- **Wrapping Arithmetic**: Uses `wrapping_add` to handle overflow gracefully
- **Memory Efficient**: Uses constant space O(1)
- **ZisK Compatible**: Uses only operations supported by the ZisK VM

## Example Results

Here are some example Fibonacci numbers:

| n  | F(n)                  |
|----|----------------------|
| 10 | 55                   |
| 20 | 6,765                |
| 30 | 832,040              |
| 40 | 102,334,155          |
| 50 | 12,586,269,025       |

## Large Numbers and Overflow

For large values of `n`, the Fibonacci numbers will exceed 64-bit precision. The program handles this using wrapping arithmetic, which means:

- Results larger than 2^64 - 1 will wrap around
- This behavior is deterministic and consistent
- For cryptographic applications, this wrapping behavior is often acceptable

## Use Cases

This example is useful for:

- **Zero-Knowledge Proofs**: Proving computation of specific Fibonacci numbers
- **Benchmarking**: Testing ZisK VM performance with iterative algorithms
- **Educational**: Understanding ZisK program structure and I/O
- **Cryptographic Applications**: Where Fibonacci sequences are used in protocols

## Files Structure

- `src/main.rs`: Main program logic
- `build.rs`: Build script that generates input data
- `Cargo.toml`: Project configuration and dependencies
- `build/input.bin`: Generated input file (created by build script)

## Dependencies

- `ziskos`: ZisK operating system interface for I/O operations
- `byteorder`: For handling byte order conversions (though not used in current implementation)

## Customization

To modify the program for different use cases:

1. **Change Input Value**: Edit `n` in `build.rs`
2. **Multiple Inputs**: Modify the input format to handle multiple computations
3. **Different Algorithms**: Replace the iterative approach with matrix exponentiation for better asymptotic complexity
4. **Extended Precision**: Use big integer libraries for arbitrary precision arithmetic

## Notes

- The program expects exactly 8 bytes of input (one `u64` value)
- Invalid input lengths will cause the program to panic with an error message
- The ZisK VM ensures deterministic execution, making results reproducible