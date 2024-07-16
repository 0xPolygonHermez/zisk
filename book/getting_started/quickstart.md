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
├── README.md
├── rust-toolchain
└── src
    ├── bin
    │   └── hello_zisk.rs
    ├── lib.rs
    └── script.ld

2 directories, 8 files
```

For running the program in the native architecture:
```
$ cargo run --target x86_64-unknown-linux-gnu
   Compiling crunchy v0.2.2
   Compiling tiny-keccak v2.0.2
   Compiling hellozisk_rust v0.1.0 (/home/edu/hello_world)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.58s
     Running `target/x86_64-unknown-linux-gnu/debug/hello_zisk`
Hello, Zisk!
keccak("Hello, Zisk!"): [147, 41, 209, 243, 3, 171, 124, 49, 98, 118, 203, 166, 56, 28, 45, 41, 53, 159, 129, 193, 208, 229, 15, 201, 63, 11, 158, 3, 183, 26, 50, 124]
```

For running the program in the ZisK architecture, this will run the program on a qemu emulating a riscv-64 architecture.
```
cargo run --target riscv64ima-polygon-ziskos-elf
   Compiling crunchy v0.2.2
   Compiling tiny-keccak v2.0.2
   Compiling hellozisk_rust v0.1.0 (/home/edu/hello_world)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.56s
     Running `qemu-system-riscv64 -cpu rv64 -machine virt -m 1G -s -nographic -serial 'mon:stdio' -bios target/riscv64ima-polygon-ziskos-elf/debug/hello_zisk`
Hello, Zisk!
keccak("Hello, Zisk!"): [147, 41, 209, 243, 3, 171, 124, 49, 98, 118, 203, 166, 56, 28, 45, 41, 53, 159, 129, 193, 208, 229, 15, 201, 63, 11, 158, 3, 183, 26, 50, 124]
```

## build inputs

To generate the input data for the program in this case, we use Protocol Buffers (it is also possible to do this manually in raw mode).
The input parameters are defined in the file src/bin/input.proto:
```
syntax = "proto3";
package inputs;

message Input {
  string msg = 1;
  uint64 n = 2;
  uint64 a = 3;
  uint64 b = 4;
}
```
Here, we have a message msg and three numbers: n, a, and b.

On the other hand, the gen_input.rs script is responsible for generating a .bin file with the parameters defined in the proto file.

```rust
fn main() -> io::Result<()> {
    let input = Input {
        msg: "Hello, Zisk!! by edu".to_string(),
        n: 0,
        a: 0,
        b: 1,
    };

    // Serialize the `Input` object to a binary file with fixed size
    serialize_input(&input)?;

    // Deserialize the `Input` object from the binary file
    let input_read = deserialize_input()?;

    // Print the deserialized data
    println!("Input: {:?}", input_read);

    Ok(())
}
```

To generate it, it is only necessary to execute:

```bash
$ cargo run --bin inputs --target x86_64-unknown-linux-gnu
   Compiling hellozisk_rust v0.1.0 (/home/edu/hellozisk_rust)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.35s
     Running `target/x86_64-unknown-linux-gnu/debug/inputs`
Input { msg: "Hello, Zisk!!", n: 0, a: 0, b: 1 }
```

this creates the `input.bin` file inside the `output` folder

## Run on ZisK simulator

You will need to have ziskjs installed in the upper path. Running it in release mode is recommended if you want to reduce the number of cycles, but debug mode helps to find problems that may be hidden in release mode.

```bash
SIM=true cargo run --release --target riscv64ima-polygon-ziskos-elf
```

## Compile and run in qemu with gdb
```bash
GDB=true cargo run --release --target riscv64ima-polygon-ziskos-elf
```

then in another terminal
```bash
riscv64-unknown-elf-gdb
    file target/riscv64ima-polygon-ziskos-elf/release/hello_zisk
    target remote :1234
    si
    x/10i $pc-16
```
or for tracing purposes:
```bash
riscv64-unknown-elf-gdb --command ../ziskjs/test/zisk_trace_gdb.py
```

