# Quickstart

In this guide, you will learn how to create and run a simple program using ZisK.

## Create a Project

The first step is to generate a new example project using the `cargo-zisk sdk new <name>` command. This command creates a new folder named `<name>` in your current directory. For example:
```bash
cargo-zisk sdk new sha_hasher
cd sha_hasher
```

This will create a project with the following structure:

```
.
├── build.rs
├── Cargo.lock
├── Cargo.toml
├── .gitignore
└── src
    └── main.rs
```

The example program takes a number `n` as input and computes the SHA-256 hash `n` times. The `build.rs` file generates an `input.bin` file containing the value of `n` (e.g., 20). This file is used in `main.rs` as input for ZisK to calculate the hash.

You can run the program on your native architecture with the following command:
```bash
$ cargo run
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

## Run on ZisK Emulator
Before generating a proof, you can test the program using the ZisK emulator to ensure correctness:
```bash
cargo-zisk run --release
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

Alternatively, you can build the program to generate an ELF file (RISC-V) and then use `ziskemu` tool to execute the ELF file (`-e`, `--elf` flag) with `input.bin` as the ZisK input (`-i`, `--inputs` flag):

```bash
cargo-zisk build --release
ziskemu -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher -i build/input.bin
```

## Prove (WIP)

### Setup

```bash
git clone https://github.com/0xPolygonHermez/zisk
git clone -b develop https://github.com/0xPolygonHermez/pil2-compiler.git
git clone -b 0.0.16 https://github.com/0xPolygonHermez/pil2-proofman.git
git clone -b 0.0.16 https://github.com/0xPolygonHermez/pil2-proofman-js
```

All following commands should be executed in the `zisk` folder.
```bash
cd zisk
```

### Compile

```bash
(cd ../pil2-compiler && npm i && cd ../zisk && node --max-old-space-size=65536 ../pil2-compiler/src/pil.js pil/zisk.pil -I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines -o pil/zisk.pilout)
```

#### Compile the PIl2 Stark C++ Library (run only once):
```bash
(cd ../pil2-proofman/pil2-stark && git submodule init && git submodule update && make clean && make -j starks_lib && make -j bctree) && export RUSTFLAGS=$RUSTFLAGS" -L native=$PWD/../pil2-proofman/pil2-stark/lib"
```

#### Generate PIL-Helpers Rust Code
Run this whenever the `.pilout` file changes:

```bash
(cd ../pil2-proofman; cargo run --bin proofman-cli pil-helpers --pilout ../zisk/pil/zisk.pilout --path ../zisk/pil/src/ -o)
```

#### Generate Setup Data
Run this whenever the `.pilout` file changes:

```bash
(cd ../pil2-proofman-js && npm i)
node --max-old-space-size=65536 ../pil2-proofman-js/src/main_setup.js -a pil/zisk.pilout -b build -t ../pil2-proofman/pil2-stark/build/bctree -r
```

#### Compile Witness Computation library (`libzisk_witness.so`)
```bash
cargo build --release
```

#### Generate a Proof
To generate the proof, the following command needs to be run.

```bash
(cd ../pil2-proofman; cargo run --release --bin proofman-cli prove --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../hello_world/target/riscv64ima-polygon-ziskos-elf/release/sha_hasher -i ../hello_world/build/input.bin --proving-key ../zisk/build/provingKey --output-dir ../zisk/proofs -v -a)
```

#### Verify the Proof
```bash
node ../pil2-proofman-js/src/main_verify -k build/provingKey/ -p proofs -t vadcop_final
```
