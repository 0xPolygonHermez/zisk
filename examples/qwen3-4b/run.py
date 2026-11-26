"""Replay one teacher-forced Qwen3-4B transcript, carrying real emulator outputs between jobs."""

import argparse
import csv
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile

import numpy as np

from prepare import CONTEXT, DIM, LAYERS, SHARD_LOG2, SHARD_ROWS, VOCAB, read_request, tagged, token_hash, u32
LAYER_HEADER = 176
HEAD_HEADER = 120


def with_activations(record, destination, header_size, expected_prefix, boundary):
    """Replace the reserved activation bytes. Retain the header, weights and openings."""
    with record.open("rb") as source, destination.open("wb") as out:
        header = source.read(8 + header_size)
        if len(header) != 8 + header_size:
            raise ValueError(f"short header in {record}")
        if not header[8:].startswith(expected_prefix):
            raise ValueError(f"wrong request, stage or dimensions in {record}")
        payload_size, = struct.unpack_from("<Q", header)
        if record.stat().st_size != 8 + ((payload_size + 7) // 8) * 8:
            raise ValueError(f"invalid length prefix in {record}")
        if header_size + len(boundary) > payload_size:
            raise ValueError(f"activation exceeds payload in {record}")
        out.write(header)
        out.write(boundary)
        source.seek(len(boundary), 1)
        shutil.copyfileobj(source, out, length=1024 * 1024)


def read_stats(path):
    with path.open(newline="") as source:
        rows = list(csv.reader(source))
    steps, = [int(r[1]) for r in rows if r[:1] == ["STEPS"]]
    cost, = [int(r[2]) for r in rows if r[:2] == ["COST", "TOTAL"]]
    return steps, cost


def run(args):
    prompt, reply, limit, nonce = read_request(args.request)
    predictions = len(reply)
    consumed = prompt + reply[:-1]
    positions = len(consumed)
    embed = args.records / "embed.stdin"
    with embed.open("rb") as stream:
        header = stream.read(116)
        if len(header) != 116 or header[8:16] != b"PCIEMBD1":
            raise ValueError("invalid embedding record")
        ids = header[16:112]
        wanted = tagged(b"request/greedy-lowest-id", ids[:32], ids[32:64], nonce,
                        token_hash(b"prompt", prompt), u32(limit, CONTEXT),
                        token_hash(b"stop-ids", [151643, 151645]))
        if (ids[64:] != wanted or header[112:] != u32(positions)
                or stream.read(4 * positions) != u32(*consumed)):
            raise ValueError("prepared embedding does not match the supplied transcript")
    boundary_size = positions * DIM * 4
    args.output.mkdir(parents=True, exist_ok=False)
    measurements = []

    def execute(name, stage_input, expected_bytes):
        prefix = args.output / name
        capture = prefix.with_suffix(".bin")
        stats = prefix.with_suffix(".csv")
        public = prefix.with_suffix(".public")
        command = [args.emulator, "-X", "-m", "--max-input-mem", "268435456",
                   "-n", "1000000000000", "-e", args.elf, "-i", stage_input,
                   "-o", public, "--save-stats", stats, "--dump-guest-buffer", capture]
        with prefix.with_suffix(".log").open("w") as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
        data = capture.read_bytes()
        if len(data) != expected_bytes:
            raise ValueError(f"{name}: expected {expected_bytes} output bytes, got {len(data)}")
        steps, cost = read_stats(stats)
        measurements.append(dict(stage=name, steps=steps, cost=cost))
        print(f"{name}: {steps:,} steps, {cost:,} cost", flush=True)
        return data

    boundary = execute("embed", embed, boundary_size)
    logits = np.empty((predictions, VOCAB), dtype="<f4")
    with tempfile.TemporaryDirectory(dir=args.output) as tmp:
        stage_input = Path(tmp) / "stage.stdin"
        for index in range(LAYERS):
            name = f"layer{index:02}"
            with_activations(args.records / f"{name}.v1.stdin", stage_input, LAYER_HEADER,
                             b"PCILAYR1" + ids + u32(index, positions), boundary)
            boundary = execute(name, stage_input, boundary_size)

        for shard, first in enumerate(range(0, VOCAB, SHARD_ROWS)):
            name = f"head-{shard:05}"
            rows = min(SHARD_ROWS, VOCAB - first)
            with_activations(args.records / f"{name}.stdin", stage_input, HEAD_HEADER,
                             b"PCIHEAD1" + ids + u32(SHARD_LOG2, shard, positions, predictions), boundary)
            raw = execute(name, stage_input, predictions * rows * 4)
            logits[:, first:first + rows] = np.frombuffer(raw, dtype="<f4").reshape(predictions, rows)

    if not np.isfinite(logits).all():
        raise ValueError("non-finite logit")
    greedy_ids = logits.argmax(axis=1).tolist()
    result = dict(stages=measurements, total_steps=sum(m["steps"] for m in measurements),
                  total_cost=sum(m["cost"] for m in measurements), greedy_ids=greedy_ids,
                  matching_ids=sum(a == b for a, b in zip(greedy_ids, reply)),
                  logits_sha256=hashlib.sha256(logits.tobytes()).hexdigest())
    if args.reference_logits:
        reference = np.fromfile(args.reference_logits, dtype="<f4").reshape(logits.shape)
        result.update(identical_logits=int((logits.view("<u4") == reference.view("<u4")).sum()),
                      max_abs_logit_difference=float(np.abs(logits.astype(float) - reference).max()))
    (args.output / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    print(f"Full request: {result['total_steps']:,} steps, {result['total_cost']:,} cost")
    print(f"Greedy predictions matching the saved reply: {result['matching_ids']}/{predictions}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--records", required=True, type=Path)
    parser.add_argument("--elf", required=True, type=Path)
    parser.add_argument("--request", type=Path, default=Path(__file__).with_name("saved_request.json"))
    parser.add_argument("--emulator", default="ziskemu")
    parser.add_argument("--output", required=True, type=Path, help="new directory for logs and outputs")
    parser.add_argument("--reference-logits", type=Path, help="optional pinned FP32 little-endian logits")
    run(parser.parse_args())
