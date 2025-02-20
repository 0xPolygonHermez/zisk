# Precompiles

Precompiles are built into ZisK operating systems and are used to accelerate expensive, frequently used operations such as keccak and secp256k1 (elliptic curve operations). These precompiles are managed by Zisk as system calls using the RISC-V `ecall` instruction.

Typically, precompiles are used to patch third-party crates that implement these operations and are then used as dependencies in the Zisk programs we write.

You can see [here](https://github.com/0xPolygonHermez/zisk-patch-tiny-keccak/tree/zisk) an example of the patched `tiny-keccak` crate.

⚠️ Currently, the only precompile supported by Zisk is keccak, but work is underway to add more precompiles shortly (e.g., secp256k1, bn254, etc.).

Available precompiles:

```rust
extern "C" {
    pub fn syscall_keccak_f(state: *mut [u64; 25]);
}
```