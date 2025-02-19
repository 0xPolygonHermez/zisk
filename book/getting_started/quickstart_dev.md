# Quickstart

In this guide, we will walk you through the steps to create a simple Zisk project.

## Requirements

Before you begin, ensure that you have [Rust](https://www.rust-lang.org/tools/install) installed on your system.

Optional recommendations:

- Use the [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extension for VS Code to enhance your Rust development experience.
- Use the [PIL2 Highlight syntax code](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) for VS Code to highlight PIL2 code when writing it.

## Clone Zisk Repository

Run the following command to clone the Zisk repository:

```bash
git clone -b develop https://github.com/0xPolygonHermez/zisk.git
```

## Compile a Verifiable Rust Program

TODO: Addinstructions to compile cargo-zisk

### Setup
Install qemu:
`sudo apt-get install qemu-system`

### Create New Hello World Project
Create a new project using the Zisk toolchain:

```bash
cargo-zisk sdk new hello_world
cd hello_world
```

Edit file `build.rs` file to modify the `OUTPUT_DIR` variable to `build`:

```rust=3
use std::path::Path;

// Define constants for the directory and file names
const OUTPUT_DIR: &str = "build";
const FILE_NAME: &str = "input.bin";
```

### Compile and Run

- RISC-V mode:
```bash
cargo-zisk run --release
```

- Zisk mode:
```bash
cargo-zisk run --release --sim
```

- Ziskemu execution:
```bash
ziskemu -i build/input.bin -x -e target/riscv64ima-polygon-ziskos-elf/release/hello_world
```

### Updating the Toolchain
To update the Zisk toolchain:

```bash
ziskup
```

If `ziskup` fails, you can manually update `ziskemu`.

### Manual Ziskemu Update
```bash
cd zisk
git pull
cargo install --path emulator
cp ~/.cargo/bin/ziskemu ~/.zisk/bin/
```

Run the emulator with:

```bash
ziskemu -i build/input.bin -x -e target/riscv64ima-polygon-ziskos-elf/debug/hello_world
```

### Easy Input Update for 64-bit Values
To put `0x0100`, reverse hex sequence:
```bash
echo -en "\x00\x01\x00\x00\x00\x00\x00\x00" > input_two_segments.bin
```
To input `0x0234`:
```bash 
echo -en "\x34\x02\x00\x00\x00\x00\x00\x00" > input_two_segments.bin
```

## Setup

### Use an already generated setup

TODO: Add instructions to use an already generated setup

### Generate a new setup

Run the following commands to clone the necessary repositories to be able to generate a new setup:

```bash
git clone -b develop https://github.com/0xPolygonHermez/pil2-compiler.git
git clone -b develop https://github.com/0xPolygonHermez/pil2-proofman.git
git clone -b develop https://github.com/0xPolygonHermez/pil2-proofman-js
```

All following commands should be executed in the `zisk` folder.

#### Compile Zisk PIL

```bash
node --max-old-space-size=131072 {path to pil2-compiler folder}/src/pil.js pil/zisk.pil -I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines,precompiles -o pil/zisk.pilout
```

#### Generate Fixed Data

```bash
cargo run --release --bin keccakf_fixed_gen
mv precompiles/keccakf/src/keccakf_fixed.bin build
```

This command will generate the `keccakf_fixed.bin` file in the `build` folder.

#### Compile the PIl2 Stark C++ Library (run only once):

```bash
(cd {path to pil2-proofman folder}/pil2-stark && git submodule init && git submodule update && make clean && make -j starks_lib && make -j bctree)
```

### Generate Setup Data
Run this whenever the `.pilout` file changes:

```bash
node --max-old-space-size=65536 {path to pil2-proofman-js folder}/src/main_setup.js -a pil/zisk.pilout -b build -t {path to pil2-proofman folder}/pil2-stark/build/bctree -i ./build/keccakf_fixed.bin
```

### Optional. Generate PIL-Helpers Rust Code
Run this whenever the `.pilout` file changes:

```bash
(cd {path to pil2-proofman folder}; cargo run --bin proofman-cli pil-helpers --pilout {path to zisk folder}pil/zisk.pilout --path {path to zisk folder}/pil/src/ -o)
```

## Compile Zisk Witness Computation library (`libzisk_witness.so`)
```bash
cargo build --release
```

## Generate & Verify Proofs

Sample inputs are located in `zisk/emulator/benches/data`:
- `input_one_segment.bin`: single SHA
- `input_two_segments.bin`: 512 SHA
- `input.bin`: large number of SHA

### Verify Constraints Only
```bash
// Using input_one_segment.bin
cargo-zisk verify-constraints --witness-lib ./target/release/libzisk_witness.so --rom ./emulator/benches/data/my.elf -i ./emulator/benches/data/input_one_segment.bin --proving-key ./build/provingKey
```

### Generate a Proof

To generate the aggregated proofs, add `-a`

```bash
// Using input_one_segment.bin
cargo-zisk prove --witness-lib ./target/release/libzisk_witness.so --rom ./emulator/benches/data/my.elf -i ./emulator/benches/data/input_one_segment.bin --proving-key ./build/provingKey --output-dir ../zisk/proofs -a -v
```

### Verify the Proof
```bash
node ../pil2-proofman-js/src/main_verify -k ./build/provingKey -p ./proofs
```

### Verify the aggregated Proof
If the aggregation proofs are being generated, can be verified with the following command:

```bash
node ../pil2-proofman-js/src/main_verify -k ./build/provingKey/ -p ./proofs -t vadcop_final
```