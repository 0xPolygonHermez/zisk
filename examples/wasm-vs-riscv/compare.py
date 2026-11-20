#!/usr/bin/env python3
"""Runs every workload on the wasm and RISC-V guests with `ziskemu -X` and prints a markdown
table: steps and variable cost at n = 0 (the fixed overhead) and per unit of work (the slope
between n = 0 and n = N), with the wasm / RISC-V ratio.

    ./compare.py [--ziskemu PATH] [--json results.json]

The guests must be built first (see README.md). Both public outputs are compared on every run.
ziskemu must be built with the `float` feature: the float and json workloads use f64 in the
wasm guest.
"""

import argparse
import json
import re
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
WASM = HERE / "wasm-guest/target/wasm32-wasip1/release/wasm-guest.wasm"
RISCV = HERE / "riscv-guest/target/elf/riscv64ima-zisk-zkvm-elf/release/riscv-guest"

# (id, name, N, what one unit of work is)
WORKLOADS = [
    (0, "fib", 100_000, "loop iteration"),
    (1, "sort", 10_000, "element sorted"),
    (2, "btree", 2_000, "insert + lookup"),
    (3, "calls", 100, "1,023 calls"),
    (4, "memcpy", 200, "4 KiB copy"),
    (5, "bigint", 1_000, "256 × 256-bit product"),
    (6, "keccak_sw", 100, "Keccak-f, software"),
    (7, "keccak_pc", 1_000, "Keccak-f, precompile"),
    (8, "float", 1_000, "f64 step"),
    (9, "json", 200, "JSON record"),
]

FIELDS = {
    "steps": r"^STEPS\s+([\d,]+)",
    "main": r"^MAIN\s+([\d,]+)",
    "opcodes": r"^OPCODES\s+([\d,]+)",
    "precompiles": r"^PRECOMPILES\s+([\d,]+)",
    "memory": r"^MEMORY\s+([\d,]+)",
    "variable": r"^VARIABLE\s+([\d,]+)",
    "rom": r"^ROM USAGE\s+([\d,]+)",
}


def run(ziskemu, guest, wid, n):
    with tempfile.NamedTemporaryFile(suffix=".bin") as f:
        f.write(struct.pack("<QQQ", 16, wid, n))
        f.flush()
        report = subprocess.run(
            [ziskemu, "-e", str(guest), "-i", f.name, "-X"],
            check=True, capture_output=True, text=True).stdout
        output = subprocess.run(
            [ziskemu, "-e", str(guest), "-i", f.name, "-c"],
            check=True, capture_output=True, text=True).stdout
    values = {}
    for key, pattern in FIELDS.items():
        m = re.search(pattern, report, re.M)
        values[key] = int(m.group(1).replace(",", "")) if m else 0
    return values, output


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--ziskemu", default="ziskemu")
    parser.add_argument("--json", help="also write the raw numbers here")
    args = parser.parse_args()

    results = []
    for wid, name, n, unit in WORKLOADS:
        row = {"name": name, "n": n, "unit": unit}
        for size in (0, n):
            outputs = {}
            for arch, guest in (("wasm", WASM), ("riscv", RISCV)):
                values, outputs[arch] = run(args.ziskemu, guest, wid, size)
                row[f"{arch}_{size and 'n' or '0'}"] = values
            if outputs["wasm"] != outputs["riscv"]:
                sys.exit(f"{name} n={size}: the public outputs differ")
        results.append(row)
        print(f"{name}: done", file=sys.stderr)

    if args.json:
        Path(args.json).write_text(json.dumps(results, indent=1))

    def ratio(a, b):
        return f"{a / b:.1f}×" if b else "–"

    print("Fixed overhead (n = 0):\n")
    print("| Workload | wasm steps | RISC-V steps | ratio | wasm cost | RISC-V cost | ratio |")
    print("|---|---:|---:|---:|---:|---:|---:|")
    for r in results[:1]:
        w, v = r["wasm_0"], r["riscv_0"]
        print(f"| any | {w['steps']:,} | {v['steps']:,} | {ratio(w['steps'], v['steps'])} "
              f"| {w['variable']:,} | {v['variable']:,} | {ratio(w['variable'], v['variable'])} |")

    print("\nPer unit of work (slope between n = 0 and n = N):\n")
    print("| Workload | Unit | N | wasm steps | RISC-V steps | ratio "
          "| wasm cost | RISC-V cost | ratio |")
    print("|---|---|---:|---:|---:|---:|---:|---:|---:|")
    for r in results:
        n = r["n"]
        slope = {
            arch: {k: (r[f"{arch}_n"][k] - r[f"{arch}_0"][k]) / n for k in FIELDS}
            for arch in ("wasm", "riscv")
        }
        w, v = slope["wasm"], slope["riscv"]
        print(f"| {r['name']} | {r['unit']} | {n:,} | {w['steps']:,.0f} | {v['steps']:,.0f} "
              f"| {ratio(w['steps'], v['steps'])} | {w['variable']:,.0f} | {v['variable']:,.0f} "
              f"| {ratio(w['variable'], v['variable'])} |")

    print("\nCost split per unit of work (wasm / RISC-V):\n")
    print("| Workload | main | opcodes | precompiles | memory |")
    print("|---|---:|---:|---:|---:|")
    for r in results:
        n = r["n"]
        cells = []
        for k in ("main", "opcodes", "precompiles", "memory"):
            w = (r["wasm_n"][k] - r["wasm_0"][k]) / n
            v = (r["riscv_n"][k] - r["riscv_0"][k]) / n
            cells.append(f"{round(w) + 0:,} / {round(v) + 0:,}")
        print(f"| {r['name']} | " + " | ".join(cells) + " |")

    print("\nROM usage (instructions): wasm {:,}, RISC-V {:,}".format(
        results[0]["wasm_0"]["rom"], results[0]["riscv_0"]["rom"]))


if __name__ == "__main__":
    main()
