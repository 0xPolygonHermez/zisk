"""Compare prepared templates with independent frozen emulator stdin fixtures.

    python3 test_prepare.py RECORDS_DIR FROZEN_RUN

FROZEN_RUN contains wide/{embed,head-*}.stdin and inputs/layer*.v1.stdin.
Checks all bytes, allowing only the runner's zero-filled activation slots.
"""
import argparse
from pathlib import Path
import struct


def compare_bytes(actual, expected, length):
    while length:
        size = min(length, 1 << 20)
        pos = actual.tell()
        left, right = actual.read(size), expected.read(size)
        if left != right or len(left) != size:
            raise ValueError(f"{actual.name}: differing or truncated block at offset {pos}")
        length -= size


def compare(records, frozen):
    layers = [f"layer{i:02}.v1.stdin" for i in range(36)]
    heads = [f"head-{i:05}.stdin" for i in range(75)]
    names = ["embed.stdin", *layers, *heads]
    if {p.name for p in records.iterdir()} != set(names):
        raise ValueError("expected exactly the 112 stage records")
    total = 0
    for name in names:
        layer = name.startswith("layer")
        source = frozen / ("inputs" if layer else "wide") / name
        template = records / name
        with template.open("rb") as actual, source.open("rb") as expected:
            payload, = struct.unpack("<Q", expected.read(8))
            expected.seek(0)
            size = 8 + payload + (-payload % 8)
            if template.stat().st_size != size or source.stat().st_size != size:
                raise ValueError(f"{name}: frame length")
            if name != "embed.stdin":
                compare_bytes(actual, expected, 8 + (176 if layer else 120))
                carry_size = 39 * 2560 * 4
                if actual.read(carry_size) != bytes(carry_size):
                    raise ValueError(f"{name}: nonzero or truncated activation slot")
                expected.seek(carry_size, 1)
            compare_bytes(actual, expected, size - actual.tell())
            total += size
    print(f"Verified {len(names)} framed records ({total:,} bytes) against the frozen run")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("records_dir", type=Path)
    parser.add_argument("frozen_run", type=Path)
    args = parser.parse_args()
    compare(args.records_dir, args.frozen_run)
