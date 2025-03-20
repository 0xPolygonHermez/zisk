# Writing Programs

This document explains how to write or modify a Rust program for execution in ZisK.

## Setup

### Code changes

Writing a Rust program for ZisK is similar to writing a standard Rust program, with a few minor modifications. Follow these steps:

1. Modify `main.rs` file:

    Add the following code to mark the main function as the entry point for ZisK:

    ```rust
    #![no_main]
    ziskos::entrypoint!(main);
    ```

2. Modify `Cargo.toml` file:

    Add the `ziskos` crate as a dependency:

    ```toml
    [dependencies]
    ziskos = { git = "https://github.com/0xPolygonHermez/zisk.git" }
    ```

Let's show these changes using the example program from the [Quickstart](./quickstart.md) section.

### Example program    

`main.rs`:
```rust
// This example program takes a number `n` as input and computes the SHA-256 hash `n` times sequentially.

// Mark the main function as the entry point for ZisK
#![no_main]
ziskos::entrypoint!(main);

use sha2::{Digest, Sha256};
use std::convert::TryInto;
use ziskos::{read_input, set_output};
use byteorder::ByteOrder;

fn main() {
    // Read the input data as a byte array from ziskos
    let input: Vec<u8> = read_input();

    // Get the 'n' value converting the input byte array into a u64 value
    let n: u64 = u64::from_le_bytes(input.try_into().unwrap());

    let mut hash = [0u8; 32];

    // Compute SHA-256 hashing 'n' times
    for _ in 0..n {
        let mut hasher = Sha256::new();
        hasher.update(hash);
        let digest = &hasher.finalize();
        hash = Into::<[u8; 32]>::into(*digest);
    }

    // Split 'hash' value into chunks of 32 bits and write them to ziskos output
    for i in 0..8 {
        let val = byteorder::BigEndian::read_u32(&mut hash[i * 4..i * 4 + 4]);
        set_output(i, val);
    }
}
```

`Cargo.toml`:
```toml
[package]
name = "sha_hasher"
version = "0.1.0"
edition = "2021"
default-run = "sha_hasher"

[dependencies]
byteorder = "1.5.0"
sha2 = "0.10.8"
ziskos = { git = "https://github.com/0xPolygonHermez/zisk.git" }
```

### Input/Output Data
To provide input data for ZisK, you need to write that data in a binary file (e.g., `input.bin`).

