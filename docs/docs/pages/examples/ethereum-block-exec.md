---
description: Ethereum Block Execution Example
---

# Ethereum Block Execution Example

This example demonstrates how to perform stateless Ethereum block validation using ZisK. The program validates an Ethereum block by executing all transactions within it and verifying the state transitions without requiring the full Ethereum state.

## Overview

Stateless block validation is a crucial component of Ethereum's scalability roadmap. Instead of maintaining the full state, validators can verify blocks using witness data that contains only the necessary state information for validation.

This example showcases:
- Stateless Ethereum block validation
- Integration with the Reth Ethereum client library
- Complex input data handling with serialization
- Real-world blockchain computation in zero-knowledge proofs
- Advanced ZisK program structure for Ethereum applications

## Program Code

### `main.rs`

```rust
#![no_main]
ziskos::entrypoint!(main);

extern crate alloc;

use alloc::sync::Arc;
use reth_chainspec::ChainSpec;
use reth_evm_ethereum::EthEvmConfig;
use reth_stateless::{fork_spec::ForkSpec, validation::stateless_validation, StatelessInput};

fn main() {
    let (input, fork_spec): (StatelessInput, ForkSpec) =
        bincode::deserialize(&ziskos::read_input()).unwrap();
    let chain_spec: Arc<ChainSpec> = Arc::new(fork_spec.into());
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    stateless_validation(input.block, input.witness, chain_spec, evm_config).unwrap();
    
    println!("Validation successful!");
}
```

### `Cargo.toml`

```toml
[package]
name = "exec_eth_block"
version = "0.1.0"
edition = "2021"
default-run = "exec_eth_block"

[dependencies]
byteorder = "1.5.0"
ziskos = { git = "https://github.com/0xPolygonHermez/zisk.git" }

bincode = "1.3"
reth-stateless = { git = "https://github.com/paradigmxyz/reth", rev = "03364a836774c72f4e354de924330fee6a41be68" }
reth-ethereum-primitives = { git = "https://github.com/paradigmxyz/reth", rev = "03364a836774c72f4e354de924330fee6a41be68", features = [
    "serde",
    "serde-bincode-compat",
] }
reth-primitives-traits = { git = "https://github.com/paradigmxyz/reth", rev = "03364a836774c72f4e354de924330fee6a41be68", features = [
    "serde",
    "serde-bincode-compat",
] }
alloy-primitives = { version = "1.2.0", default-features = false, features = [
    "map-foldhash",
    "serde",
    "tiny-keccak",
] }
reth-evm-ethereum = { git = "https://github.com/paradigmxyz/reth", rev = "03364a836774c72f4e354de924330fee6a41be68" }
reth-chainspec = { git = "https://github.com/paradigmxyz/reth", rev = "03364a836774c72f4e354de924330fee6a41be68" }
```

## Key Features

### Stateless Validation
- **No Full State Required**: Validates blocks without maintaining the complete Ethereum state
- **Witness-Based**: Uses cryptographic witness data to prove state transitions
- **Efficient**: Reduces storage requirements while maintaining security guarantees

### Reth Integration
- **Production-Ready**: Uses the Reth Ethereum client library for robust validation
- **Fork Compatibility**: Supports different Ethereum hard forks through `ForkSpec`
- **EVM Execution**: Full Ethereum Virtual Machine execution for transaction processing

### Input Structure
The program expects serialized input containing:
- **StatelessInput**: Contains the block data and witness information
- **ForkSpec**: Specifies which Ethereum hard fork rules to apply

## Architecture

### Data Flow
1. **Input Deserialization**: Deserializes the block and witness data using `bincode`
2. **Chain Specification**: Converts fork specification to chain configuration
3. **EVM Configuration**: Sets up the Ethereum Virtual Machine with proper parameters
4. **Validation**: Executes stateless validation of the block

### Validation Process
1. **Block Header Validation**: Verifies block header consistency
2. **Transaction Execution**: Executes all transactions in the block
3. **State Root Verification**: Confirms the final state root matches the block header
4. **Witness Verification**: Validates that the witness data is consistent

## Input Data Format

### Creating Input Data

The input data must be a serialized combination of:
- `StatelessInput`: Block and witness data
- `ForkSpec`: Ethereum fork specification

```rust
use bincode;
use reth_stateless::{StatelessInput, fork_spec::ForkSpec};

// Example of creating input data (would typically be done outside ZisK)
let stateless_input = StatelessInput {
    block: block_data,
    witness: witness_data,
};

let fork_spec = ForkSpec::Shanghai; // or other fork
let serialized = bincode::serialize(&(stateless_input, fork_spec)).unwrap();

// Write to input.bin
std::fs::write("input.bin", serialized).unwrap();
```

### Witness Data
The witness contains:
- **Account Data**: Account balances, nonces, and code
- **Storage Proofs**: Merkle proofs for accessed storage slots
- **Code**: Smart contract bytecode accessed during execution
- **State Proofs**: Merkle proofs for state transitions

## Running the Example

### Prerequisites

