#!/usr/bin/env python3
"""Build framed emulator inputs from a local Qwen3-4B-Instruct-2507 checkpoint.

    python3 prepare.py CHECKPOINT RECORDS_DIR [--request saved_request.json]

Supply config.json, model.safetensors.index.json and the three original BF16
safetensors shards. The new directory contains 112 stdin templates (~8 GB).
The runner replaces the zero-filled incoming activation in each layer/head
with the preceding guest's FP32 output. The embedding record is ready to run.
"""

import argparse
from contextlib import contextmanager
from dataclasses import dataclass
import hashlib
import json
import math
from pathlib import Path
import struct
import tempfile
import time

DIM, VOCAB, LAYERS, CONTEXT = 2560, 151936, 36, 40960
ROW_BYTES, DEPTH, SHARD_LOG2 = DIM * 2, 18, 11
SHARD_ROWS = 1 << SHARD_LOG2
TIED, NORM = "model.embed_tokens.weight", "model.norm.weight"
CHECKPOINT_ID = "b3a575752a697efff539af25aa7cab9127e13857cbf0e5e66fc08cd6edf0f87e"
SUITE = (
    b"qwen3-4b token-ids greedy-lowest-id; layer-record v2; child-receipt v3; "
    b"mac-arm64-bf16-sdpa-v1; torch cf30153c; transformers 5.15.1; libm 0.2.16; "
    b"rows-v1 depth 18; embed-record v1; head-shard-record v1 receipt v1; "
    b"zisk leaf proofs, publics = 32-byte digest then zeros; bundle v1; head shard log2 11"
)
# Enrollment order is lexicographic and is also the layer record's tensor order.
LAYER_SHAPES = {
    "input_layernorm": [DIM],
    "mlp.down_proj": [DIM, 9728],
    "mlp.gate_proj": [9728, DIM],
    "mlp.up_proj": [9728, DIM],
    "post_attention_layernorm": [DIM],
    "self_attn.k_norm": [128],
    "self_attn.k_proj": [1024, DIM],
    "self_attn.o_proj": [DIM, 4096],
    "self_attn.q_norm": [128],
    "self_attn.q_proj": [4096, DIM],
    "self_attn.v_proj": [1024, DIM],
}


def u32(*values):
    return struct.pack(f"<{len(values)}I", *values)


def tagged(domain, *parts):
    return hashlib.sha256(b"PCI/v1/" + domain + b"\0" + b"".join(parts)).digest()


def token_hash(domain, ids):
    return tagged(domain, struct.pack("<Q", len(ids)), u32(*ids))


def profile(t, r):
    return struct.pack("<8I2f2I16x", 1, DIM, 9728, 32, 8, 128, VOCAB, t,
                       5_000_000.0, 1e-6, 32, r)


def read_request(path):
    request = json.loads(path.read_bytes())
    prompt, reply = request["prompt_ids"], request["reply_ids"]
    for seq in (prompt, reply):
        if not isinstance(seq, list) or not seq or any(
            type(i) is not int or not 0 <= i < VOCAB for i in seq
        ):
            raise ValueError("expected nonempty arrays of in-vocabulary token IDs")
    limit = request["max_reply"]
    if type(limit) is not int or not len(reply) <= limit <= CONTEXT - len(prompt):
        raise ValueError("invalid reply limit")
    nonce = bytes.fromhex(request["nonce"])
    if len(nonce) != 32:
        raise ValueError("expected a 32-byte request nonce")
    return prompt, reply, limit, nonce


def chunks(stream, length):
    while length:
        block = stream.read(min(length, 1 << 20))
        if not block:
            raise ValueError(f"truncated file: {stream.name}")
        yield block
        length -= len(block)


@dataclass(frozen=True)
class Tensor:
    path: Path
    offset: int
    size: int
    digest: bytes

    def write(self, output, start=0, length=None):
        length = self.size - start if length is None else length
        if not 0 <= start <= start + length <= self.size:
            raise ValueError("checkpoint tensor range")
        with self.path.open("rb") as f:
            f.seek(self.offset + start)
            for block in chunks(f, length):
                output.write(block)