If your program requires complex input data, consider using a serialization mechanism (like [`bincode`](https://crates.io/crates/bincode) crate) to store it in `input.bin` file.

In your program, use the `ziskos::read_input()` function to retrieve the input data from the `input.bin` file:

```rust
// Read the input data as a byte array from ziskos
let input: Vec<u8> = read_input();
```    

To write public output data, use the `ziskos::set_output()` function. Since the function accepts `u32` values, split the output data into 32-bit chunks if necessary and increase the `id` parameter of the function in each call:

```rust
// Split 'hash' value into chunks of 32 bits and write them to ziskos output
for i in 0..8 {
    let val = byteorder::BigEndian::read_u32(&mut hash[i * 4..i * 4 + 4]);
    set_output(i, val);
}
```    

## Build

Before compiling your program for ZisK, you can test it on the native architecture just like any regular Rust program using the `cargo` command.

Once your program is ready to run on ZisK, compile it into an ELF file (RISC-V architecture), using the `cargo-zisk` CLI tool:

```bash
cargo-zisk build
```

This command compiles the program using the `riscv64ima_polygon_ziskos` target. The resulting `sha_hasher` ELF file (without extension) is generated in the `./target/riscv64ima-polygon-ziskos-elf/debug` directory.

For production, compile the ELF file with the `--release` flag, similar to how you compile Rust projects:

```bash
cargo-zisk build --release
```

In this case, the `sha_hasher` ELF file will be generated in the `./target/riscv64ima-polygon-ziskos-elf/release` directory.

## Execute

You can test your compiled program using the ZisK emulator (`ziskemu`) before generating a proof. Use the `-e` (`--elf`) flag to specify the location of the ELF file and the `-i` (`--inputs`) flag to specify the location of the input file:

```bash
cargo-zisk build --release
ziskemu -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher -i build/input.bin
```

Alternatively, you can build and execute the program in the ZisK emulator with a single command:

```bash
cargo-zisk run --release -i build/input.bin
```

If the program requires a large number of ZisK steps, you might encounter the following error:
```
Error during emulation: EmulationNoCompleted
Error: Error executing Run command
```

To resolve this, you can increase the number of execution steps using the `-n` (`--max-steps`) flag. For example:
```bash
ziskemu -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher -i build/input.bin -n 10000000000
```

## Metrics and Statistics

### Performance Metrics
You can get performance metrics related to the program execution in ZisK using the `-m` (`--log-metrics`) flag in the `cargo-zisk run` command or in `ziskemu` tool:

```bash
cargo-zisk run --release -i build/input.bin -m
```

Or

```bash
ziskemu -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher -i build/input.bin -m
```

The output will include details such as execution time, throughput, and clock cycles per step:
```
process_rom() steps=85309 duration=0.0009 tp=89.8565 Msteps/s freq=3051.0000 33.9542 clocks/step
98211882
bd13089b
6ccf1fca
...
```

### Execution Statistics
You can get statistics related to the program execution in Zisk using the `-x` (`--stats`) flag in the `cargo-zisk run` command or in `ziskemu` tool:

```bash
cargo-zisk run --release -i build/input.bin -x
```

Or

```bash
ziskemu -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher -i build/input.bin -x
```

The output will include details such as cost definitions, total cost, register reads/writes, opcode statistics, etc:
```
Cost definitions:
    AREA_PER_SEC: 1000000 steps
    COST_MEMA_R1: 0.00002 sec
    COST_MEMA_R2: 0.00004 sec
    COST_MEMA_W1: 0.00004 sec
    COST_MEMA_W2: 0.00008 sec
    COST_USUAL: 0.000008 sec
    COST_STEP: 0.00005 sec

Total Cost: 12.81 sec
    Main Cost: 4.27 sec 85308 steps
    Mem Cost: 2.22 sec 222052 steps
    Mem Align: 0.05 sec 2701 steps
    Opcodes: 6.24 sec 1270 steps (81182 ops)
    Usual: 0.03 sec 4127 steps
    Memory: 135563 a reads + 1625 na1 reads + 10 na2 reads + 84328 a writes + 524 na1 writes + 2 na2 writes = 137198 reads + 84854 writes = 222052 r/w

Opcodes:
    flag: 0.00 sec (0 steps/op) (89 ops)
    copyb: 0.00 sec (0 steps/op) (10568 ops)
    add: 1.12 sec (77 steps/op) (14569 ops)
    ltu: 0.01 sec (77 steps/op) (101 ops)
    ...
    xor: 1.06 sec (77 steps/op) (13774 ops)
    signextend_b: 0.03 sec (109 steps/op) (320 ops)
    signextend_w: 0.03 sec (109 steps/op) (320 ops)

98211882
bd13089b
6ccf1fca
...
```

## Prove

### Verify Constraints
Before to generate a proof (that can take some time) you can verify that all the constraints are satisfied:

```bash
cargo-zisk verify-constraints -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher -i build/input.bin -w $HOME/.zisk/bin/libzisk_witness.so -k $HOME/.zisk/provingKey
```

If everything is correct, you will see an output similar to:

```
[INFO ] GlCstVfy: --> Checking global constraints
[INFO ] CstrVrfy: ··· ✓ All global constraints were successfully verified
[INFO ] CstrVrfy: ··· ✓ All constraints were verified
```

### Generate Proof

To generate a proof, run following command:

```bash
cargo-zisk prove -e target/riscv64ima-polygon-ziskos-elf/release/sha_hasher -i build/input.bin -w $HOME/.zisk/bin/libzisk_witness.so -k $HOME/.zisk/provingKey -o proof -a -y
```
In this command:

* `-e` (`--elf`) flag is used to specify the ELF file localtion.
* `-i` (`--inputs`) flag is used specify the input file location.
* `-w` (`--witness`) and `-k` (`--proving-key`) flags are used to specify the location of the witness library and proving key files required for proof generation; they are optional, set by default to the paths found in the `$HOME/.zisk` installation folder.
* `-o` (`--output`) flag determines the output directory (in this example `proof`).
* `-a` (`--aggregation`) flag indicates that a final aggregated proof (containing all generated sub-proofs) should be produced.
* `-y` (`--verify-proofs`) flag instructs the tool to verify the proof immediately after it is generated (verification can also be performed later using the `cargo-zisk verify` command).

If the process is successful, you should see a message similar to:

```
...
[INFO ] ProofMan:     ✓ Vadcop Final proof was verified
[INFO ]      stop <<< GENERATING_VADCOP_PROOF 91706ms
[INFO ] ProofMan: Proofs generated successfully
```

### Verify Proof

To verify a generated proof, use the following command:

```bash
cargo-zisk verify -p ./proof/proofs/vadcop_final_proof.json -u ./proof/publics.json -s $HOME/.zisk/provingKey/zisk/vadcop_final/vadcop_final.starkinfo.json -e $HOME/.zisk/provingKey/zisk/vadcop_final/vadcop_final.verifier.bin -k $HOME/.zisk/provingKey/zisk/vadcop_final/vadcop_final.verkey.json
```

In this command:

* `-p` (`--proof`) flag specifies the final proof file generated with cargo-zisk prove.
* `-u` (`--public_inputs`) flag provides the path to the public inputs associated with the proof.
* The remaining flags specify the files required for verification; they are optional, set by default to the files found in the `$HOME/.zisk` directory.