1. **Input Data**: You need a properly formatted `input.bin` file containing:
   - Ethereum block data
   - Corresponding witness data
   - Fork specification

2. **Block Data Source**: Obtain block data from:
   - Ethereum node RPC calls
   - Block explorers
   - Pre-generated test data

### Build and Execute

1. **Build the program:**
   ```bash
   cargo-zisk build --release
   ```

2. **Run with input data:**
   ```bash
   cargo-zisk run --release -i block_build/input.bin
   ```

3. **For large blocks, increase max steps:**
   ```bash
   cargo-zisk run --release -i block_build/input.bin --max-steps 100000000
   ```

### Performance Tuning

Block validation can be computationally intensive. Consider these optimizations:

```bash
# Run with performance metrics
cargo-zisk run --release -i block_build/input.bin -m

# Run with execution statistics
cargo-zisk run --release -i block_build/input.bin -x

# Increase memory and steps for large blocks
ziskemu -e target/riscv64ima-zisk-zkvm-elf/release/exec_eth_block \
        -i block_build/input.bin \
        -n 1000000000
```

## Use Cases

### Layer 2 Scaling
- **Rollup Validation**: Prove correct execution of Layer 2 blocks
- **Fraud Proofs**: Generate proofs for disputed transactions
- **State Compression**: Reduce on-chain storage requirements

### Cross-Chain Bridges
- **Block Verification**: Prove Ethereum block validity on other chains
- **State Relay**: Transfer Ethereum state information securely
- **Interoperability**: Enable cross-chain applications

### Compliance and Auditing
- **Transaction Verification**: Prove specific transactions occurred
- **Regulatory Compliance**: Demonstrate adherence to rules
- **Audit Trails**: Create verifiable execution records

## Advanced Configuration

### Fork Specifications

Different Ethereum hard forks have different validation rules:

```rust
// Examples of different fork specifications
ForkSpec::London        // EIP-1559 fee market
ForkSpec::Shanghai      // Beacon chain withdrawals
ForkSpec::Cancun        // Blob transactions (EIP-4844)
```

### Chain Specifications

Customize for different Ethereum networks:

```rust
// Mainnet configuration
ChainSpec::mainnet()

// Testnet configurations
ChainSpec::goerli()
ChainSpec::sepolia()

// Custom configurations for private networks
```

## Generate Proof

Follow the standard ZisK proof generation process:

1. **Program setup:**
   ```bash
   cargo-zisk rom-setup -e target/riscv64ima-zisk-zkvm-elf/release/exec_eth_block -k $HOME/.zisk/provingKey
   ```

2. **Verify constraints:**
   ```bash
   cargo-zisk verify-constraints -e target/riscv64ima-zisk-zkvm-elf/release/exec_eth_block -i block_build/input.bin
   ```

3. **Generate proof:**
   ```bash
   cargo-zisk prove -e target/riscv64ima-zisk-zkvm-elf/release/exec_eth_block -i block_build/input.bin -o proof -a -y
   ```

### Parallel Proof Generation

For complex blocks, use parallel processing:

```bash
mpirun --bind-to none -np 4 -x OMP_NUM_THREADS=8 \
       target/release/cargo-zisk prove \
       -e target/riscv64ima-zisk-zkvm-elf/release/exec_eth_block \
       -i block_build/input.bin -o proof -a -y
```

## Troubleshooting

### Common Issues

1. **Serialization Errors**:
   - Ensure input data uses compatible `bincode` version
   - Verify data structure matches expected format

2. **Memory Issues**:
   - Large blocks may require more memory
   - Consider using concurrent proof generation with appropriate memory allocation

3. **Fork Compatibility**:
   - Ensure fork specification matches the block's network rules
   - Update Reth dependencies for latest fork support

4. **Execution Timeouts**:
   - Complex blocks may require increased step limits
   - Monitor execution with metrics flags

### Debug Mode

Run in debug mode for detailed execution information:

```bash
RUST_LOG=debug cargo-zisk run --release -i block_build/input.bin
```

## Integration Ideas

### Web3 Application Integration

```javascript
// Example: Generating input data from web3
const web3 = new Web3('https://mainnet.infura.io/v3/YOUR-PROJECT-ID');

async function prepareBlockInput(blockNumber) {
    const block = await web3.eth.getBlock(blockNumber, true);
    const witness = await generateWitness(block); // Custom witness generation
    const forkSpec = determineForkSpec(blockNumber);
    
    return {
        block: block,
        witness: witness,
        forkSpec: forkSpec
    };
}
```

### Smart Contract Verification

```solidity
// Example: Verifying ZisK proofs on-chain
contract BlockValidator {
    function verifyBlock(
        bytes calldata proof,
        bytes32 blockHash,
        uint256 blockNumber
    ) external returns (bool) {
        // Verify ZisK proof of block execution
        return ZiskVerifier.verify(proof, blockHash, blockNumber);
    }
}
```

This example demonstrates the power of ZisK for real-world Ethereum applications, enabling scalable and verifiable blockchain computation through zero-knowledge proofs.