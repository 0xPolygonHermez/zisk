# Quickstart

In this section, we will show you how to create a simple program using ZisK.

## Create Project

```bash
git clone https://github.com/0xPolygonHermez/zisk
git clone -b develop https://github.com/0xPolygonHermez/pil2-compiler.git
git clone -b develop https://github.com/0xPolygonHermez/pil2-proofman.git
git clone --recursive -b develop  https://github.com/0xPolygonHermez/pil2-stark.git
git clone -b feature/setup https://github.com/0xPolygonHermez/pil2-proofman-js
```

The first step is to create a new project using the `cargo-zisk sdk new <name>` command. This command will create a new folder in your current directory.

```bash
cargo-zisk sdk new hello_world
cd hello_world
```

This will create a new project with the following structure:

```
.
├── build.rs
├── Cargo.lock
├── Cargo.toml
├── .gitignore
└── src
    └── main.rs

2 directories, 8 files
```

For running the program in the native architecture:
```
$ cargo run --target x86_64-unknown-linux-gnu
     Running `target/x86_64-unknown-linux-gnu/debug/sha_hasher`
n:10000 [82, 229, 228, 9, 207, 11, 252, 118, 235, 27, 13, 44, 75, 164, 54, 106, 253, 126, 193, 14, 54, 32, 188, 119, 81, 120, 47, 45, 222, 206, 161, 159]
```

## Run on ZisK emulator

```bash
cargo-zisk run --release
   Compiling sha_hasher v0.1.0 (/home/edu/hello_world)
    Finished `release` profile [optimized] target(s) in 0.20s
     Running `ziskemu -i build/input.bin -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher`
n:10000 [82, 229, 228, 9, 207, 11, 252, 118, 235, 27, 13, 44, 75, 164, 54, 106, 253, 126, 193, 14, 54, 32, 188, 119, 81, 120, 47, 45, 222, 206, 161, 159]
```
or  
```bash
cargo-zisk build --release
ziskemu -i build/input.bin -x -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher
```
### metrics
```bash
cargo-zisk run --release -m
   Compiling sha_hasher v0.1.0 (/home/edu/hello_world)
    Finished `release` profile [optimized] target(s) in 0.20s
     Running `ziskemu -i build/input.bin -m -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher`
n:10000 [82, 229, 228, 9, 207, 11, 252, 118, 235, 27, 13, 44, 75, 164, 54, 106, 253, 126, 193, 14, 54, 32, 188, 119, 81, 120, 47, 45, 222, 206, 161, 159]
process_rom() steps=42454508 duration=0.7520 tp=56.4588 Msteps/s freq=2874.0000 50.9043 clocks/step
```

### stats
```bash
cargo-zisk run --release --stats
   Compiling sha_hasher v0.1.0 (/home/edu/hello_world)
    Finished `release` profile [optimized] target(s) in 0.20s
     Running `ziskemu -i build/input.bin -x -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher`
n:10000 [82, 229, 228, 9, 207, 11, 252, 118, 235, 27, 13, 44, 75, 164, 54, 106, 253, 126, 193, 14, 54, 32, 188, 119, 81, 120, 47, 45, 222, 206, 161, 159]
Cost definitions:
    AREA_PER_SEC: 1000000 steps
    COST_MEMA_R1: 0.00002 sec
    COST_MEMA_R2: 0.00004 sec
    COST_MEMA_W1: 0.00004 sec
    COST_MEMA_W2: 0.00008 sec
    COST_USUAL: 0.000008 sec
    COST_STEP: 0.00005 sec

Total Cost: 6392.55 sec
    Main Cost: 2122.73 sec 42454507 steps
    Mem Cost: 1107.83 sec 110782774 steps
    Mem Align: 21.22 sec 1061034 steps
    Opcodes: 3125.94 sec 1432 steps (40600544 ops)
    Usual: 14.83 sec 1853964 steps

Opcodes:
    flag: 0.00 sec (0 steps/op) (40583 ops)
    copyb: 0.00 sec (0 steps/op) (5266028 ops)
    add: 553.92 sec (77 steps/op) (7193707 ops)
    sub: 0.00 sec (77 steps/op) (11 ops)
    ltu: 6.95 sec (77 steps/op) (90237 ops)
    eq: 1.54 sec (77 steps/op) (19953 ops)
    sll: 620.18 sec (109 steps/op) (5689699 ops)
    srl: 8.73 sec (109 steps/op) (80061 ops)
    add_w: 0.00 sec (77 steps/op) (55 ops)
    sub_w: 0.77 sec (77 steps/op) (10005 ops)
    srl_w: 718.31 sec (109 steps/op) (6589961 ops)
    and: 168.70 sec (77 steps/op) (2190887 ops)
    or: 471.96 sec (77 steps/op) (6129357 ops)
    xor: 531.30 sec (77 steps/op) (6899979 ops)
    signextend_b: 17.44 sec (109 steps/op) (160000 ops)
    signextend_w: 26.16 sec (109 steps/op) (240000 ops)
    mul: 0.00 sec (97 steps/op) (20 ops)
    muluh: 0.00 sec (97 steps/op) (1 ops)
```

## Update zisk toolchain to latest version

```bash
ziskup
```

## Prepare Your Setup

All following commands should be executed in the `zisk` folder.

### Compile Zisk PIL

```bash
cd ../pil2-compiler
npm i
cd ../zisk
node --max-old-space-size=65536 ../pil2-compiler/src/pil.js pil/zisk.pil -I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines -o pil/zisk.pilout
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
(cd ../pil2-proofman-js && npm i)
node --max-old-space-size=65536 ../pil2-proofman-js/src/main_setup.js -a pil/zisk.pilout -b build -t ../pil2-stark/build/bctree
```

### Compile Witness Computation library (`libzisk_witness.so`)
```bash
RUSTFLAGS="-L native=/home/{path to your pil2-stark folder}/lib" cargo build --release
```

### Generate a Proof
```bash
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli prove --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../hello_world/target/riscv64ima-polygon-ziskos-elf/release/sha_hasher.elf -i ../hello_world/build/input.bin --proving-key ../zisk/build/provingKey --output-dir ../zisk/proofs -d -v)
```

### Verify the Proof
```bash
(node ../pil2-proofman-js/src/main_verify -k ./build/provingKey -p ./proofs)
```

### Verify Constraints Only
```bash
(cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../hello_world/target/riscv64ima-polygon-ziskos-elf/release/sha_hasher.elf -i ../hello_world/build/input.bin --proving-key ../zisk/build/provingKey)
```