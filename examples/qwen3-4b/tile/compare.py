"""Regenerate the archived key-projection tile and verify both guest commitments."""

import argparse
import csv
import hashlib
import json
from pathlib import Path
import struct
import subprocess

import numpy as np

HERE = Path(__file__).resolve().parent
GOLD = bytes.fromhex("f7c1ceb9b4258f70eee60ef741e0a18e56aabe8ce331e25054aeb8ffd7d7cbf3")
FRAMES = {
    5: "bef989bb43574b95f36c681b270648e03b2780dfdb7831109a66fb845f92a47f",
    6: "c0ed3f1d0dcd7260c224ff0c32037b678403239f7b1a66894a95b995bbb7ab8a",
}


def tensor(model, index, name):
    path = model / index[name]
    with path.open("rb") as f:
        size, = struct.unpack("<Q", f.read(8))
        entry = json.loads(f.read(size))[name]
    if entry["dtype"] != "BF16":
        raise ValueError(f"expected BF16 tensor {name}")
    return np.memmap(path, dtype="<u2", mode="r", offset=8 + size + entry["data_offsets"][0],
                     shape=tuple(entry["shape"]))


def widen(bits):
    return (bits.astype("<u4") << 16).view("<f4")


def bf16(v):
    bits = v.view("<u4")
    return widen((bits + 0x7fff + ((bits >> 16) & 1)) >> 16)


def operands(model):
    request = json.loads((HERE.parent / "saved_request.json").read_text())
    tokens = request["prompt_ids"] + request["reply_ids"][:-1]
    index = json.loads((model / "model.safetensors.index.json").read_text())["weight_map"]
    x = widen(tensor(model, index, "model.embed_tokens.weight")[tokens])
    norm = widen(tensor(model, index, "model.layers.0.input_layernorm.weight"))
    weights = tensor(model, index, "model.layers.0.self_attn.k_proj.weight")[:64]

    # Pinned arm64 RMSNorm sums sixteen lanes per 256 columns, then folds four groups.
    squares = (x * x).reshape(39, 10, 16, 16)
    partial = np.zeros((39, 10, 16), dtype="<f4")
    for i in range(16):
        partial += squares[:, :, i, :]
    lanes = np.zeros((39, 16), dtype="<f4")
    for i in range(10):
        lanes += partial[:, i, :]
    groups = lanes[:, :4] + lanes[:, 4:8] + lanes[:, 8:12] + lanes[:, 12:]
    total = groups[:, 0] + groups[:, 1] + groups[:, 2] + groups[:, 3]
    inv = np.float32(1) / np.sqrt(total / np.float32(2560) + np.float32(1e-6))
    return bf16(bf16(x * inv[:, None]) * norm).tobytes() + weights.tobytes()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checkpoint", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--emu", default="ziskemu")
    parser.add_argument("--elf", default=HERE / "target/riscv64ima-zisk-zkvm-elf/release/qwen-key-tile")
    args = parser.parse_args()
    data = operands(args.checkpoint)
    args.output.mkdir(parents=True, exist_ok=False)
    for mode, expected in FRAMES.items():
        payload = struct.pack("<8s4I", b"PCISMPL1", mode, 39, 64, 2560) + data
        frame = struct.pack("<Q", len(payload)) + payload
        if hashlib.sha256(frame).hexdigest() != expected:
            raise ValueError(f"mode {mode} input differs from the measured tile")
        base = args.output / f"mode{mode}"
        stdin, public, stats, log = [base.with_suffix(ext) for ext in (".stdin", ".public", ".csv", ".log")]
        stdin.write_bytes(frame)
        with log.open("w") as f:
            subprocess.run([args.emu, "-X", "-e", args.elf, "-i", stdin, "-o", public,
                            "--save-stats", stats], stdout=f, stderr=subprocess.STDOUT, check=True)
        if public.read_bytes() != GOLD + bytes(224):
            raise ValueError(f"mode {mode} output differs from the frozen reference")
        with stats.open(newline="") as f:
            records = list(csv.reader(f))
        steps, = [int(r[1]) for r in records if r[:1] == ["STEPS"]]
        cost, = [int(r[2]) for r in records if r[:2] == ["COST", "TOTAL"]]
        print(f"mode {mode}: {steps:,} steps, {cost:,} cost, 2,496/2,496 outputs match", flush=True)


if __name__ == "__main__":
    main()