def enroll(model):
    shapes = {TIED: [VOCAB, DIM], NORM: [DIM]}
    stages = [{"kind": "embedding", "tensors": [TIED]}]
    for i in range(LAYERS):
        group = {f"model.layers.{i}.{name}.weight": shape for name, shape in LAYER_SHAPES.items()}
        stages.append({"kind": "layer", "index": i, "tensors": sorted(group)})
        shapes.update(group)
    stages.append({"kind": "head", "tensors": [NORM, TIED]})

    files = {}
    for name in ("config.json", "model.safetensors.index.json"):
        raw = (model / name).read_bytes()
        files[name] = hashlib.sha256(raw).hexdigest()
    mapping = json.loads(raw)["weight_map"]
    shards = {f"model-{i:05}-of-00003.safetensors" for i in range(1, 4)}
    if mapping.keys() != shapes.keys() or set(mapping.values()) != shards:
        raise ValueError("expected the original Qwen3-4B-Instruct-2507 BF16 tensor index")
    tensors, inventory = {}, {}
    for shard in sorted(shards):
        path = model / shard
        with path.open("rb") as f:
            prefix = f.read(8)
            size, = struct.unpack("<Q", prefix)
            if size > 16 << 20:
                raise ValueError(f"oversize safetensors header: {path}")
            header = f.read(size)
            whole = hashlib.sha256(prefix + header)
            entries = json.loads(header)
            entries.pop("__metadata__", None)
            if entries.keys() != {name for name in mapping if mapping[name] == shard}:
                raise ValueError(f"shard/index tensor mismatch: {shard}")
            for name, entry in sorted(entries.items(), key=lambda pair: pair[1]["data_offsets"]):
                start, end = entry["data_offsets"]
                offset = f.tell()
                length = end - start
                if (size + 8 + start != offset or entry["dtype"] != "BF16"
                        or entry["shape"] != shapes[name] or length != 2 * math.prod(shapes[name])):
                    raise ValueError(f"unexpected layout: {name}")
                tensor_hash = hashlib.sha256()
                for block in chunks(f, length):
                    whole.update(block)
                    tensor_hash.update(block)
                tensors[name] = Tensor(path, offset, length, tensor_hash.digest())
                inventory[name] = dict(dtype="BF16", shape=shapes[name], sha256=tensor_hash.hexdigest())
            if f.read(1):
                raise ValueError(f"trailing shard data: {shard}")
            files[shard] = whole.hexdigest()
    payload = json.dumps(dict(version=1, files=files, stages=stages, tensors=inventory),
                         sort_keys=True, separators=(",", ":")).encode()
    checkpoint = hashlib.sha256(b"PCI/checkpoint-inventory/v1\0" + payload).digest()
    if checkpoint.hex() != CHECKPOINT_ID:
        raise ValueError(f"unexpected publisher checkpoint: {checkpoint.hex()}")
    return checkpoint, tensors, stages


class RowTree:
    """SHA-256 row openings, with zero leaves beyond the end of the vocabulary."""

    def __init__(self, tied):
        with tied.path.open("rb") as f:
            f.seek(tied.offset)
            leaves = [tagged(b"row", u32(i), f.read(ROW_BYTES)) for i in range(VOCAB)]
        leaves.extend([bytes(32)] * ((1 << DEPTH) - VOCAB))
        self.levels = [leaves]
        while len(self.levels[-1]) > 1:
            children = iter(self.levels[-1])
            self.levels.append([tagged(b"row-node", a, b) for a, b in zip(children, children)])
        self.root = tagged(b"rows-root", u32(VOCAB, ROW_BYTES, DEPTH), self.levels[-1][0])

    def opening(self, level, index):
        siblings = []
        for nodes in self.levels[level:-1]:
            siblings.append(nodes[index ^ 1])
            index >>= 1
        return b"".join(siblings)


