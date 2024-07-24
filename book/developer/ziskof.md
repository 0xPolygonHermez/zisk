# Ziskof

## Riscof tests
The following test generates the riscof test files, converts the corresponding .elf files into ZisK ROMs, and executes them providing the output in stdout for comparison against a reference RISCV implementation.  This process is not trivial and has been semi-automatized.

First, compile the ZisK Emulator:

```sh
$ cargo clean
$ cargo build --release
```

Second, download and run a docker image from the riscof repository to generate and run the riscof tests:

```sh
$ docker run --rm -v ./target/release/ziskemu:/program -v ./riscof/:/workspace/output/ -ti  hermeznetwork/ziskof:latest
```

The test can take a few minutes to complete.  Any error would be displayed in red.