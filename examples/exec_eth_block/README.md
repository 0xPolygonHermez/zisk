# Ethereum Block Execution in ZisK

This example demonstrates how to execute Ethereum blocks within the ZisK zero-knowledge virtual machine using stateless validation.

## Overview

This ZisK program performs stateless validation of Ethereum blocks using the `reth-stateless` library. Unlike traditional block execution that requires access to the full blockchain state, this implementation uses witness data to validate blocks without maintaining state locally.

## Key Features

- **Stateless Validation**: Validates Ethereum blocks using only the block data and witness information
- **ZisK Integration**: Runs entirely within the ZisK zero-knowledge virtual machine
- **Ethereum Compatibility**: Uses the same validation logic as the Reth Ethereum client
- **Test Block Support**: Designed to work with blocks from the Ethereum specification tests

## Prerequisites

- ZisK toolchain installed
- `cargo-zisk` build tool
- `ziskemu` emulator

## Building the Example

To build the example, run:

```bash
cargo-zisk build --release
```

This will compile the program for the ZisK RISC-V target (`riscv64ima-zisk-zkvm-elf`).

## Running the Example

To execute the program with a test block:

```bash
ziskemu -e target/riscv64ima-zisk-zkvm-elf/release/exec_eth_block -i block_build/input.bin
```

### Input Format

The input file (`block_build/input.bin`) should contain a serialized tuple of:
- `StatelessInput`: Contains the block data and witness information
- `ForkSpec`: Specifies which Ethereum fork rules to apply

## Important Notes

⚠️ **Special Block Type**: The blocks executed by this program are not normal Ethereum mainnet blocks. They are specially crafted blocks from the Ethereum specification tests that include witness data for stateless execution.

## Technical Implementation

The program:

1. Deserializes the input to extract block data, witness, and fork specification
2. Creates an Ethereum chain specification from the fork rules
3. Configures the Ethereum Virtual Machine (EVM) with the appropriate settings
4. Performs stateless validation using the `reth-stateless` library
5. Outputs validation results

### Dependencies

- `reth-stateless`: Core stateless validation logic
- `reth-ethereum-primitives`: Ethereum data types and primitives
- `reth-evm-ethereum`: Ethereum EVM configuration
- `reth-chainspec`: Chain specification handling
- `alloy-primitives`: Low-level Ethereum primitives
- `ziskos`: ZisK operating system interface

## Use Cases

This example is particularly useful for:

- Zero-knowledge proofs of Ethereum block execution
- Stateless client implementations
- Testing Ethereum fork compatibility
- Educational purposes for understanding stateless validation

## Output

Upon successful execution, the program will output:
```
Validation successful!
```

If validation fails, the program will panic with an error message describing the validation failure.