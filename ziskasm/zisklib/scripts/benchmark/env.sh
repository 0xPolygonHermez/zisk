# Sourced by the benchmark scripts. Override any of these from the environment:
#   RISCV_CC  bare-metal RISC-V C compiler (riscv64-unknown-elf-gcc or the xpack
#             riscv-none-elf-gcc)
#   ZISKEMU   emulator to measure; build it with
#             `cargo build --release -p ziskemu --features ziskasm`
#   WORK      scratch directory for the ELFs, outputs and logs
BENCH=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ZISK=$(cd "$BENCH/../../../.." && pwd)
CC=${RISCV_CC:-riscv64-unknown-elf-gcc}
ZISKEMU=${ZISKEMU:-$ZISK/target/release/ziskemu}
WORK=${WORK:-/tmp/zisklib_bench}
mkdir -p "$WORK"; : > "$WORK/empty.bin"
CFLAGS="-march=rv64ima_zicsr_zbb -mabi=lp64 -mcmodel=medany -nostdlib -ffreestanding -O2
 -Wl,--gc-sections -w -I$ZISK/ziskasm/lang/c/example -I$ZISK/ziskasm/lang/c/include
 -T $ZISK/ziskbuild/zisk_linker_script.ld"

# guest_cc OUT.elf SRC.c [-D...]: build a C guest linked against the EF zkvm_* thunks.
guest_cc() {
  local out=$1 src=$2; shift 2
  $CC $CFLAGS "$@" -o "$out" "$ZISK/ziskasm/lang/c/src/_start.s" "$src" "$ZISK/ziskasm/lang/c/src/zkvm_calls.s" \
      "$ZISK/ziskasm/lang/c/src/zkvm_mem.s"
}

# emu_run ELF OUT.bin [emu] -> "steps cost" (total steps and variable cost of the run)
emu_run() {
  timeout 1200 "${3:-$ZISKEMU}" -e "$1" -i "$WORK/empty.bin" -o "$2" -X 2>&1 |
    awk '/^STEPS/{gsub(",","",$2); s=$2} /^VARIABLE/{gsub(",","",$2); v=$2} END{print s, v}'
}
