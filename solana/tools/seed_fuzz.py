"""Reconstruct the public proof seed; discovered fuzz inputs stay outside Git."""
import json
from pathlib import Path
root = Path(__file__).resolve().parents[1]
fixture = json.loads((root/'fixtures/blake3.json').read_text())
proof = b''.join(bytes.fromhex(fixture[k].removeprefix('0x')) for k in ['programVK','publicValues','proofBytes'])
directory = root/'fuzz/corpus/verify'
directory.mkdir(parents=True,exist_ok=True)
(directory/'release-proof').write_bytes(proof)
