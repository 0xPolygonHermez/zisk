#!/usr/bin/env python3
"""Encode the single-batch, non-Stylus recorded transfer for native replay."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import struct


MASK = (1 << 64) - 1
ROTATION = (0, 1, 62, 28, 27, 36, 44, 6, 55, 20, 3, 10, 43, 25, 39,
            41, 45, 15, 21, 8, 18, 2, 61, 56, 14)
ROUND_CONSTANT = (1, 0x8082, 0x800000000000808A, 0x8000000080008000, 0x808B,
                  0x80000001, 0x8000000080008081, 0x8000000000008009, 0x8A, 0x88,
                  0x80008009, 0x8000000A, 0x8000808B, 0x800000000000008B,
                  0x8000000000008089, 0x8000000000008003, 0x8000000000008002,
                  0x8000000000000080, 0x800A, 0x800000008000000A,
                  0x8000000080008081, 0x8000000000008080, 0x80000001,
                  0x8000000080008008)


def keccak(data):
    """Test oracle only; the VM authenticates with its native hash implementation."""
    def rotate(value, bits):
        return ((value << bits) | (value >> ((64 - bits) % 64))) & MASK

    padded = bytearray(data)
    padded.append(1)
    padded.extend(bytes((-len(padded)) % 136))
    padded[-1] |= 0x80
    lanes = [0] * 25
    for offset in range(0, len(padded), 136):
        for index, word in enumerate(struct.unpack("<17Q", padded[offset:offset + 136])):
            lanes[index] ^= word
        for constant in ROUND_CONSTANT:
            parity = [lanes[x] ^ lanes[x+5] ^ lanes[x+10] ^ lanes[x+15] ^ lanes[x+20]
                      for x in range(5)]
            theta = [parity[(x-1) % 5] ^ rotate(parity[(x+1) % 5], 1) for x in range(5)]
            moved = [0] * 25
            for y in range(5):
                for x in range(5):
                    moved[y + 5*((2*x + 3*y) % 5)] = rotate(lanes[x+5*y] ^ theta[x], ROTATION[x+5*y])
            for y in range(5):
                for x in range(5):
                    lanes[x+5*y] = moved[x+5*y] ^ ((~moved[(x+1) % 5+5*y]) & moved[(x+2) % 5+5*y])
            lanes[0] ^= constant
    return struct.pack("<4Q", *lanes[:4])


def unsigned(value, bits):
    if type(value) is not int or not 0 <= value < 1 << bits:
        raise ValueError(f"expected u{bits}, got {value!r}")
    return value


def bytes32(value):
    if not isinstance(value, str) or not value.startswith("0x") or len(value) != 66:
        raise ValueError("expected 0x-prefixed bytes32")
    return bytes.fromhex(value[2:])


def pack_state(state):
    return (bytes32(state["BlockHash"]) + bytes32(state["SendRoot"]) +
            struct.pack("<QQ", unsigned(state["Batch"], 64), unsigned(state["PosInBatch"], 64)))


def encode_request(request, wasm):
    # shortcut: single complete non-Stylus batch, extend only with observed consumed-window data.
    if request["HasDelayedMsg"] is not False or request.get("DelayedMsgB64", ""):
        raise ValueError("this fixture encoder supports no consumed delayed message")
    if any(request["UserWasms"].values()):
        raise ValueError("Stylus modules are unsupported")
    start, end = pack_state(request["StartState"]), pack_state(request["ExpectedEndState"])
    seq_start = unsigned(request["StartState"]["Batch"], 64)
    if (request["StartState"]["PosInBatch"] != 0 or request["ExpectedEndState"]["PosInBatch"] != 0
            or seq_start == MASK or request["ExpectedEndState"]["Batch"] != seq_start + 1):
        raise ValueError("expected one complete batch with zero start/end position")
    batches = request["BatchInfo"]
    if len(batches) != 1 or unsigned(batches[0]["Number"], 64) != seq_start:
        raise ValueError("expected exactly the starting sequencer batch")
    message = base64.b64decode(batches[0]["DataB64"], validate=True)
    if len(message) < 40:
        raise ValueError("short sequencer header")
    delayed_start = int.from_bytes(message[32:40], "big")
    preimages = []
    for raw_type, entries in request["PreimagesB64"].items():
        ty = unsigned(int(raw_type), 8)
        for raw_key, raw_value in entries.items():
            key = base64.b64decode(raw_key, validate=True)
            value = base64.b64decode(raw_value, validate=True)
            if len(key) != 32:
                raise ValueError("preimage key must contain 32 bytes")
            unsigned(len(value), 32)
            preimages.append((ty, key, value))
    preimages.sort()
    if len(preimages) > 8192 or len({(ty, key) for ty, key, _ in preimages}) != len(preimages):
        raise ValueError("duplicate preimage or preimage bound exceeded")
    zero = bytes(32)
    payload = bytearray(b"NZKWIT01" + start + zero + zero +
                        struct.pack("<QQIII", seq_start, delayed_start, len(preimages), 1, 0))
    for ty, key, value in preimages:
        payload.extend(bytes([ty]) + key + struct.pack("<I", len(value)) + value)
    payload.extend(struct.pack("<QI", seq_start, unsigned(len(message), 32)) + message)
    if len(payload) > 64 * 1024 * 1024:
        raise ValueError("witness exceeds 64 MiB")
    seq_acc = keccak(zero + keccak(message) + zero)
    common = start + end
    seq_claim = keccak(b"ARBZSEQ1" + common + zero + seq_acc + struct.pack("<QQ", seq_start, seq_start+1))
    delayed_claim = keccak(b"ARBZDEL1" + common + zero + zero + struct.pack("<QQ", delayed_start, delayed_start))
    expected = hashlib.sha256(wasm).digest() + common + seq_claim + delayed_claim
    return bytes(payload), expected


def frame(payload):
    return struct.pack("<Q", len(payload)) + payload + bytes((-len(payload)) % 8)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def self_test():
    state = {"BlockHash": "0x" + "00" * 32, "SendRoot": "0x" + "00" * 32,
             "Batch": 0, "PosInBatch": 0}
    end = dict(state, Batch=1)
    request = {"StartState": state, "ExpectedEndState": end,
               "PreimagesB64": {"0": {base64.b64encode(b"a" * 32).decode(): "YWJj"}},
               "BatchInfo": [{"Number": 0, "DataB64": base64.b64encode(bytes(40)).decode()}],
               "HasDelayedMsg": False, "DelayedMsgB64": "", "UserWasms": {"wasm": {}}}
    payload, expected = encode_request(request, b"wasm")
    assert payload[:8] == b"NZKWIT01", "encoder must emit witness magic"
    assert len(payload) == 180 + 37 + 3 + 12 + 40
    assert payload[168:180] == struct.pack("<III", 1, 1, 0)
    assert len(expected) == 256 and expected[:32] == hashlib.sha256(b"wasm").digest()
    assert keccak(b"").hex() == "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
    assert keccak(b"abc").hex() == "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45"
    assert len(frame(payload)) % 8 == 0
    assert struct.unpack("<Q", frame(payload)[:8])[0] == len(payload)
    altered = dict(request, ExpectedEndState=dict(end, BlockHash="0x" + "01" * 32))
    altered_payload, altered_expected = encode_request(altered, b"wasm")
    assert altered_payload == payload and altered_expected != expected
    for bad in [dict(request, HasDelayedMsg=True), dict(request, BatchInfo=[]),
                dict(request, BatchInfo=[{"Number": 0, "DataB64": "AA=="}]),
                dict(request, UserWasms={"wasm": {"module": "x"}}),
                dict(request, ExpectedEndState=dict(end, Batch=2))]:
        try:
            encode_request(bad, b"wasm")
        except ValueError:
            pass
        else:
            raise AssertionError("invalid fixture accepted")
    try:
        json.loads('{"x": 1, "x": 2}', object_pairs_hook=unique_object)
    except ValueError:
        pass
    else:
        raise AssertionError("duplicate JSON keys accepted")
    print("prepare-witness self-test passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--block-file", type=Path)
    parser.add_argument("--replay-wasm", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if not all((args.block_file, args.replay_wasm, args.output, args.expected)):
        parser.error("--block-file, --replay-wasm, --output and --expected are required")
    request = json.loads(args.block_file.read_text(), object_pairs_hook=unique_object)
    payload, expected = encode_request(request, args.replay_wasm.read_bytes())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.expected.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(frame(payload))
    args.expected.write_bytes(expected)
    print(f"wrote {args.output} ({len(payload)} witness bytes) and {args.expected} (256 bytes)")


if __name__ == "__main__":
    main()
