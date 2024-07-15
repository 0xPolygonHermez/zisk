#!/bin/bash

# Verificar si la variable de entorno SIM está configurada como true
if [ "$SIM" == "true" ]; then
    echo "The SIM environment variable is set to true."
    node ../../../ziskjs/src/sim/main.js -e $1 -n 1000000 -i input.bin -t trace.tr
else
    echo "The SIM environment variable is set to false. Running the QEMU RISC-V..."
    .cargo/file_size.sh
    qemu-system-riscv64 \
    -cpu rv64 \
    -machine virt \
    -device loader,file=./input_size.bin,addr=0x90000000 \
    -device loader,file=./input.bin,addr=0x90000008 \
    -m 1G \
    -s \
    -nographic \
    -serial mon:stdio \
    -bios none \
    -kernel $1
fi


