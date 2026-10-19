#!/usr/bin/env python3
"""Check the frequent-operations tables against what the Lean proofs assume.

An `add` executed by Main is proved on the operation bus (5000) by one of
several AIRs. One of them is a table, not a circuit: the FROPS ("frequent
operations") tables, precomputed rows of (op, a, b, c, flag) that the compiler
groups into the virtual AIR `VirtualTableZisk0`. Its contents are fixed data,
outside the pilout, so a proof can only assume them. This checks the
assumption against the data the setup commits.

For every argument the virtual table proves on the operation bus, the tuple is
read from the extractor's IR (`pilout-constraints extract`) -- constants and
fixed columns, exactly as the bus argument names them -- and evaluated at every
row of the compiled fixed file (`tmp/fixed/VirtualTableZisk0.fixed`). Every row
whose op is a checked opcode must carry the op's result:

    add (0x0a):  c = a + b mod 2^64,  flag = 0,  every limb below 2^32

The fixed file is the compiler's: a row-major array of u64, one column per
fixed column of the AIR, in id order.

Usage: check_frops.py <ir.json> <VirtualTableZisk0.fixed>
"""

import json
import sys

import numpy as np

OPERATION_BUS = 5000
MASK32 = (1 << 32) - 1

# op -> (name, the result it must carry). Arguments are Python ints.
SEMANTICS = {
    0x0A: ("add", lambda a, b: ((a + b) % 2**64, 0)),
}


def bus_args(ir):
    """The proving arguments on the operation bus: (hint index, tuple)."""
    out = []
    for k, hint in enumerate(ir["hints"]):
        if hint["name"] != "gsum_debug_data":
            continue
        fields = {f.get("name"): f["value"] for f in hint["fields"][0]["value"]["value"]}
        const = lambda name: fields[name]["value"].get("value")
        if const("busid") != str(OPERATION_BUS) or const("type_piop") != "1":
            continue
        tuple_ = [e["value"]["value"] for e in fields["expressions"]["value"]]
        out.append((k, tuple_))
    return out


P = 0xFFFF_FFFF_0000_0001  # the Goldilocks prime


def evaluator(ir, columns, n):
    """An operand -> its value at every row, in [0, P), as a uint64 array.

    Expressions are evaluated over the pool, mod P. Only constants and
    unshifted fixed columns can appear: a table has nothing else."""
    def operand(o):
        kind = o["kind"]
        if kind == "const":
            return np.full(n, int(o["value"]) % P, dtype=np.uint64)
        if kind == "fixed" and o.get("offset", 0) == 0:
            return np.asarray(columns[:, o["col"]]) % np.uint64(P)
        if kind == "expr":
            return node(ir["expressions"][o["idx"]])
        sys.exit(f"operand {o} is not something a fixed table can contain")

    def node(e):
        op = e["op"]
        if op == "neg":
            x = operand(e["value"]).astype(object)
            return ((-x) % P).astype(np.uint64)
        x = operand(e["lhs"]).astype(object)
        y = operand(e["rhs"]).astype(object)
        r = {"add": x + y, "sub": x - y, "mul": x * y}[op]
        return (r % P).astype(np.uint64)

    return operand


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    ir = json.load(open(sys.argv[1]))
    n = ir["air"]["num_rows"]
    ncols = len(ir["columns"]["fixed"])
    data = np.memmap(sys.argv[2], dtype="<u8", mode="r")
    if data.size != n * ncols:
        sys.exit(f"{sys.argv[2]}: {data.size} values, expected {n} rows x {ncols} columns")
    value = evaluator(ir, data.reshape(n, ncols), n)

    checked = 0
    failures = 0
    for k, tuple_ in bus_args(ir):
        if len(tuple_) != 8:
            sys.exit(f"hint {k}: a {len(tuple_)}-element tuple, expected (op, a, b, c, flag)")
        op, a0, a1, b0, b1, c0, c1, flag = (value(o) for o in tuple_)
        for code, (name, result) in SEMANTICS.items():
            rows = np.nonzero(op == code)[0]
            if rows.size == 0:
                continue
            limbs = [x[rows] for x in (a0, a1, b0, b1, c0, c1)]
            if any(int(x.max()) > MASK32 for x in limbs):
                print(f"FAIL hint {k}: a {name} row has a limb of 2^32 or more")
                failures += 1
                continue
            a = limbs[0].astype(object) + (limbs[1].astype(object) << 32)
            b = limbs[2].astype(object) + (limbs[3].astype(object) << 32)
            c = limbs[4].astype(object) + (limbs[5].astype(object) << 32)
            f = flag[rows].astype(object)
            bad = [r for r, x, y, z, w in zip(rows, a, b, c, f) if result(int(x), int(y)) != (int(z), int(w))]
            for r in bad[:5]:
                print(f"FAIL hint {k} row {r}: {name} {a[rows == r]} {b[rows == r]} -> {c[rows == r]}")
            failures += len(bad)
            checked += rows.size
            print(f"hint {k}: {rows.size} {name} rows, {len(bad)} wrong")

    if failures:
        sys.exit(f"FAIL: {failures} rows of {checked} do not carry their op's result")
    print(f"OK: all {checked} rows with a checked op carry its result")


if __name__ == "__main__":
    main()
