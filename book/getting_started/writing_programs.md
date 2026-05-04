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

use alloy_sol_types::SolValue;
use common::Output;
use sha2::{Digest, Sha256};

fn main() {
    // Read the input data
    let n: u32 = ziskos::io::read();

    let mut hash = [0u8; 32];

    // Compute SHA-256 hashing 'n' times
    for _ in 0..n {
        let mut hasher = Sha256::new();
        hasher.update(hash);
        let digest = &hasher.finalize();
        hash = Into::<[u8; 32]>::into(*digest);
    }

    let output = Output {
        hash: hash.into(),
        iterations: n,
        magic_number: 0xDEADBEEF,
    };

    println!("Computed hash: {:02x?}", output.hash);
    println!("Iterations: {}", output.iterations);

    let bytes = output.abi_encode();

    println!("Bytes to commit: {:?}", bytes);

    // Write raw ABI-encoded bytes directly (no bincode serialization)
    ziskos::io::commit_slice(&bytes);
}
```

`Cargo.toml`:
```toml
[package]
name = "guest"
version = "0.1.0"
edition = "2024"

[dependencies]
byteorder = "1.5.0"
sha2 = "0.10.8"
serde = { version = "1.0", default-features = false, features = ["derive"] }
ziskos = { workspace = true }
alloy-sol-types = "1.5.7"
common = { path = "../common" }
```

### Input/Output Data

To read input data in your ZisK program, use the `ziskos::io::read()` function, which deserializes data from the input:

```rust
// Read a u32 value from input
let n: u32 = ziskos::io::read();
```

You can also read custom types that implement the `Deserialize` trait:

```rust
// Read a custom struct from input
let my_data: MyStruct = ziskos::io::read();
```

To write public output data, use the `ziskos::io::commit_slice()` function, which commits a slice to the output:

```rust
    let bytes = output.abi_encode();

    println!("Bytes to commit: {:?}", bytes);

    // Write raw ABI-encoded bytes directly (no bincode serialization)
    ziskos::io::commit_slice(&bytes);
