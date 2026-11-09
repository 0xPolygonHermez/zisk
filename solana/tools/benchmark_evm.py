"""Measure the unchanged release verifier on an ephemeral, loopback-only Anvil."""
import hashlib
import json
from pathlib import Path
import socket
import subprocess
import time
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def main():
    subprocess.run(['forge', 'build'], cwd=ROOT, check=True)
    artifact = json.loads((ROOT / 'out/ZiskVerifier.sol/ZiskVerifier.json').read_text())
    fixture = json.loads((ROOT / 'fixtures/blake3.json').read_text())
    proof = bytes.fromhex(fixture['proofBytes'].removeprefix('0x'))
    assert len(proof) == 6752
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    url = f'http://127.0.0.1:{port}'

    def rpc(method, *params):
        body = json.dumps(dict(jsonrpc='2.0', id=1, method=method, params=params)).encode()
        with urlopen(Request(url, body, {'Content-Type': 'application/json'}), timeout=30) as response:
            result = json.load(response)
        if 'error' in result:
            raise RuntimeError(result['error'])
        return result['result']

    node = subprocess.Popen(
        ['anvil', '--silent', '--host', '127.0.0.1', '--port', str(port),
         '--chain-id', '31337', '--hardfork', 'prague', '--base-fee', '0'],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(100):
            try:
                accounts = rpc('eth_accounts')
                break
            except OSError:
                if node.poll() is not None:
                    raise RuntimeError('Local Anvil failed to start')
                time.sleep(0.05)
        else:
            raise RuntimeError('Local Anvil did not become ready')
        assert rpc('eth_chainId') == hex(31337)

        def transact(data, to=None):
            tx = {'from': accounts[0], 'data': data, 'gas': hex(10_000_000), 'gasPrice': hex(10**9)}
            if to is not None:
                tx['to'] = to
            tx_hash = rpc('eth_sendTransaction', tx)
            for _ in range(100):
                receipt = rpc('eth_getTransactionReceipt', tx_hash)
                if receipt is not None:
                    return receipt
                time.sleep(0.05)
            raise RuntimeError('Local transaction was not mined')

        deployment = transact(artifact['bytecode']['object'])
        assert int(deployment['status'], 16) == 1
        address = deployment['contractAddress']
        measurements = {}
        signature = 'verifySnarkProof(bytes32,bytes32,bytes,bytes)'
        args = [fixture[k] for k in ['programVK', 'rootCVadcopFinal', 'publicValues']]
        valid = command('cast', 'calldata', signature, *args, fixture['proofBytes'])
        changed = proof[:512] + proof[:64] + proof[576:]
        invalid = command('cast', 'calldata', signature, *args, '0x' + changed.hex())
        words = '[' + ','.join('0x' + proof[i:i+32].hex() for i in range(0, len(proof), 32)) + ']'
        public = json.loads((ROOT / 'fixtures/publics.json').read_text())[0]
        core = command('cast', 'calldata', 'verifyProof(bytes32[211],uint256[1])', words, f'[{public}]')
        assert int(rpc('eth_call', dict(to=address, data=core), 'latest'), 16) == 1
        assert rpc('eth_call', dict(to=address, data=valid), 'latest') == '0x'
        for name, data, status in [('valid_wrapper', valid, 1), ('late_invalid_wrapper', invalid, 0), ('valid_core', core, 1)]:
            receipt = transact(data, address)
            assert int(receipt['status'], 16) == status, name
            raw = bytes.fromhex(data[2:])
            zeros = raw.count(0)
            intrinsic = 21_000 + zeros * 4 + (len(raw) - zeros) * 16
            floor = 21_000 + 10 * (zeros + 4 * (len(raw) - zeros))
            gas = int(receipt['gasUsed'], 16)
            assert gas > floor, 'Calldata floor binds: subtraction would not isolate execution'
            measurements[name] = dict(transaction_gas=gas, intrinsic_gas=intrinsic,
                                      execution_gas=gas-intrinsic, calldata_bytes=len(raw),
                                      calldata_zero_bytes=zeros, calldata_floor_gas=floor, status=status)
        report = dict(runtime=command('anvil', '--version').splitlines()[0], hardfork='prague',
                      compiler=artifact['metadata']['compiler'],
                      optimizer=artifact['metadata']['settings']['optimizer'],
                      proof_sha256=hashlib.sha256(proof).hexdigest(),
                      runtime_bytecode_sha256=hashlib.sha256(bytes.fromhex(artifact['deployedBytecode']['object'][2:])).hexdigest(),
                      runtime_bytecode_bytes=len(bytes.fromhex(artifact['deployedBytecode']['object'][2:])),
                      deployment_gas=int(deployment['gasUsed'], 16), measurements=measurements)
        (ROOT / 'benchmarks/evm.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(report, indent=2))
    finally:
        node.terminate()
        node.wait(timeout=10)


if __name__ == '__main__':
    main()
