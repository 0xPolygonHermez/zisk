#!/usr/bin/env bash
# Reproduces the saved request (see README.md): downloads the Qwen3-4B-Instruct-2507 checkpoint,
# prepares the inputs of the 112 executions, builds the guest and an assembly emulator
# (ziskemuasm) for it, and runs them.
# Needs about 8 GB each for the checkpoint and the prepared inputs, plus build and run output.
#
# It can be run again: the checkpoint and the inputs are kept and reused (delete the records
# directory to prepare them again), and an earlier run's output is moved aside to run-<date>.
#
# Usage: reproduce.sh [-v] [-c] [-h | --help]
#   -v          verbose: explain each phase before starting it
#   -c          clean: delete everything the script creates, then exit: target/qwen3-benchmark
#               (the checkpoint, the inputs, the assembly emulator and every run), this example's
#               build directories, and shared memory left by an interrupted run
#   -h, --help  print this help
#
# Each phase ends with a line giving its duration and the total so far.
#
# The assembly emulator reports steps but no cost. EMULATOR=ziskemu runs on ziskemu instead,
# which also reports the cost but is much slower.
set -euo pipefail

usage() { sed -n '2,20s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"; }

# getopts knows only short options: handle --help first
for argument in "$@"; do
  case $argument in
    --help) usage; exit 0 ;;
    --*) echo "unknown option: $argument" >&2; usage >&2; exit 1 ;;
  esac
done

verbose=0
clean=0
while getopts "vch" option; do
  case $option in
    v) verbose=1 ;;
    c) clean=1 ;;
    h) usage; exit 0 ;;
    *) usage >&2; exit 1 ;;
  esac