```

You can also use `commit()` function to output any type that implements the `Serialize` trait. The data will be serialized and made available as public outputs that can be verified by anyone checking the proof.

## Build

Before compiling your program for ZisK, you can test it on the native architecture just like any regular Rust program using the `cargo` command.

Once your program is ready to run on ZisK, compile it into an ELF file (RISC-V architecture), using the `cargo-zisk` CLI tool from the guest project folder:

```bash
cargo-zisk build
```

This command compiles the program using the `zisk` target. The resulting `guest` ELF file (without extension) is generated in the `./target/elf/riscv64ima-zisk-zkvm-elf/debug` directory.

For production, compile the ELF file with the `--release` flag, similar to how you compile Rust projects:

```bash
cargo-zisk build --release
```

In this case, the `guest` ELF file will be generated in the `./target/elf/riscv64ima-zisk-zkvm-elf/release` directory.

## Execute

You can test your compiled program using the emulator before generating a proof. Use the `-i` (`--inputs`) flag to specify the location of the input file:

```bash
cargo-zisk run --release -i ../host/tmp/input.bin
```

If the program requires a large number of ZisK steps, you might encounter the following error:
```
Error during emulation: EmulationNoCompleted
Error: Error executing Run command
```

To resolve this, use ziskemu directly and increase the number of execution steps using the `-n` (`--max-steps`) flag. For example:
```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i ../host/tmp/input.bin -n 10000000000
```

## Metrics and Statistics

### Performance Metrics
You can get performance metrics related to the program execution in ZisK using the `-m` (`--log-metrics`) flag in `ziskemu` tool:


```bash
ziskemu -e target/elf/riscv64ima-zisk-zkvm-elf/release/guest -i ../host/tmp/input.bin -m
```

The output will include details such as execution time, throughput, and clock cycles per step:
```
process_rom() steps=4450270 duration=0.0436 tp=102.0505 Msteps/s freq=3504.0000 34.3359 clocks/step
...
```

### Execution Statistics
You can get statistics related to the program execution in Zisk using the `-p` (`--profiling`) flag with `summary` in `cargo-zisk`:


```bash
cargo-zisk run --release -i ../host/tmp/input.bin -p summary
```

The output will include details such as cost definitions, total cost, opcode statistics, etc:

```
R╔══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
║  ◆ REPORT SUMMARY                                                                                                    ║
╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣
║  STEPS                                                                                                    4,450,270  ║
║  COST                                                                                                   787,338,404  ║
║  RAM                                                                                            0.00 MB / 507.75 MB  ║
╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
╔══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
║  ◆ COST DISTRIBUTION SUMMARY                                                                                         ║
╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣
║  CATEGORY                                                                                               COST      %  ║
║  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  ║
║  Base         █████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░     293,601,280  37.3%  ║
║  Main         ██████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░     302,618,360  38.4%  ║
║  Opcodes      █████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░     174,799,164  22.2%  ║
║  Precompiles  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░         234,155   0.0%  ║
║  Memory       ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░      16,085,445   2.0%  ║
║  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  ║
║  Total                                                                                           787,338,404 100.0%  ║
╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
╔══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
║  ◆ COST DISTRIBUTION BY OPCODE                                            ║  ◆ OPS vs FROPS                          ║
╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣
║  OPCODE                                                      COST      %  ║      OPS + FROPS           FROPS      %  ║
║  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  ║  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  ║
║  xor                      █░░░░░░░░░░░░░░░░░░░░░░      41,398,920   5.3%  ║       42,240,480         841,560   2.0%  ║
║  or                       █░░░░░░░░░░░░░░░░░░░░░░      36,646,620   4.7%  ║       38,881,560       2,234,940   5.7%  ║
║  srl_w                    █░░░░░░░░░░░░░░░░░░░░░░      34,606,615   4.4%  ║       36,040,000       1,433,385   4.0%  ║
║  sll                      █░░░░░░░░░░░░░░░░░░░░░░      30,019,783   3.8%  ║       34,007,662       3,987,879  11.7%  ║
║  add                      ░░░░░░░░░░░░░░░░░░░░░░░      16,846,475   2.1%  ║       16,998,100         151,625   0.9%  ║
║  and                      ░░░░░░░░░░░░░░░░░░░░░░░      12,917,580   1.6%  ║       13,456,080         538,500   4.0%  ║
║  signextend_w             ░░░░░░░░░░░░░░░░░░░░░░░         849,590   0.1%  ║          849,590               0   0.0%  ║
║  signextend_b             ░░░░░░░░░░░░░░░░░░░░░░░         848,053   0.1%  ║          848,053               0   0.0%  ║
║  srl                      ░░░░░░░░░░░░░░░░░░░░░░░         429,883   0.1%  ║          439,953          10,070   2.3%  ║
║  dma_xmemset              ░░░░░░░░░░░░░░░░░░░░░░░         200,496   0.0%  ║                                          ║
║  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  ║  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  ║
║  Total                                                175,033,319  22.2%  ║      184,735,683       9,702,364   5.3%  ║
╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
╔══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
║  ◆ TOP COST FUNCTIONS                                                                                                ║
╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣
║   0 sha2::sha256::compress256                                           ████████████░░░░░░░░     473,976,966  60.2%  ║
║   1 std::io::stdio::_print                                              ░░░░░░░░░░░░░░░░░░░░       4,290,957   0.5%  ║
║   2 core::fmt::write                                                    ░░░░░░░░░░░░░░░░░░░░       4,258,155   0.5%  ║
║   3 <alloc::vec::Vec<u8> as core::fmt::Debug>::fmt                      ░░░░░░░░░░░░░░░░░░░░       3,852,860   0.5%  ║
║   4 <core::fmt::builders::DebugSet>::entry                              ░░░░░░░░░░░░░░░░░░░░       3,746,448   0.5%  ║
║   5 <std::..::Adapter<…> as core::fmt::Write>::write_str                ░░░░░░░░░░░░░░░░░░░░       2,549,696   0.3%  ║
║   6 <&u8 as core::fmt::Debug>::fmt                                      ░░░░░░░░░░░░░░░░░░░░       2,193,178   0.3%  ║
║   7 <u8 as core::fmt::Display>::fmt                                     ░░░░░░░░░░░░░░░░░░░░       2,105,434   0.3%  ║
║   8 <std::..::LineWriterShim<…> as std::io::Write>::write_all           ░░░░░░░░░░░░░░░░░░░░       1,953,802   0.2%  ║
║   9 <core::fmt::Formatter>::pad_integral                                ░░░░░░░░░░░░░░░░░░░░       1,820,586   0.2%  ║
║  10 core::slice::memchr::memrchr                                        ░░░░░░░░░░░░░░░░░░░░         843,066   0.1%  ║
║  11 memset                                                              ░░░░░░░░░░░░░░░░░░░░         499,356   0.1%  ║
║  12 <std::io::buffered::bufwriter::BufWriter<…>>::flush_buf             ░░░░░░░░░░░░░░░░░░░░         202,008   0.0%  ║
║  13 sys_write                                                           ░░░░░░░░░░░░░░░░░░░░         196,791   0.0%  ║
║  14 <core::fmt::Formatter>::pad_integral::write_prefix                  ░░░░░░░░░░░░░░░░░░░░         190,411   0.0%  ║
║  15 memcpy                                                              ░░░░░░░░░░░░░░░░░░░░         117,529   0.0%  ║
║  16 ziskos::io::commit_slice                                            ░░░░░░░░░░░░░░░░░░░░          85,079   0.0%  ║
║  17 <alloy_primitives::..::FixedBytes<…> as core::fmt::Debug>::fmt      ░░░░░░░░░░░░░░░░░░░░          57,891   0.0%  ║
║  18 <u32 as core::fmt::Display>::fmt                                    ░░░░░░░░░░░░░░░░░░░░          29,674   0.0%  ║
║  19 <core::fmt::Formatter as core::fmt::Write>::write_str               ░░░░░░░░░░░░░░░░░░░░          19,363   0.0%  ║
║  20 <core::fmt::Formatter>::debug_list                                  ░░░░░░░░░░░░░░░░░░░░          13,582   0.0%  ║
║  21 <core::fmt::builders::DebugList>::finish                            ░░░░░░░░░░░░░░░░░░░░          13,189   0.0%  ║
║  22 <…>::initialize::<…>                                                ░░░░░░░░░░░░░░░░░░░░           7,830   0.0%  ║
║  23 <u32>::_fmt_inner                                                   ░░░░░░░░░░░░░░░░░░░░           7,338   0.0%  ║
║  24 std::io::stdio::print_to_buffer_if_capture_used                     ░░░░░░░░░░░░░░░░░░░░           6,165   0.0%  ║
╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
```

## Prove

### Program Setup

Before generating a proof, you need to generate the program setup files. This must be done the first time after building the program ELF file, or any time it changes:

```bash
cargo-zisk program-setup
```
The program setup files will be generated in the `cache` directory located at `$HOME/.zisk`.

To clean the `cache` directory content, use the following command:
```bash
cargo-zisk utils clean-cache --all
```

### Generate Proof

To generate a proof, run the following command:

```bash
cargo-zisk prove -i ../host/tmp/input.bin -o proof
```

In this command:

* `-i` (`--input`) specifies the input file location.
* `-o` (`--output`) determines the output directory (in this example `proof`).

**Note**: If you have installed the GPU version of the ZisK binaries, you can use the `--gpu` flag to enable GPU acceleration during proof generation.

If the process is successful, you should see a message similar to:

```
...
INFO: --- PROVE SUMMARY ------------------------
INFO: Proof Time: 5.097 seconds
INFO: Execution completed in 5097ms, steps: 4450272
INFO: Execution summary: Proofman 4910ms + Execution 34ms + Count&Plan 17ms + Count&Plan MO 0ms
```

### Concurrent Proof Generation

Zisk proofs can be generated using multiple processes concurrently to improve performance and scalability. The standard MPI (Message Passing Interface) approach is used to launch these processes, which can run either on the same server or across multiple servers.

To execute a Zisk proof using multiple processes, use the following command:

```bash
mpirun --bind-to none -np <num_processes> -x OMP_NUM_THREADS=<num_threads_per_process> -x RAYON_NUM_THREADS=<num_threads_per_process> target/release/cargo-zisk <zisk arguments>
```
In this command:

* `<num_processes>` specifies the number of processes to launch.
* `<num_threads_per_process>` sets the number of threads used by each process via the `OMP_NUM_THREADS` and `RAYON_NUM_THREADS` environment variables.
* `--bind-to none` prevents binding processes to specific cores, allowing the operating system to schedule them dynamically for better load balancing.

Running a Zisk proof with multiple processes enables efficient workload distribution across multiple servers. **On a single server with many cores, splitting execution into smaller subsets of cores generally improves performance by increasing concurrency**. As a general rule, `<num_processes>` * `<num_threads_per_process>` should match the number of available CPU cores or double that if hyperthreading is enabled.

The total memory requirement increases proportionally with the number of processes. If each process requires approximately 25GB of memory, running P processes will require roughly (25 * P)GB of memory. Ensure that the system has sufficient available memory to accommodate all running processes.

### Verify Proof

To verify a generated proof, use the following command:

```bash
cargo-zisk verify -p proof
```

In this command:

* `-p` (`--proof`) specifies the final proof file generated with cargo-zisk prove.
* The remaining flags specify the files required for verification; they are optional, set by default to the files found in the `$HOME/.zisk` directory.
