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

Five checks:

1. **Results on the operation bus (5000).** Every row whose op is a checked
   opcode carries the op's result (the FROPS tables of precomputed results):

       add (0x0a):  c = a + b mod 2^64,  flag = 0
       ltu (0x06):  c = flag = [a < b], unsigned
       eq  (0x09):  c = flag = [a = b]
       and (0x0e):  c = a & b,  flag = 0

   with every limb below 2^32.

2. **Ops a table never serves.** The Arith table (bus 331) and the
   BinaryExtension table (bus 124) are where Arith and BinaryExtension look
   their `op` column up, as the first element of the tuple. A row on one of
   those buses never has a modeled op there, which is what rules the two AIRs
   out as provers of one.

3. **The Binary table's rows for the modeled ops (bus 125).** Binary computes
   each op byte by byte, each byte a lookup of `(pos, op, a, b, cin, c, flags)`.
   Every row has an op below 2^10 (the widest is SEXT_FF, 0x201), so never one
   of the negative `b_op`s a 32-bit form of a modeled op would need; and every
   row of a modeled op has `pos` 0 or 1 (middle or last byte), bytes below 256,
   `cin` 0 or 1, and computes its byte:

       add  middle: c + 256*flags = a + b + cin, flags 0 or 1
            last:   c = (a + b + cin) mod 256, flags 0 or 64 (c's sign bit)
       ltu  c = 0, flags = 1 if a < b, cin if a = b, else 0
       eq   c = 0, flags = 0 if cin = 0 and a = b, else 1; the last byte inverted
       and  c = a & b, flags = 0

4. **Fixed ops on the memory bus (10).** A precompile's memory accesses send
   an op that is a sum of its clock columns plus 1: fixed data, which
   `lean/Proofs/MemBus.lean` takes as `FixedOp`. Every argument on bus 10
   whose op reads only constants and fixed columns (shifted or not) has op 1
   or 2 at every row — never 3, a register's. When `build/membus.json` (from
   `gen_membus.py`) is present, the arguments found must be as many, per AIR,
   as the ones it proves with `FixedOp`.

5. **Main's row counter.** The fixed column `SEGMENT_STEP` of Main is the row
   index, 0 to N-1, which `lean/Proofs/MainRegs.lean` takes as part of
   `MainSegOk` (a step's memory step counts from it).

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
    0x06: ("ltu", lambda a, b: (int(a < b), int(a < b))),
    0x09: ("eq", lambda a, b: (int(a == b), int(a == b))),
    0x0E: ("and", lambda a, b: (a & b, 0)),
}

BINARY_TABLE = 125

# table bus -> (name, ops whose first element it must never be)
MODELED = {code: name for code, (name, _) in SEMANTICS.items()}
NEVER = {
    331: ("Arith table", MODELED),
    124: ("BinaryExtension table", MODELED),
}


MEMORY_BUS = 10


class NotFixed(Exception):
    """An operand that is not fixed data: a witness column, a challenge."""


def bus_args(ir):
    """Every bus argument, either side: (hint index, opids, tuple)."""
    out = []
    for k, hint in enumerate(ir["hints"]):
        if hint["name"] != "gsum_debug_data":
            continue
        fields = {f.get("name"): f["value"] for f in hint["fields"][0]["value"]["value"]}
        opids = {int(e["value"]["value"]["value"]) for e in fields["opids"]["value"]}
        tuple_ = [e["value"]["value"] for e in fields["expressions"]["value"]]
        out.append((k, opids, tuple_))
    return out


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

    Expressions are evaluated over the pool, mod P. Only constants and fixed
    columns can appear, a shifted column read cyclically; anything else raises
    `NotFixed`."""
    cache = {}

    def operand(o):
        kind = o["kind"]
        if kind == "const":
            return np.full(n, int(o["value"]) % P, dtype=np.uint64)
        if kind == "fixed":
            col = np.asarray(columns[:, o["col"]]) % np.uint64(P)
            off = o.get("offset", 0)
            return np.roll(col, -off) if off else col
        if kind == "expr":
            if o["idx"] not in cache:
                cache[o["idx"]] = node(ir["expressions"][o["idx"]])
            return cache[o["idx"]]
        raise NotFixed(f"operand {o} is not fixed data")

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


def binary_row_ok(op, pos, a, b, cin, c, f):
    """Whether one Binary-table row of a modeled op computes its byte."""
    if not (a < 256 and b < 256 and c < 256 and cin in (0, 1) and pos in (0, 1)):
        return False
    last = pos == 1
    if op == 0x0A:
        if last:
            return f in (0, 64) and c == (a + b + cin) % 256
        return f in (0, 1) and c + 256 * f == a + b + cin
    if op == 0x06:
        return c == 0 and f == (1 if a < b else cin if a == b else 0)
    if op == 0x09:
        neq = 0 if (cin == 0 and a == b) else 1
        return c == 0 and f == (1 - neq if last else neq)
    if op == 0x0E:
        return c == (a & b) and f == 0
    return True


def check_binary_rows(k, rows, values):
    """Check 3 on one argument's rows on the Binary table; returns failures."""
    if len(values) != 7:
        sys.exit(f"hint {k}: a {len(values)}-element tuple, expected (pos, op, a, b, cin, c, flags)")
    pos, op, a, b, cin, c, flags = (v[rows] for v in values)
    failures = 0
    wide = int((op >= 1024).sum())
    if wide:
        print(f"FAIL hint {k}: {wide} rows with an op of 2^10 or more")
        failures += wide
    counts = []
    for code, (name, _) in SEMANTICS.items():
        sel = op == code
        n = int(sel.sum())
        if not n:
            continue
        cols = [x[sel].astype(object) for x in (pos, a, b, cin, c, flags)]
        bad = 0
        for r, (p_, x, y, ci, z, f) in enumerate(zip(*cols)):
            if not binary_row_ok(code, p_, x, y, ci, z, f):
                if bad < 5:
                    print(f"FAIL hint {k}: {name} row {rows[sel][r]} = {(p_, x, y, ci, z, f)}")
                bad += 1
        failures += bad
        counts.append(f"{n} {name} rows, {bad} wrong")
    print(f"  hint {k}: {rows.size} rows on the Binary table, ops all below 2^10: {wide == 0}; "
          + ("; ".join(counts) if counts else "no modeled op"))
    return failures


