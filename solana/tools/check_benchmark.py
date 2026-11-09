"""Reject missing measurements, runtime-limit violations and >5% CU regressions."""
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PATTERNS = {
    'valid_cu': r'invalid=false CU=(\d+)',
    'late_invalid_cu': r'invalid=true CU=(\d+)',
    'consumer_valid_cu': r'consumer valid CU=(\d+)',
    'consumer_invalid_cu': r'consumer invalid CU=(\d+)',
    'largest_upload_tx_bytes': r'largest_tx=(\d+)',
}

def parse(log):
    result = {}
    for key, pattern in PATTERNS.items():
        values = re.findall(pattern, log)
        if len(values) != 1:
            raise ValueError(f'expected exactly one {key} measurement')
        result[key] = int(values[0])
    return result


def validate(actual, baseline):
    for key in PATTERNS:
        value = actual[key]
        if value <= 0:
            raise ValueError(f'invalid measurement: {key}')
        if key.endswith('_cu'):
            if value >= 1_400_000:
                raise ValueError(f'{key} reached transaction ceiling')
            if value * 100 > baseline[key] * 105:
                raise ValueError(f'{key} regressed by more than 5%')
        elif value > 1232:
            raise ValueError('upload transaction exceeds packet limit')


if __name__ == '__main__':
    actual = parse(Path(sys.argv[1]).read_text())
    baseline = json.loads((ROOT/'benchmarks/integration.json').read_text())
    validate(actual, baseline['measurements'])
    paths = ['target/deploy/zisk_solana_program.so', 'target/deploy/zisk_consumer_example.so',
             'Cargo.lock', 'runtime-tests/Cargo.lock', 'package-lock.json']
    report = {'measurements': actual, 'hashes': {p: hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in paths}}
    (ROOT/'.cache').mkdir(exist_ok=True)
    (ROOT/'.cache/benchmark.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(actual))
