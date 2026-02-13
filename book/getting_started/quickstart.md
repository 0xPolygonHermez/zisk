# Quickstart

In this guide, you will learn how to install ZisK, create a simple program and run it using ZisK.

## Installation

ZisK currently supports **Linux x86_64** and **macOS** platforms (see note below).

**Note:** On **macOS**, proof generation is not yet optimized, so some proofs may take longer to generate.

**Ubuntu 22.04 or higher** is required.

**macOS 14 or higher** with [Xcode](https://developer.apple.com/xcode/) installed is required.

1. Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed.

2. Install all required dependencies with:
    - **Ubuntu**:
        ```bash
        sudo apt-get install -y xz-utils jq curl build-essential qemu-system libomp-dev libgmp-dev nlohmann-json3-dev protobuf-compiler uuid-dev libgrpc++-dev libsecp256k1-dev libsodium-dev libpqxx-dev nasm libopenmpi-dev openmpi-bin openmpi-common libclang-dev clang gcc-riscv64-unknown-elf
        ```
    - **macOS**:
        ```bash
        brew reinstall jq curl libomp protobuf openssl nasm pkgconf open-mpi libffi nlohmann-json libsodium
        ```

3. To install ZisK using ziskup, run the following command in your terminal:
    ```bash
    curl https://raw.githubusercontent.com/0xPolygonHermez/zisk/main/ziskup/install.sh | bash
    ```

## Create a Project

The first step is to generate a new example project using the `cargo-zisk sdk new <name>` command. This command creates a new directory named `<name>` in your current directory. For example:
```bash
cargo-zisk sdk new sha_hasher
cd sha_hasher
```

This will create a project with the following structure:

```
.
├── build.rs
├── Cargo.toml
├── .gitignore
├── guest
|   ├── src
|   |    └── main.rs
|   └── Cargo.toml
└── host
    ├── src
    |    └── main.rs
    ├── bin
    |    ├── compressed.rs
    |    ├── execute.rs
    |    ├── prove.rs
    |    ├── plonk.rs
    |    ├── verify-constraints.rs
    |    └── ziskemu.rs
    ├── build.rs
    └── Cargo.toml
```

The example program takes a number `n` as input and computes the SHA-256 hash `n` times.

The `build.rs` file generates an `input.bin` file containing the value of `n` (e.g., 20). This file is used in `main.rs` as input to calculate the hash.

## Build

The next step is to build the program using the `cargo-zisk` command to generate an ELF file (RISC-V), which will be used later to generate the proof. Execute:

```bash
cargo-zisk build --release
```

This command builds the program using the `zkvm` target. The resulting `sha_hasher` ELF file (without extension) is generated in the `./target/elf/riscv64ima-zisk-zkvm-elf/release` directory.

## Execute

Before generating a proof, you can test the program using the ZisK emulator to ensure its correctness:

```bash
cargo run --release --bin ziskemu
```

The emulator will execute the program and display the public outputs:

```
public 0: 0x98211882
public 1: 0xbd13089b
public 2: 0x6ccf1fca
public 3: 0x81f7f0e4
public 4: 0xabf6352a
public 5: 0x0c39c9b1
public 6: 0x1f142cac
public 7: 0x233f1280
```

These outputs should match the native execution, confirming the program works correctly.

## Verify Constraints

Once you've confirmed the program executes correctly, you can verify the constraints without generating a full proof. This is useful for debugging and ensuring correctness:

```bash
cargo run --release --bin verify-constraints
```

This command will:
1. Execute the program using the ZisK emulator
2. Generate the execution trace
3. Verify all arithmetic and logical constraints
4. Check that all state machine transitions are valid

If successful, you'll see:

```
✓ All constraints for Instance #0 of Main were verified
✓ All constraints for Instance #0 of Rom were verified
...
✓ All global constraints were successfully verified
```

## Prove

To generate a cryptographic proof of execution, run:

```bash
cargo run --release --bin prove
```

This will:
1. Execute the program and generate the execution trace
2. Compute witness values for all state machines
3. Generate the polynomial commitments
4. Create the zk-STARK proof

The proof will be saved in the `./proof` directory. This process may take several minutes depending on the program complexity.

## Compressed Proof (Optional)

After generating the proof, you can optionally create a compressed version to reduce the proof size:

```bash
cargo run --release --bin compressed
```

This generates an additional compressed proof on top of the existing one using recursive composition. The compressed proof is significantly smaller while maintaining the same security guarantees.