def check_fixed_ops(k, op):
    """Check 4 on one argument's op, at every row; returns failures."""
    bad = int(((op != 1) & (op != 2)).sum())
    for r in np.nonzero((op != 1) & (op != 2))[0][:5]:
        print(f"FAIL hint {k} row {r}: a fixed op of {int(op[r])} on the memory bus")
    print(f"  hint {k}: a fixed op on the memory bus, 1 or 2 at all {op.size} rows: {bad == 0}")
    return bad


def is_fixed_op(ir, o):
    """Whether an operand reads fixed columns and nothing but constants besides."""
    seen, fixed = set(), [False]

    def walk(x):
        kind = x["kind"]
        if kind == "fixed":
            fixed[0] = True
            return True
        if kind == "const":
            return True
        if kind == "expr":
            if x["idx"] in seen:
                return True
            seen.add(x["idx"])
            e = ir["expressions"][x["idx"]]
            return all(walk(e[side]) for side in ("lhs", "rhs", "value") if side in e)
        return False

    return walk(o) and fixed[0]


def main():
    args = sys.argv[1:]
    if not args or len(args) % 2:
        sys.exit(__doc__)
    total_failures = 0
    expected = {}
    try:
        for e in json.load(open("build/membus.json")):
            if e["kind"] == "fixed":
                expected[e["air"]] = expected.get(e["air"], 0) + 1
    except FileNotFoundError:
        expected = None
    for ir_path, fixed_path in zip(args[::2], args[1::2]):
        ir = json.load(open(ir_path))
        n = ir["air"]["num_rows"]
        ncols = len(ir["columns"]["fixed"])
        data = np.memmap(fixed_path, dtype="<u8", mode="r")
        if data.size != n * ncols:
            sys.exit(f"{fixed_path}: {data.size} values, expected {n} rows x {ncols} columns")
        value = evaluator(ir, data.reshape(n, ncols), n)
        print(f"{ir['air']['air']}: {n} rows, {ncols} fixed columns")

        names = [c["name"] for c in ir["columns"]["fixed"]]
        if "Main.SEGMENT_STEP" in names:
            col = np.asarray(data.reshape(n, ncols)[:, names.index("Main.SEGMENT_STEP")])
            bad = int((col != np.arange(n, dtype=np.uint64)).sum())
            print(f"  Main.SEGMENT_STEP is the row index at all {n} rows: {bad == 0}")
            total_failures += bad

        fixed_ops = 0
        for k, opids, tuple_ in bus_args(ir):
            if MEMORY_BUS in opids and tuple_ and is_fixed_op(ir, tuple_[0]):
                total_failures += check_fixed_ops(k, value(tuple_[0]))
                fixed_ops += 1
        air = ir["air"]["air"]
        if expected is not None and (fixed_ops or air in expected):
            if fixed_ops != expected.get(air, 0):
                print(f"FAIL {air}: {fixed_ops} fixed ops on the memory bus, "
                      f"MemBus.lean takes {expected.get(air, 0)} as FixedOp")
                total_failures += 1

        results_checked = 0
        # Checks 1-3 are about the virtual tables; another AIR's tuples are witness data.
        table_args = proving_args(ir) if air.startswith("VirtualTable") else []
        for k, opids, bus, tuple_ in table_args:
            watched = (opids & ({OPERATION_BUS, BINARY_TABLE} | set(NEVER)))
            if not watched:
                continue
            bus_values = value(bus)
            values = [value(o) for o in tuple_]
            if OPERATION_BUS in opids:
                rows = np.nonzero(bus_values == OPERATION_BUS)[0]
                checked, failures = check_results(k, rows, values)
                results_checked += checked
                total_failures += failures
            if BINARY_TABLE in opids:
                rows = np.nonzero(bus_values == BINARY_TABLE)[0]
                total_failures += check_binary_rows(k, rows, values)
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
