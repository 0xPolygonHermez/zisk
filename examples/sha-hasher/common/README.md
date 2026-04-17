# SHA Hasher Common

Shared types between the SHA hasher guest and host programs.

## Purpose

This crate contains the `Output` struct definition using Solidity ABI encoding (`sol!` macro). 
By sharing this definition between guest and host, we ensure:

1. **Consistency**: Both programs use the exact same struct layout
2. **Maintainability**: Changes to the output format only need to be made in one place
3. **Type Safety**: The compiler ensures both sides agree on the structure

## Usage

### In Guest (no_std)
```rust
use sha_hasher_common::Output;
use alloy_sol_types::SolValue;

let output = Output { 
    hash: hash.into(), 
    iterations: n, 
    magic_number: 0xDEADBEEF 
};
let bytes = output.abi_encode();
ziskos::io::commit(&bytes);
```

### In Host
```rust
use sha_hasher_host::Output; // Re-exported from common

let output = Output { 
    hash: hash.into(), 
    iterations: n, 
    magic_number: 0xDEADBEEF 
};
let publics = PublicValues::write_abi(&output)?;
```
