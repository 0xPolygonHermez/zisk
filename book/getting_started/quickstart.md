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

The first step is to generate a new example project using the `cargo-zisk new <name>` command. This command creates a new directory named `<name>` in your current directory. For example:
```bash
cargo-zisk new sha_hasher
cd sha_hasher
```

This will create a project with the following structure:

```
.
├── common
|   ├── src
|   |    └── main.rs
|   └── Cargo.toml
├── guest
|   ├── src
|   |    └── main.rs
|   └── Cargo.toml
├── host
|   ├── src
|   |    └── main.rs
|   ├── bin
|   |    ├── execute.rs
|   |    ├── minimal.rs
|   |    ├── prove.rs
|   |    ├── plonk.rs
|   |    └── run.rs
|   ├── Cargo.toml
|   └── build.rs
└── Cargo.toml
```

The example program takes a number `n` as input and computes the SHA-256 hash `n` times.

## Build

The next step is to build the program to generate an ELF file (RISC-V), which will be used later to generate the proof. Execute:

```bash
cargo build --release
```

This command builds the program using the `zkvm` target. The resulting `sha_hasher` ELF file (without extension) is generated in the `./target/elf/riscv64ima-zisk-zkvm-elf/release` directory.

## Execute

Before generating a proof, you can test the program using the ZisK emulator to ensure its correctness:

```bash
cargo run --release --bin execute
```

The emulator will execute the program and display the public outputs:

```
Public outputs:
  Hash: 0x36c1cb4f826ae42ceba848227e0c5f786178ca9dceca6772e5d728d09c30a2f6
  Iterations: 1000
  Magic number: 0xdeadbeef
```

These outputs should match the native execution, confirming the program works correctly.

## Prove

To generate a cryptographic proof of execution, run:

```bash
mkdir tmp
cargo run --release --bin prove
```

This will:
1. Execute the program and generate the execution trace
2. Compute witness values for all state machines
3. Generate the polynomial commitments
4. Create the zk-STARK proof

The proof will be saved in the `./tmp` directory. This process may take several minutes depending on the program complexity.

## Compressed Proof (Optional)

After generating the proof, you can optionally create a compressed version to reduce the proof size:

```bash
cargo run --release --bin minimal
```

This generates an additional compressed proof on top of the existing one using recursive composition. The compressed proof is significantly smaller while maintaining the same security guarantees.
