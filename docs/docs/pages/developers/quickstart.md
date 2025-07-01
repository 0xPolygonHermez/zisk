---
description: Quickstart 
---

# Quickstart

In this guide, you will learn how to install ZisK, create a simple program and run it using ZisK.

## Installation

ZisK currently supports **Linux x86_64** and **macOS** platforms (see note below).

>**Note:** Proof generation and verification on **macOS** are not yet supported. We’re actively working to add this functionality.

**Ubuntu 22.04 or higher** is required.

**macOS 14 or higher** is required.

1. Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed.

2. Install all required dependencies with:
    - **Ubuntu**:
        ```bash
        sudo apt-get install -y xz-utils jq curl build-essential qemu-system libomp-dev libgmp-dev nlohmann-json3-dev protobuf-compiler uuid-dev libgrpc++-dev libsecp256k1-dev libsodium-dev libpqxx-dev nasm libopenmpi-dev openmpi-bin openmpi-common
        ```
    - **macOS**:
        ```bash
        brew reinstall jq curl libomp protobuf openssl nasm pkgconf open-mpi libffi
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
└── src
    └── main.rs
```

The example program takes a number `n` as input and computes the SHA-256 hash `n` times. 

The `build.rs` file generates an `input.bin` file containing the value of `n` (e.g., 20). This file is used in `main.rs` as input to calculate the hash.

You can run the program on your native architecture with the following command:
```bash
cargo run
```
The output will be:
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

## Build

The next step is to build the program using the `cargo-zisk` command to generate an ELF file (RISC-V), which will be used later to generate the proof. Execute:

```bash
cargo-zisk build --release
```

This command builds the program using the `zkvm` target. The resulting `sha_hasher` ELF file (without extension) is generated in the `./target/riscv64ima-zisk-zkvm-elf/release` directory.

## Execute

Before generating a proof, you can test the program using the ZisK emulator to ensure its correctness. Specify the ELF file (using the `-e` or `--elf flag`) and the input file `input.bin` (using the `-i` or `--inputs` flag):

```bash
ziskemu -e target/riscv64ima-zisk-zkvm-elf/release/sha_hasher -i build/input.bin
```

The output will be:
```
98211882
bd13089b
6ccf1fca
81f7f0e4
abf6352a
0c39c9b1
1f142cac
233f1280
```

Alternatively, you can build and run the program with:

```bash
cargo-zisk run --release -i build/input.bin
```

## Prove

Before generating a proof, you need to generate the program setup files. Execute:

```bash
cargo-zisk rom-setup -e target/riscv64ima-zisk-zkvm-elf/release/sha_hasher
```

Once the program setup is complete, you can generate and verify a proof using the `cargo-zisk prove` command by providing the ELF file (with the `-e` or `--elf` flag) and the input file (with the `-i` or `--input` flag).

To generate and verify a proof for the previously built ELF and input files, execute:

```bash
cargo-zisk prove -e target/riscv64ima-zisk-zkvm-elf/release/sha_hasher -i build/input.bin -o proof -a -y
```

This command generates the proof in the `./proof` directory. If everything goes well, you will see a message similar to:

```
...
[INFO ] ProofMan:     ✓ Vadcop Final proof was verified
[INFO ]      stop <<< GENERATING_VADCOP_PROOF 91706ms
[INFO ] ProofMan: Proofs generated successfully
```

**Note**: You can use concurrent proof generation and GPU support to reduce proving time. For more details, refer to the [Writing Programs](./writing_programs.md) guide.

## Verify Proof

To verify a generated proof, use the following command:

```bash
cargo-zisk verify -p ./proof/proofs/vadcop_final_proof.json -u ./proof/publics.json
```