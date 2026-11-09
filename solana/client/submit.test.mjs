import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';

test('uploader rejects a missing expected guest before reading a wallet or contacting RPC',()=>{
    const result=spawnSync(process.execPath,['client/submit.mjs','--rpc','http://127.0.0.1:1',
        '--program','11111111111111111111111111111111','--wallet','missing-wallet.json',
        '--proof','fixtures/blake3.json'],{cwd:new URL('../',import.meta.url),encoding:'utf8'});
    assert.notEqual(result.status,0);
    assert.match(result.stderr,/--guest/);
    assert.doesNotMatch(result.stderr,/ENOENT|fetch failed/);
});
