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
git clone -b develop https://github.com/0xPolygonHermez/zisk.git
git clone -b develop https://github.com/0xPolygonHermez/pil2-proofman.git
git clone -b develop https://github.com/0xPolygonHermez/pil2-proofman-js
```

## Compile a Verifiable Rust Program

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
ziskemu -i build/input.bin -x -e target/riscv64ima-zisk-zkvm-elf/release/hello_world
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
ziskemu -i build/input.bin -x -e target/riscv64ima-zisk-zkvm-elf/debug/hello_world
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

!!!!!! Download pil2-proofman to be able to compile the std
node --max-old-space-size=131072 --stack-size=1500 ../pil2-proofman-js/src/main_setup.js -a pil/zisk_pre_040.pilout -b build/build_pre_040 -t ../pil2-proofman/pil2-stark/build/bctree -i ./build/keccakf_fixed.bin
com es genera el fixed.bin???

cargo run --release --bin keccakf_fixed_gen
```bash
node --max-old-space-size=65536 ../pil2-compiler/src/pil.js pil/zisk.pil -I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines,precompiles -o pil/zisk.pilout
```

### Compile PILs with `std_mock` (for testing without `std`):
```bash
node ../pil2-compiler/src/pil.js pil/zisk.pil -I pil,../pil2-components/lib/std_mock/pil,state-machines -o pil/zisk.pilout
```

### Compile the PIl2 Stark C++ Library (run only once):
```bash
(cd ../pil2-proofman/pil2-stark && git submodule init && git submodule update && make clean && make -j starks_lib && make -j bctree)
```

### Generate PIL-Helpers Rust Code
Run this whenever the `.pilout` file changes:

```bash
(cd ../pil2-proofman; cargo run --bin proofman-cli pil-helpers --pilout ../zisk/pil/zisk.pilout --path ../zisk/pil/src/ -o)
```

### Generate Setup Data
Run this whenever the `.pilout` file changes:

```bash[]
node --max-old-space-size=131072 --stack-size=1500 ../pil2-proofman-js/src/main_setup.js -a pil/zisk.pilout -b build -t ../pil2-proofman/pil2-stark/build/bctree
```

### Compile Witness Computation library (`libzisk_witness.so`)
```bash
cargo build --release
```

> If you get a library not found error, set the path manually:
> ```bash
> export RUSTFLAGS="-L native={path to your pil2-stark folder}/pil2-stark/lib"
> ```

## Generate & Verify Proofs

Sample inputs are located in `zisk/emulator/benches/data`:
- `input_one_segment.bin`: single SHA
- `input_two_segments.bin`: 512 SHA
- `input.bin`: large number of SHA

### Verify Constraints Only
```bash
// Using input_one_segment.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input_one_segment.bin --proving-key ../zisk/build/provingKey)

// Using input_two_segments.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input_two_segments.bin --proving-key ../zisk/build/provingKey)`

// Using input.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input.bin --proving-key ../zisk/build/provingKey)`
```

### Generate a Proof

To generate the aggregated proofs, add `-a`

```bash
// Using input_one_segment.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli prove --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input_one_segment.bin --proving-key ../zisk/build/provingKey --output-dir ../zisk/proofs -a -v)

// Using input_two_segments.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli prove --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input_two_segments.bin --proving-key ../zisk/build/provingKey --output-dir ../zisk/proofs -a -v)

// Using input.bin
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli prove --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input.bin --proving-key ../zisk/build/provingKey --output-dir ../zisk/proofs -a -v)
```

### Distributed prove

Zisk can run proves using multiple processes in the same server or in multiple servers. To use zisk in distributed mode you need to have installed a mpi library. To use the distributed mode the compilation command is:

```bash
cargo-zisk build --release --features distributed
```

Then the execution command will be:

```bash
mpirun --bind-to none -np <number_processes> -x OMP_NUM_THREADS=<number_of_threads_per_process> target/release/cargo-zisk prove -e target/riscv64ima-zisk-zkvm-elf/release/sha_hasher -i build/input.bin -w $HOME/.zisk/bin/libzisk_witness.so -k $HOME/.zisk/provingKey -o proof -a -y
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
