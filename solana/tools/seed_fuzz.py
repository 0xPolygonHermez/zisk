"""Reconstruct public seeds; generated/discovered fuzz corpora stay outside Git."""
import json
from pathlib import Path
root = Path(__file__).resolve().parents[1]
fixture = json.loads((root/'fixtures/blake3.json').read_text())
proof = b''.join(bytes.fromhex(fixture[k].removeprefix('0x')) for k in ['programVK','publicValues','proofBytes'])
sequence = [(0,0,0,0,0,0)] + [(1,0,0,0,7,0)]*7 + [
    (1,0,0,1,7,0), (2,0,0,0,0,0), (3,0,0,0,0,0),
    (4,0,0,0,0,0), (0,16,0,0,0,0), (6,0,0,0,0,0)]
for target, name, data in [('verify','release-proof',proof), ('buffer_state','lifecycle',bytes(x for event in sequence for x in event))]:
    directory = root/'fuzz/corpus'/target
    directory.mkdir(parents=True,exist_ok=True)
    (directory/name).write_bytes(data)