done
shift $((OPTIND - 1))
(($# == 0)) || { usage >&2; exit 1; }

emulator=${EMULATOR:-asm}
[[ $emulator == asm || $emulator == ziskemu ]] || { echo "EMULATOR must be asm or ziskemu" >&2; exit 1; }

example=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$example/../.." && pwd)
work=$root/target/qwen3-benchmark

# The runner names its ziskemuasm server's shared memories and semaphores QWEN<pid>_..., after
# its own process ID. A run that is interrupted (Ctrl-C) leaves them behind, holding about 1 GB of
# memory, so they are removed at every start and exit, and by -c. A running runner's are kept
remove_stale_shared_memory() {
  local entry pid
  for entry in /dev/shm/QWEN* /dev/shm/sem.QWEN*; do
    [[ -e $entry ]] || continue
    pid=${entry##*QWEN}
    pid=${pid%%_*}
    if [[ $pid =~ ^[0-9]+$ ]] && ! kill -0 "$pid" 2>/dev/null; then
      echo "Deleting $entry, left by an interrupted run"
      rm -f "$entry"
    fi
  done
}

if ((clean)); then
  for dir in "$work" "$example"/{runner,guest,tile}/target; do
    [[ -e $dir ]] || continue
    echo "Deleting $dir ($(du -sh "$dir" | cut -f1))"
    rm -rf "$dir"
  done
  remove_stale_shared_memory
  exit 0
fi
remove_stale_shared_memory

# Phases: phase TITLE [PARAGRAPH...] ends the previous phase, printing its duration, and starts
# a new one; in verbose mode, it first prints the numbered title and the explanation (one
# paragraph per argument). The last phase ends when the script exits.
bold='' plain=''
if [[ -t 1 ]]; then bold=$'\033[1m' plain=$'\033[0m'; fi
phase_number=0
phase_count=$([[ $emulator == asm ]] && echo 6 || echo 5)
phase_title=''
phase_start=$SECONDS

# Formats a number of seconds as 1h 02m 03s, 2m 03s or 3s
duration() {
  local s=$1
  if ((s >= 3600)); then
    printf '%dh %02dm %02ds' $((s / 3600)) $((s % 3600 / 60)) $((s % 60))
  elif ((s >= 60)); then
    printf '%dm %02ds' $((s / 60)) $((s % 60))
  else
    printf '%ds' "$s"
  fi
}

# Prints how long the current phase took, or that it failed
end_phase() {
  local outcome=${1:-done in}
  [[ -n $phase_title ]] || return 0
  printf '%s==> [%d/%d] %s: %s %s (total %s)%s\n' "$bold" "$phase_number" "$phase_count" \
    "$phase_title" "$outcome" "$(duration $((SECONDS - phase_start)))" "$(duration "$SECONDS")" "$plain"
  phase_title=''
}
trap 'if (($? == 0)); then end_phase; else end_phase "failed after"; fi; remove_stale_shared_memory' EXIT

phase() {
  end_phase
  phase_number=$((phase_number + 1))
  phase_title=$1
  shift
  if ((verbose)); then
    printf '\n%s==> [%d/%d] %s%s\n' "$bold" "$phase_number" "$phase_count" "$phase_title" "$plain"
    for paragraph in "$@"; do
      printf '%s\n' "$paragraph" | fold -s -w 100 | sed 's/^/    /'
    done
    echo
  fi
  phase_start=$SECONDS
}

if ((verbose)); then
  echo "Reproducing the saved Qwen3-4B request on the ZisK emulator ($emulator)."
  echo "Everything is written under $work."
fi

# The inputs are published by prepare only once complete, so existing ones are reused, and then
# the checkpoint is not needed
qwen=$example/runner/target/release/qwen3-4b
if [[ -d $work/records ]]; then
  phase "Skipping the checkpoint download" \
    "The inputs were already prepared, in $work/records, so the checkpoint is not downloaded again."
  phase "Building the runner and reusing the inputs" \
    "The runner (qwen3-4b, a native program) is built. The inputs of the 112 executions in \
$work/records were prepared by an earlier run and are reused; delete that directory to prepare \
them again."
  cargo build --release --locked --manifest-path "$example/runner/Cargo.toml"
else
  phase "Downloading the checkpoint" \
    "The original BF16 weights of Qwen3-4B-Instruct-2507 (about 8 GB in three safetensors shards), \
with its config and index, pinned to one revision of the Hugging Face repository, into \
$work/checkpoint. Files already there are downloaded again."
  mkdir -p "$work/checkpoint"
  checkpoint_url=https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507/resolve/cdbee75f17c01a7cc42f958dc650907174af0554
  for file in config.json model.safetensors.index.json model-0000{1,2,3}-of-00003.safetensors; do
    curl --fail --location --retry 3 "$checkpoint_url/$file" -o "$work/checkpoint/$file" || exit 1
  done

  phase "Building the runner and preparing the inputs" \
    "The runner (qwen3-4b, a native program) checks the checkpoint's tensor inventory and splits the \
model into the inputs of 112 separate ZisK executions, about 8 GB in $work/records:" \
    "- 1 embedding: looks up the 39 input tokens (28 of the prompt, 11 of the reply);" \
    "- 36 decoder layers: each one gets one layer's weights (about 193 MiB);" \
    "- 75 vocabulary-head shards: each one scores 2,048 tokens of the vocabulary." \
    "The activations that pass from one execution to the next are filled in later, during the run."
  cargo build --release --locked --manifest-path "$example/runner/Cargo.toml"
  "$qwen" prepare "$work/checkpoint" "$work/records"
fi

# The runner writes into a fresh directory: move an earlier run aside, named after its date
if [[ -e $work/run ]]; then
  previous=$work/run-$(date -r "$work/run" +%Y%m%d-%H%M%S)
  if ((verbose)); then echo "Moving the earlier run's output to $previous."; fi
  mv "$work/run" "$previous"
fi

phase "Building the guest" \
  "The guest is the RISC-V program that ZisK runs for every execution: it reads its stage's input, \
computes the stage with the dot_bf16 precompile for the matrix multiplications, and publishes its \
output buffer. It is built with the ZisK toolchain (cargo +zisk)."
(
  cd "$example/guest"
  RUSTFLAGS="--cfg zisk_guest -C target-feature=+unaligned-scalar-mem -C link-arg=-T$root/ziskbuild/zisk_linker_script.ld -L native=$HOME/.zisk/bin" \
    cargo +zisk build --release --locked --target riscv64ima-zisk-zkvm-elf
)
elf=$example/guest/target/riscv64ima-zisk-zkvm-elf/release/qwen3-4b-guest

if [[ $emulator == ziskemu ]]; then
  phase "Building ziskemu" \
    "ZisK's Rust emulator, which interprets the guest and measures both its steps and its cost."
  (cd "$root" && cargo build --release --locked -p ziskemu)

  phase "Running the 112 executions on ziskemu" \
    "The embedding, the 36 layers in order (each one gets the previous one's output) and the 75 \
head shards, one ziskemu process each. ziskemu runs with statistics on, to measure the cost, which \
makes it slow: about 3 minutes per layer, so about 2 hours in all." \
    "At the end, the runner takes the most likely token at each of the 12 predicted positions and \
compares it with the saved reply. Logs, outputs and summary.json go to $work/run."
  "$qwen" run --records "$work/records" --emulator "$root/target/release/ziskemu" \
    --elf "$elf" --output "$work/run"
  exit 0
fi

phase "Building the transpiler and the C libraries" \
  "zisk-transpiler-riscv translates the guest into x86-64 assembly; ziskclib and zisk-lib-c are \
the C libraries the assembly emulator links with (precompiles, floats, hints)."
asm=$work/asm
mkdir -p "$asm"
(cd "$root" && cargo build --release --locked -p zisk-transpiler-riscv -p ziskclib -p zisk-lib-c)

phase "Building the assembly emulator for this guest" \
  "The transpiler turns the guest ELF into x86-64 assembly in fast mode (--gen=0: execution only, \
no traces for proving), and emulator-asm links it into a native executable, ziskemuasm, in $asm. \
emulator-asm's build directory is cleaned first, since objects left there by builds for other trace \
targets would otherwise be reused."
"$root/target/release/zisk-transpiler-riscv" "$elf" "$asm/emu.asm" --gen=0
make -C "$root/emulator-asm" clean
make -C "$root/emulator-asm" EMU_PATH="$asm/emu.asm" OUT_PATH="$asm/ziskemuasm"

phase "Running the 112 executions on the assembly emulator" \
  "The runner starts one ziskemuasm server and sends it the embedding, the 36 layers in order \
(each one gets the previous one's output) and the 75 head shards. After each execution the server \
writes the guest's output buffer to a file (--dump-guest-buffer), which feeds the next one. About \
15 seconds per layer, so about 12 minutes in all; it reports steps, but no cost." \
  "At the end, the runner takes the most likely token at each of the 12 predicted positions and \
compares it with the saved reply. Logs, outputs and summary.json go to $work/run."
"$qwen" run --records "$work/records" --asm "$asm/ziskemuasm" --output "$work/run"
