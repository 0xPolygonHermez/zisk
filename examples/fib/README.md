# zisk hello world in rust

## Compile in gost

```bash
cargo run --target riscv64ima-polygon-ziskos-elf
#or 
cargo run --target x86_64-unknown-linux-gnu
```

## Compile and run in qemu
```bash
cargo build --bin hello_zisk --target .cargo/riscv64ima-unknown-none-elf.json -Z build-std=core
readelf -l target/riscv64ima-unknown-none-elf/debug/hello_zisk
qemu-system-riscv64 -cpu rv64 -machine virt -m 1G -nographic -bios target/riscv64ima-unknown-none-elf/debug/hello_zisk -S -gdb tcp:localhost:2222
cp target/riscv64ima-unknown-none-elf/debug/hello_zisk ../ziskjs/work/
```

```bash
cd /home/jbaylina/riscv-gnu-toolchain/gdb/gdb
./gdb
    file /home/jbaylina/hellozisk_rust/target/riscv64ima-unknown-none-elf/debug/hello_zisk
    target remote localhost:2222
    si
    x/10i $pc-16
```

## Trace with qemu
```
qemu-system-riscv64 -cpu rv64 -machine virt -m 1G -nographic -bios target/riscv64ima-unknown-none-elf/debug/hello_zisk -gdb tcp:localhost:2222 -S
/home/jbaylina/riscv-gnu-toolchain/opt/riscv/bin/riscv64-unknown-elf-gdb --command zisk_trace_gdb.py
```

## Trace with ziskjs
```
/home/jbaylina/n/bin/node src/sim/main.js -r work/hello_zisk.ziskrom -n 10000 -t work/sim.tr -p 100
```

### mvendorid CSR 0xF11
    We may pay for a number

    JEDEC Codes:
        https://www.jedec.org/standards-documents/docs/jep-106ab

    JEDEC -> mvendorid
        https://github.com/riscv/riscv-isa-manual/issues/32

### marchid CSR 0xF12
    Lets use FE here temporally until we open source
    RISC-V Architercures
        https://github.com/riscv/riscv-isa-manual/blob/main/marchid.md




