# Precompiles

Precompiles are built-in system functions within ZisK’s operating system that accelerate computationally expensive and frequently used operations such as the Keccak-f permutation and Secp256k1 addition and doubling. 

These precompiles improve proving efficiency by offloading intensive computations from ZisK programs to dedicated, pre-integrated sub-processors. ZisK manages precompiles as system calls using the RISC-V `ecall` instruction.

## How Precompiles Work

Precompiles are primarily used to patch third-party crates, replacing costly operations with system calls. This ensures that commonly used cryptographic primitives like Keccak hashing and elliptic curve operations can be efficiently executed within ZisK programs.

Typically, precompiles are used to patch third-party crates that implement these operations and are then used as dependencies in the Zisk programs we write.

You can see [here](https://github.com/0xPolygonHermez/zisk-patch-tiny-keccak/tree/zisk) an example of the patched `tiny-keccak` crate.

## Supported Precompiles

⚠️ Currently, ZisK only supports the keccak precompile, but work is underway to introduce additional cryptographic primitives, including:
- Secp256k1 group operations.
- BN254 group operations.
- SHA2-256.

### Available Precompiles in ZisK

```rust
extern "C" {
    pub fn syscall_keccak_f(state: *mut [u64; 25]);
}
```
- `syscall_keccak_f`: Executes a Keccak permutation function on a 25-element state array.