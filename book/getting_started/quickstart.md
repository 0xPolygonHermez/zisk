# Quickstart

In this section, we will show you how to create a simple program using ZisK.

## Create Project

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
n:20 [152, 33, 24, 130, 189, 19, 8, 155, 108, 207, 31, 202, 129, 247, 240, 228, 171, 246, 53, 42, 12, 57, 201, 177, 31, 20, 44, 172, 35, 63, 18, 128]
```

## Run on ZisK emulator

```bash
cargo-zisk run --release
   Compiling sha_hasher v0.1.0 (/home/edu/hello_world)
    Finished `release` profile [optimized] target(s) in 0.20s
     Running `ziskemu -i build/input.bin -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher`
n:20 [152, 33, 24, 130, 189, 19, 8, 155, 108, 207, 31, 202, 129, 247, 240, 228, 171, 246, 53, 42, 12, 57, 201, 177, 31, 20, 44, 172, 35, 63, 18, 128]
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
n:20 [152, 33, 24, 130, 189, 19, 8, 155, 108, 207, 31, 202, 129, 247, 240, 228, 171, 246, 53, 42, 12, 57, 201, 177, 31, 20, 44, 172, 35, 63, 18, 128]
process_rom() steps=99288 duration=0.0024 tp=40.9284 Msteps/s freq=2892.0000 70.6600 clocks/step
```

### stats
```bash
cargo-zisk run --release --stats
   Compiling sha_hasher v0.1.0 (/home/edu/hello_world)
    Finished `release` profile [optimized] target(s) in 0.20s
     Running `ziskemu -i build/input.bin -x -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher`
n:20 [152, 33, 24, 130, 189, 19, 8, 155, 108, 207, 31, 202, 129, 247, 240, 228, 171, 246, 53, 42, 12, 57, 201, 177, 31, 20, 44, 172, 35, 63, 18, 128]
Cost definitions:
    AREA_PER_SEC: 1000000 steps
    COST_MEMA_R1: 0.00002 sec
    COST_MEMA_R2: 0.00004 sec
    COST_MEMA_W1: 0.00004 sec
    COST_MEMA_W2: 0.00008 sec
    COST_USUAL: 0.000008 sec
    COST_STEP: 0.00005 sec

Total Cost: 14.25 sec
    Main Cost: 4.96 sec 99287 steps
    Mem Cost: 2.54 sec 254054 steps
    Mem Align: 0.06 sec 3130 steps
    Opcodes: 6.63 sec 1335 steps (92652 ops)
    Usual: 0.05 sec 6636 steps
    Memory: 155262 a reads + 1846 na1 reads + 0 na2 reads + 96304 a writes + 642 na1 writes + 0 na2 writes = 157108 reads + 96946 writes = 254054 r/w
    Registy: 147515 reads + 90588 writes = 238103 r/w

Opcodes:
    flag: 0.00 sec (0 steps/op) (660 ops)
    copyb: 0.00 sec (0 steps/op) (16521 ops)
    add: 1.39 sec (77 steps/op) (18059 ops)
    sub: 0.00 sec (77 steps/op) (10 ops)
    ltu: 0.03 sec (77 steps/op) (412 ops)
    eq: 0.02 sec (77 steps/op) (224 ops)
    sll: 1.24 sec (109 steps/op) (11360 ops)
    srl: 0.02 sec (109 steps/op) (216 ops)
    add_w: 0.00 sec (77 steps/op) (52 ops)
    sub_w: 0.00 sec (77 steps/op) (24 ops)
    srl_w: 1.43 sec (109 steps/op) (13141 ops)
    and: 0.40 sec (77 steps/op) (5168 ops)
    or: 0.94 sec (77 steps/op) (12209 ops)
    xor: 1.06 sec (77 steps/op) (13779 ops)
    signextend_b: 0.03 sec (109 steps/op) (320 ops)
    signextend_w: 0.05 sec (109 steps/op) (480 ops)
    mul: 0.00 sec (97 steps/op) (17 ops)
```

## Update zisk toolchain to latest version

```bash
ziskup
```

## Prepare Your Setup

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

### Compile Zisk PIL

```bash
(cd ../pil2-compiler && npm i && cd ../zisk && node --max-old-space-size=65536 ../pil2-compiler/src/pil.js pil/zisk.pil -I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines -o pil/zisk.pilout)
```

### Compile the PIl2 Stark C++ Library (run only once):
```bash
(cd ../pil2-proofman/pil2-stark && git submodule init && git submodule update && make clean && make -j starks_lib && make -j bctree) && export RUSTFLAGS=$RUSTFLAGS" -L native=$PWD/../pil2-proofman/pil2-stark/lib"
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
node --max-old-space-size=65536 ../pil2-proofman-js/src/main_setup.js -a pil/zisk.pilout -b build -t ../pil2-proofman/pil2-stark/build/bctree -r
```

### Compile Witness Computation library (`libzisk_witness.so`)
```bash
cargo build --release
```

### Generate a Proof
To generate the proof, the following command needs to be run.

```bash
(cd ../pil2-proofman; cargo run --release --bin proofman-cli prove --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../hello_world/target/riscv64ima-polygon-ziskos-elf/release/sha_hasher -i ../hello_world/build/input.bin --proving-key ../zisk/build/provingKey --output-dir ../zisk/proofs -v -a)
```

### Verify the Proof
```bash
node ../pil2-proofman-js/src/main_verify -k build/provingKey/ -p proofs -t vadcop_final
```
