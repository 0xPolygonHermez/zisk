#!/usr/bin/env bash
set -euo pipefail

task_emulator=${1:?Usage: check-runtime.sh EMULATOR ELF FIXTURES ARTIFACTS}
task_elf=${2:?}
task_fixtures=${3:?}
task_artifacts=${4:?}
mkdir -p "$task_artifacts"

for task_mode in fast trace; do
    task_flags=(--steps --log-metrics)
    if [ "$task_mode" = trace ]; then task_flags=(-X --generate-minimal-traces --log-metrics); fi
    for task_calls in 0 1 8 9 1024 1025; do
        task_output="$task_artifacts/$task_mode-$task_calls.bin"
        "$task_emulator" --elf "$task_elf" --inputs "$task_fixtures/input-$task_calls.bin" \
            --output "$task_output" "${task_flags[@]}" \
            > "$task_artifacts/$task_mode-$task_calls.log" 2>&1
        node -e '
            const fs = require("node:fs"), assert = require("node:assert/strict");
            const expected = fs.readFileSync(process.argv[1]);
            const actual = fs.readFileSync(process.argv[2]);
            assert.equal(actual.length, 256);
            assert.deepEqual(actual.subarray(0, expected.length), expected);
            assert(actual.subarray(expected.length).every(value => value === 0));
        ' "$task_fixtures/expected-$task_calls.bin" "$task_output"
        printf '%s / %s calls: exact output passed\n' "$task_mode" "$task_calls"
    done
done

for task_lane in {0..15}; do
    task_log="$task_artifacts/invalid-lane-$task_lane.log"
    if "$task_emulator" --elf "$task_elf" --inputs "$task_fixtures/input-invalid-lane-$task_lane.bin" > "$task_log" 2>&1; then
        printf 'Noncanonical lane %s was accepted\n' "$task_lane" >&2
        exit 1
    fi
    rg -q 'noncanonical KoalaBear precompile input' "$task_log"
done
for task_offset in {1..7}; do
    task_log="$task_artifacts/misaligned-$task_offset.log"
    if "$task_emulator" --elf "$task_elf" --inputs "$task_fixtures/input-misaligned-$task_offset.bin" > "$task_log" 2>&1; then
        printf 'Misaligned offset %s was accepted\n' "$task_offset" >&2
        exit 1
    fi
    rg -q 'KoalaBear state must be 8-byte aligned' "$task_log"
done
printf 'All 16 noncanonical lanes and seven misaligned addresses rejected.\n'
