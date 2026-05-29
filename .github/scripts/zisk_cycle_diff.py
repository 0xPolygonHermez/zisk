#!/usr/bin/env python3
"""Render a markdown diff of `ziskemu -X` reports between two branches.

Reads per-program report files produced by zisk_bench.sh from a base dir and a
PR dir, parses the STEPS + COST DISTRIBUTION numbers, and prints a markdown
comment: a summary table (headline Steps + Total Cost per guest) followed by a
collapsible full cost breakdown per guest.

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


def delta(b, p):
    """Return a signed percentage with a color indicator, or N/A.

    🔴 increase (regression — cost went up), 🟢 decrease (improvement),
    ➖ no change. Lower is better, so a positive diff is a regression.
    """
    if b is None or p is None:
        return "N/A"
    d = p - b
    if d == 0:
        return "➖ 0.00%"
    if b == 0:
        return "🔴 new" if d > 0 else "🟢 —"
    dot = "🔴" if d > 0 else "🟢"
    return f"{dot} {d / b * 100:+.2f}%"


def summary(rows):
    """Headline table: Steps + Total Cost (PR value and delta) per guest."""
    out = [
        "| Guest | Steps | Δ Steps | Total Cost | Δ Total Cost |",
        "| --- | --- | --- | --- | --- |",
    ]
    for prog, base, pr in rows:
        out.append(
            f"| {prog} "
            f"| {fmt(pr.get('STEPS'))} | {delta(base.get('STEPS'), pr.get('STEPS'))} "
            f"| {fmt(pr.get('TOTAL'))} | {delta(base.get('TOTAL'), pr.get('TOTAL'))} |"
        )
    return "\n".join(out)


def breakdown(program, base, pr):
    """Full per-row table for one guest, wrapped in a collapsible section."""
    lines = [
        "| Metric | Base Branch | Current PR | Diff | Diff (%) |",
        "| --- | --- | --- | --- | --- |",
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
            pct = f"{(d / b * 100):.2f}%" if b != 0 else "N/A"
        lines.append(f"| {name} | {fmt(b)} | {fmt(p)} | {diff} | {pct} |")
    table = "\n".join(lines)
    return (
        f"<details>\n<summary><b>{program}</b> — full cost breakdown</summary>\n\n"
        f"{table}\n\n"
        "</details>"
    )


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    base_dir, pr_dir = sys.argv[1], sys.argv[2]

    # Programs = the .txt reports present in the PR dir (fall back to base dir).
    src = pr_dir if os.path.isdir(pr_dir) else base_dir
    programs = sorted(f[:-4] for f in os.listdir(src) if f.endswith(".txt"))

    rows = [
        (
            prog,
            parse_report(os.path.join(base_dir, f"{prog}.txt")),
            parse_report(os.path.join(pr_dir, f"{prog}.txt")),
        )
        for prog in programs
    ]

    out = ["## 🔄 ZisK Cycle Tracking", ""]
    out.append(
        "Emulator cost report (`ziskemu -X`) comparing this PR against the base "
        "branch. **Lower is better** — a positive diff is a regression."
    )
    out.append("")

    if not rows:
        out.append("> ⚠️ No benchmark reports were produced.")
        print("\n".join(out))
        return

    out.append("### Summary")
    out.append("")
    out.append(summary(rows))
    out.append("")
    out.append("### Per-guest breakdown")
    out.append("")
    for prog, base, pr in rows:
        out.append(breakdown(prog, base, pr))
        out.append("")

    out.append("---")
    out.append(
        "<sub>🔺 increase (regression) · 🔻 decrease (improvement) · ➖ no change. "
        "`RAM USAGE` is excluded (runner-dependent); STEPS and all costs are "
        "deterministic functions of (ELF, input).</sub>"
    )

    print("\n".join(out))


if __name__ == "__main__":
    main()