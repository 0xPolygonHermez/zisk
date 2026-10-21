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

    if stale:
        print(f"\nSTALE -- named in the Sail model but not in zisk_ops.rs:")
        for op in stale:
            print(f"  {op}  (ZOP_{op.upper()})")

    if unmodeled:
        print(f"\nUNMODELED -- {len(unmodeled)} Rust ops with no Sail clause:")
        print("  " + ", ".join(unmodeled))

    if stale:
        print("\nFAIL: the Sail model references opcodes that no longer exist.")
        return 1
    if args.strict and unmodeled:
        print("\nFAIL: --strict and the model is incomplete.")
        return 1
    print("\nOK: no stale opcodes.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