@contextmanager
def frame(path, header):
    # The payload length excludes the LE u64 prefix and the trailing ABI padding.
    with path.open("xb") as f:
        f.write(bytes(8))
        f.write(header)
        yield f
        size = f.tell() - 8
        f.write(bytes(-size % 8))
        f.seek(0)
        f.write(struct.pack("<Q", size))


def prepare(model, out, request):
    if out.exists():
        raise FileExistsError(f"choose a fresh records directory: {out}")
    prompt, reply, limit, nonce = read_request(request)
    consumed = prompt + reply[:-1]
    t, r = len(consumed), len(reply)
    checkpoint, tensors, stages = enroll(model)
    tied = tensors[TIED]
    tree = RowTree(tied)
    if tree.root.hex() != "97a9569bbf996b61e0973f4f171d6dfb00499c2e0abcd76f73ebe58920922571":
        raise ValueError("unexpected tied embedding row commitment")
    # Model identity binds whole tensor digests for every role. The embedding
    # and head records additionally open rows under the separate row commitment.
    weights = []
    layers = stages[1:-1]
    for stage in stages:
        kind = {"embedding": 0, "layer": 1, "head": 2}[stage["kind"]]
        names = stage["tensors"]
        weights.append(tagged(b"stage-weights", bytes([kind]), u32(stage.get("index", 0), len(names)),
                              *(tensors[name].digest for name in names)))
    model_id = tagged(b"model/qwen3-4b/mac-arm64-bf16-sdpa-v1", checkpoint, profile(0, 0),
                      u32(len(weights)), *weights)
    suite_id = tagged(b"suite", SUITE)
    request_id = tagged(b"request/greedy-lowest-id", model_id, suite_id, nonce,
                        token_hash(b"prompt", prompt), u32(limit, CONTEXT),
                        token_hash(b"stop-ids", [151643, 151645]))
    ids = model_id + suite_id + request_id
    out.parent.mkdir(parents=True, exist_ok=True)
    # Publish only after all frames close successfully. An interrupted build can be retried.
    with tempfile.TemporaryDirectory(dir=out.parent, prefix=".qwen-prepare-") as scratch:
        records = Path(scratch) / "records"
        records.mkdir()
        with frame(records / "embed.stdin", b"PCIEMBD1" + ids + u32(t) + u32(*consumed)) as f:
            for token in consumed:
                tied.write(f, token * ROW_BYTES, ROW_BYTES)
                f.write(tree.opening(0, token))
        for i, stage in enumerate(layers):
            header = b"PCILAYR1" + ids + u32(i, t) + profile(t, r)
            with frame(records / f"layer{i:02}.v1.stdin", header) as f:
                f.seek(4 * DIM * t, 1)  # sparse zero activation for the runner
                for name in stage["tensors"]:
                    tensors[name].write(f)
        for i, first in enumerate(range(0, VOCAB, SHARD_ROWS)):
            header = b"PCIHEAD1" + ids + u32(SHARD_LOG2, i, t, r)
            with frame(records / f"head-{i:05}.stdin", header) as f:
                f.seek(4 * DIM * t, 1)
                tensors[NORM].write(f)
                tied.write(f, first * ROW_BYTES, min(SHARD_ROWS, VOCAB - first) * ROW_BYTES)
                f.write(tree.opening(SHARD_LOG2, i))
        records.rename(out)
    return request_id.hex()


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("checkpoint", type=Path)
    parser.add_argument("records_dir", type=Path)
    parser.add_argument("--request", type=Path, default=Path(__file__).with_name("saved_request.json"))
    args = parser.parse_args()
    started = time.monotonic()
    request_id = prepare(args.checkpoint, args.records_dir, args.request)
    print(f"Prepared 112 records in {time.monotonic() - started:.2f}s at {args.records_dir}")
    print(f"Request ID {request_id}")


if __name__ == "__main__":
    main()
