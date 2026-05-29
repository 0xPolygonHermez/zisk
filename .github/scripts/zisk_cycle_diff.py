#!/usr/bin/env python3
"""Render a markdown diff of `ziskemu -X` reports between two branches.

Reads per-program report files produced by zisk_bench.sh from a base dir and a
PR dir, parses the STEPS + COST DISTRIBUTION numbers, and prints one markdown
table per program (Base Branch | Current PR | Diff | Diff (%)).

Usage: zisk_cycle_diff.py <BASE_DIR> <PR_DIR>
"""
import os
import re
import sys

# Report label -> friendly row name, in display order. Keys must match the
# leading label of each line emitted by emulator/src/stats/stats.rs::report().
ROWS = [
    ("STEPS", "Total Steps"),
    ("MAIN", "Main Cost"),
    ("OPCODES", "Opcodes Cost"),
    ("PRECOMPILES", "Precompiles Cost"),
    ("MEMORY", "Memory Cost"),
    ("VARIABLE", "Variable Cost"),
    ("BASE", "Base Cost"),
    ("TOTAL", "Total Cost"),
    ("FROPS", "Frops Cost"),
]

LINE_RE = re.compile(r"^\s*([A-Z]+)\s+([\d,]+)")


def parse_report(path):
    """Parse a ziskemu -X report into {LABEL: int}. Missing file -> {}."""
    out = {}
    if not os.path.isfile(path):
        return out
    wanted = {label for label, _ in ROWS}
    with open(path) as f:
        for line in f:
            m = LINE_RE.match(line)
            if m and m.group(1) in wanted:
                # First occurrence wins (the summary section comes first).
                out.setdefault(m.group(1), int(m.group(2).replace(",", "")))
    return out


def fmt(n):
    return f"{n:,}" if n is not None else "N/A"


def table(program, base, pr):
    lines = [
        f"|{program:^22}|Base Branch|Current PR|Diff|Diff (%)|",
        "|----------------------|-----------|-----------|----|--------|",
    ]
    for label, name in ROWS:
        b = base.get(label)
        p = pr.get(label)
        if b is None and p is None:
            continue
        if b is None or p is None:
            diff = pct = "N/A"
        else:
            d = p - b
            diff = f"{d:,}"
            pct = f"{(d / b * 100):.2f}" if b != 0 else "N/A"
        lines.append(f"|{name}|{fmt(b)}|{fmt(p)}|{diff}|{pct}|")
    return "\n".join(lines)


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    base_dir, pr_dir = sys.argv[1], sys.argv[2]

    # Programs = the .txt reports present in the PR dir (fall back to base dir).
    src = pr_dir if os.path.isdir(pr_dir) else base_dir
    programs = sorted(
        f[:-4] for f in os.listdir(src) if f.endswith(".txt")
    )

    out = ["## ZisK cycle tracking", ""]
    out.append(
        "Emulator cost report (`ziskemu -X`) for each benchmark guest, "
        "PR vs base branch.\n"
    )
    for prog in programs:
        base = parse_report(os.path.join(base_dir, f"{prog}.txt"))
        pr = parse_report(os.path.join(pr_dir, f"{prog}.txt"))
        out.append(table(prog, base, pr))
        out.append("")

    print("\n".join(out))


if __name__ == "__main__":
    main()