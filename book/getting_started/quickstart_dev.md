# Quickstart

In this guide, we will walk you through the steps to create a simple Zisk project.

## Requirements

Before you begin, ensure that you have [Rust](https://www.rust-lang.org/tools/install) installed on your system.

Optional recommendations:

- Use the [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extension for VS Code to enhance your Rust development experience.
- Use the [PIL2 Highlight syntax code](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) for VS Code to highlight PIL2 code when writing it.

## Clone Repositories

Run the following commands to clone the necessary repositories:

```bash
git clone -b develop https://github.com/0xPolygonHermez/pil2-compiler.git
git clone -b develop https://${ZISK_TOKEN}@github.com/0xPolygonHermez/zisk.git
git clone -b develop https://github.com/0xPolygonHermez/pil2-proofman.git
git clone --recursive -b develop  https://github.com/0xPolygonHermez/pil2-stark.git
git clone -b feature/setup https://github.com/0xPolygonHermez/pil2-proofman-js
```

## Compile a Verifiable Rust Program

### Setup
Install qemu:
`sudo apt-get install qemu-system`


Set up tokens to access repositories:

```
export GITHUB_ACCESS_TOKEN=....
export ZISK_TOKEN=....
```

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

## Prepare Your Setup

All following commands should be executed in the `zisk` folder.

### Compile Zisk PIL

```bash
node --max-old-space-size=65536 ../pil2-compiler/src/pil.js pil/zisk.pil -I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines -o pil/zisk.pilout
```

### Compile PILs with `std_mock` (for testing without `std`):
```bash
node ../pil2-compiler/src/pil.js pil/zisk.pil -I pil,../pil2-components/lib/std_mock/pil,state-machines -o pil/zisk.pilout
```

### Compile the PIl2 Stark C++ Library (run only once):
```bash
(cd ../pil2-stark && git submodule init && git submodule update && make clean && make -j starks_lib && make -j bctree)
```

### Generate PIL-Helpers Rust Code
Run this whenever the `.pilout` file changes:

```bash
(cd ../pil2-proofman; cargo run --bin proofman-cli pil-helpers --pilout ../zisk/pil/zisk.pilout --path ../zisk/pil/src/ -o)
```

### Generate Setup Data
Run this whenever the `.pilout` file changes:

```bash
node --max-old-space-size=65536 ../pil2-proofman-js/src/main_setup.js -a pil/zisk.pilout -b build -t ../pil2-stark/build/bctree
```

### Compile Witness Computation library (`libzisk_witness.so`)
```bash
cargo build --release
```

> If you get a library not found error, set the path manually:
> ```bash
> export RUSTFLAGS="-L native=/home/{path to your pil2-stark folder}/lib"
> ```

## Generate & Verify Proofs

Sample inputs are located in `zisk/emulator/benches/data`:
- `input_one_segment.bin`: single SHA
- `input_two_segments.bin`: 512 SHA
- `input.bin`: large number of SHA

### Generate a Proof

```bash
// Using input_one_segment.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli prove --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input_one_segment.bin --proving-key ../zisk/build/provingKey --output-dir ../zisk/proofs -d)

// Using input_two_segments.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli prove --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input_two_segments.bin --proving-key ../zisk/build/provingKey --output-dir ../zisk/proofs -d)`

// Using input.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli prove --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input.bin --proving-key ../zisk/build/provingKey --output-dir ../zisk/proofs -d)`
```

### Verify the Proof
```bash
node ../pil2-proofman-js/src/main_verify -k ./build/provingKey -p ./proofs
```

### Verify Constraints Only
```bash
// Using input_one_segment.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input_one_segment.bin --proving-key ../zisk/build/provingKey)

// Using input_two_segments.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input_two_segments.bin --proving-key ../zisk/build/provingKey)`

// Using input.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input.bin --proving-key ../zisk/build/provingKey)`
```
