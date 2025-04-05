# Precompiles

Precompiles are built-in system functions within ZisK’s operating system that accelerate computationally expensive and frequently used operations such as the Keccak-f permutation and Secp256k1 addition and doubling. 

These precompiles improve proving efficiency by offloading intensive computations from ZisK programs to dedicated, pre-integrated sub-processors. ZisK manages precompiles as system calls using the RISC-V `ecall` instruction.

## How Precompiles Work

Precompiles are primarily used to patch third-party crates, replacing costly operations with system calls. This ensures that commonly used cryptographic primitives like Keccak hashing and elliptic curve operations can be efficiently executed within ZisK programs.

Typically, precompiles are used to patch third-party crates that implement these operations and are then used as dependencies in the Zisk programs we write.

You can see [here](https://github.com/0xPolygonHermez/zisk-patch-tiny-keccak/tree/zisk) an example of the patched `tiny-keccak` crate.

### Available Precompiles in ZisK

Below is a summary of the precompiles currently available in ZisK:

- [syscall_keccak_f](https://github.com/0xPolygonHermez/zisk/tree/main/ziskos/entrypoint/src/syscalls/keccakf.rs): Executes a Keccak permutation function.
- [syscall_arith25](https://github.com/0xPolygonHermez/zisk/tree/main/ziskos/entrypoint/src/syscalls/arith256.rs): Executes a 256-bit multiplication and addition.
- [syscall_arith256_mod](https://github.com/0xPolygonHermez/zisk/tree/main/ziskos/entrypoint/src/syscalls/arith256_mod.rs): Executes a modular 256-bit multiplication and addition.
- [secp256k1_add](https://github.com/0xPolygonHermez/zisk/tree/main/ziskos/entrypoint/src/syscalls/secp256k1_add.rs): Executes the addition of two points on the Secp256k1 curve.
- [secp256k1_dbl](https://github.com/0xPolygonHermez/zisk/tree/main/ziskos/entrypoint/src/syscalls/secp256k1_dbl.rs): Executes the doubling of a point on the Secp256k1 curve.