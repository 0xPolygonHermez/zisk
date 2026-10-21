#!/usr/bin/env python3
"""Check the Sail model's opcode names against core/src/zisk_ops.rs.

The ZisK opcode table is referenced by name, as a string, from several
places that no compiler checks together: the .zisk assembly sources and
(now) this Sail model. A rename in zisk_ops.rs therefore breaks them
silently -- exactly how `blake2` -> `blake2b` slipped through a clean
textual merge and was only caught when the zisklib assemble test ran.

This guard makes that failure loud and immediate:

  STALE   an op named in the Sail model no longer exists in Rust.
          This is an error: the model is describing a machine we do not
          ship.

  WRONG CODE  the model gives an op another opcode than Rust does
          (its `zisk_op_of_code` clause). An error, like STALE.

  UNMODELED  an op in Rust with no Sail clause yet. Informational --
          expected while the model is being filled in. Pass --strict to
          fail on these too, once coverage is meant to be complete.

Naming convention: the Sail constructor ZOP_<NAME> corresponds to the
Rust mnemonic <name> lowercased, e.g. ZOP_SLL_W <-> "sll_w".
"""

import argparse
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
RUST_OPS = ROOT / "core" / "src" / "zisk_ops.rs"
SAIL_MODEL = pathlib.Path(__file__).resolve().parent / "model"

# Matches the mnemonic in e.g.  (Blake2b, "blake2b", Blake2b, BLAKE2B_COST, ...)
RUST_OP_RE = re.compile(r'^\s*\(\w+,\s*"([a-z0-9_]+)"', re.M)
RUST_CODE_RE = re.compile(r'^\s*\(\w+,\s*"([a-z0-9_]+)",\s*\w+,\s*\w+,\s*(0x[0-9a-fA-F]+)', re.M)
SAIL_CODE_RE = re.compile(r"^\s*function clause zisk_op_of_code\s*\((0x[0-9a-fA-F]+)\)\s*=\s*Some\(ZOP_([A-Z0-9_]+)\(\)\)", re.M)
SAIL_OP_RE = re.compile(r"^\s*union clause zisk_op\s*=\s*ZOP_([A-Z0-9_]+)", re.M)


def rust_ops() -> set[str]:
    if not RUST_OPS.exists():
        sys.exit(f"cannot find {RUST_OPS}")
    return set(RUST_OP_RE.findall(RUST_OPS.read_text()))


def sail_ops() -> set[str]:
    names = set()
    for f in sorted(SAIL_MODEL.glob("*.sail")):
        names |= {m.lower() for m in SAIL_OP_RE.findall(f.read_text())}
    return names


def rust_codes() -> dict[str, int]:
    return {n: int(c, 16) for n, c in RUST_CODE_RE.findall(RUST_OPS.read_text())}


def sail_codes() -> dict[str, int]:
    codes = {}
    for f in sorted(SAIL_MODEL.glob("*.sail")):
        codes |= {n.lower(): int(c, 16) for c, n in SAIL_CODE_RE.findall(f.read_text())}
    return codes


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--strict", action="store_true",
                    help="also fail when Rust ops have no Sail clause")
    args = ap.parse_args()

    rust, sail = rust_ops(), sail_ops()
    stale = sorted(sail - rust)
    unmodeled = sorted(rust - sail)
    modeled = len(sail & rust)

    print(f"Rust ops: {len(rust)}   Sail clauses: {len(sail)}   "
          f"modeled: {modeled}/{len(rust)} ({100 * modeled // max(len(rust), 1)}%)")

    rc, sc = rust_codes(), sail_codes()
    wrong = sorted(op for op in sail & rust if sc.get(op) != rc.get(op))
    if wrong:
        print(f"\nWRONG CODE -- the model's opcode is not zisk_ops.rs's:")
        for op in wrong:
            got = f"{sc[op]:#04x}" if op in sc else "none"
            print(f"  {op}: model {got}, Rust {rc[op]:#04x}")

    if stale:
        print(f"\nSTALE -- named in the Sail model but not in zisk_ops.rs:")
        for op in stale:
            print(f"  {op}  (ZOP_{op.upper()})")

    if unmodeled:
        print(f"\nUNMODELED -- {len(unmodeled)} Rust ops with no Sail clause:")
        print("  " + ", ".join(unmodeled))

    if stale or wrong:
        print("\nFAIL: the Sail model references opcodes that no longer exist, or numbers them wrongly.")
        return 1
    if args.strict and unmodeled:
        print("\nFAIL: --strict and the model is incomplete.")
        return 1
    print("\nOK: no stale opcodes, and every opcode number agrees.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
