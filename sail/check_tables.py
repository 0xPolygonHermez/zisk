#!/usr/bin/env python3
"""Check the virtual tables' contents against what the Lean proofs assume.

Some of what the proofs rely on is not a constraint but the contents of a
table: fixed data, outside the pilout, which the compiler groups into the
virtual AIRs `VirtualTableZisk0` and `VirtualTableZisk1`. A proof can only
assume such contents, so this checks each assumption against the data the
setup commits.

Every proving argument of a table is read from the extractor's IR
(`pilout-constraints extract`) -- its bus and its tuple, constants, fixed
columns and expressions over them, exactly as the bus argument states them --
and evaluated at every row of the compiled fixed file. A merged table may
serve several buses through one argument, picking the bus per row; the bus is
evaluated per row too, and each check applies to the rows on its bus.

Two checks:

1. **Results on the operation bus (5000).** Every row whose op is a checked
   opcode carries the op's result (the FROPS tables of precomputed results):

       add (0x0a):  c = a + b mod 2^64,  flag = 0,  every limb below 2^32

2. **Ops a table never serves.** The Arith table (bus 331) and the
   BinaryExtension table (bus 124) are where Arith and BinaryExtension look
   their `op` column up, as the first element of the tuple. A row on one of
   those buses never has `add` there, which is what rules the two AIRs out as
   provers of an `add`.

The fixed file is the compiler's: a row-major array of u64, one column per
fixed column of the AIR, in id order.

Usage: check_tables.py <ir.json> <Air.fixed> [<ir.json> <Air.fixed> ...]
"""

import json
import sys

import numpy as np

OPERATION_BUS = 5000
MASK32 = (1 << 32) - 1
P = 0xFFFF_FFFF_0000_0001  # the Goldilocks prime

# op -> (name, the result it must carry). Arguments are Python ints.
SEMANTICS = {
    0x0A: ("add", lambda a, b: ((a + b) % 2**64, 0)),
}

# table bus -> (name, ops whose first element it must never be)
NEVER = {
    331: ("Arith table", {0x0A: "add"}),
    124: ("BinaryExtension table", {0x0A: "add"}),
}


def proving_args(ir):
    """Every proving argument: (hint index, opids, bus operand, tuple)."""
    out = []
    for k, hint in enumerate(ir["hints"]):
        if hint["name"] != "gsum_debug_data":
            continue
        fields = {f.get("name"): f["value"] for f in hint["fields"][0]["value"]["value"]}
        if fields["type_piop"]["value"].get("value") != "1":
            continue
        opids = {int(e["value"]["value"]["value"]) for e in fields["opids"]["value"]}
        tuple_ = [e["value"]["value"] for e in fields["expressions"]["value"]]
        out.append((k, opids, fields["busid"]["value"], tuple_))
    return out


def evaluator(ir, columns, n):
    """An operand -> its value at every row, in [0, P), as a uint64 array.

    Expressions are evaluated over the pool, mod P. Only constants and
    unshifted fixed columns can appear: a table has nothing else."""
    cache = {}

    def operand(o):
        kind = o["kind"]
        if kind == "const":
            return np.full(n, int(o["value"]) % P, dtype=np.uint64)
        if kind == "fixed" and o.get("offset", 0) == 0:
            return np.asarray(columns[:, o["col"]]) % np.uint64(P)
        if kind == "expr":
            if o["idx"] not in cache:
                cache[o["idx"]] = node(ir["expressions"][o["idx"]])
            return cache[o["idx"]]
        sys.exit(f"operand {o} is not something a fixed table can contain")

    def node(e):
        op = e["op"]
        if op == "neg":
            x = operand(e["value"]).astype(object)
            return ((-x) % P).astype(np.uint64)
        lhs, rhs = operand(e["lhs"]), operand(e["rhs"])
        if op == "add" and not (lhs >= np.uint64(P // 2)).any() and not (rhs >= np.uint64(P // 2)).any():
            return lhs + rhs  # no reduction needed: both far below P
        x, y = lhs.astype(object), rhs.astype(object)
        r = {"add": x + y, "sub": x - y, "mul": x * y}[op]
        return (r % P).astype(np.uint64)

    return operand


def check_results(k, rows, tuple_values):
    """Check 1 on one argument's rows on the operation bus; returns (checked, failures)."""
    if len(tuple_values) < 8:
        sys.exit(f"hint {k}: a {len(tuple_values)}-element tuple, expected (op, a, b, c, flag, ...)")
    op, a0, a1, b0, b1, c0, c1, flag = (v[rows] for v in tuple_values[:8])
    checked = failures = 0
    for code, (name, result) in SEMANTICS.items():
        sel = op == code
        if not sel.any():
            continue
        limbs = [x[sel] for x in (a0, a1, b0, b1, c0, c1)]
        if any(int(x.max()) > MASK32 for x in limbs):
            print(f"FAIL hint {k}: a {name} row has a limb of 2^32 or more")
            failures += 1
            continue
        a = limbs[0].astype(object) + (limbs[1].astype(object) << 32)
        b = limbs[2].astype(object) + (limbs[3].astype(object) << 32)
        c = limbs[4].astype(object) + (limbs[5].astype(object) << 32)
        f = flag[sel].astype(object)
        bad = [j for j, (x, y, z, w) in enumerate(zip(a, b, c, f))
               if result(int(x), int(y)) != (int(z), int(w))]
        for j in bad[:5]:
            print(f"FAIL hint {k} row {rows[sel][j]}: {name} {a[j]} {b[j]} -> {c[j]}")
        print(f"  hint {k}: {int(sel.sum())} {name} rows, {len(bad)} wrong")
        checked += int(sel.sum())
        failures += len(bad)
    return checked, failures


def main():
    args = sys.argv[1:]
    if not args or len(args) % 2:
        sys.exit(__doc__)
    total_failures = 0
    for ir_path, fixed_path in zip(args[::2], args[1::2]):
        ir = json.load(open(ir_path))
        n = ir["air"]["num_rows"]
        ncols = len(ir["columns"]["fixed"])
        data = np.memmap(fixed_path, dtype="<u8", mode="r")
        if data.size != n * ncols:
            sys.exit(f"{fixed_path}: {data.size} values, expected {n} rows x {ncols} columns")
        value = evaluator(ir, data.reshape(n, ncols), n)
        print(f"{ir['air']['air']}: {n} rows, {ncols} fixed columns")

        results_checked = 0
        for k, opids, bus, tuple_ in proving_args(ir):
            watched = (opids & ({OPERATION_BUS} | set(NEVER)))
            if not watched:
                continue
            bus_values = value(bus)
            values = [value(o) for o in tuple_]
            if OPERATION_BUS in opids:
                rows = np.nonzero(bus_values == OPERATION_BUS)[0]
                checked, failures = check_results(k, rows, values)
                results_checked += checked
                total_failures += failures
            for table_bus, (table, ops) in NEVER.items():
                if table_bus not in opids:
                    continue
                rows = np.nonzero(bus_values == table_bus)[0]
                first = values[0][rows]
                for code, name in ops.items():
                    hits = rows[first == code]
                    for r in hits[:5]:
                        print(f"FAIL hint {k} row {r}: the {table} serves {name}")
                    total_failures += hits.size
                    print(f"  hint {k}: {rows.size} rows on the {table} (bus {table_bus}), "
                          f"{hits.size} with op {name}")
        if results_checked:
            print(f"  {results_checked} rows with a checked op on the operation bus")

    if total_failures:
        sys.exit(f"FAIL: {total_failures} rows contradict what the proofs assume")
    print("OK: the tables are what the proofs assume")


if __name__ == "__main__":
    main()
