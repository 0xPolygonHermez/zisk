import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

test('release fixture binds the exact committed bytes and leaf root', () => {
  const f = JSON.parse(readFileSync(new URL('../fixtures/blake3.json', import.meta.url)));
  const bytes = name => Buffer.from(f[name].slice(2), 'hex');
  assert.equal(bytes('proofBytes').length, 6752);
  assert.equal(bytes('programVK').length, 32);
  assert.equal(bytes('publicValues').length, 512);
  const root = Buffer.alloc(32);
  [17146226293136250724n,16833344589786796689n,10321050178608571017n,2095991909911874349n]
    .forEach((x, i) => root.writeBigUInt64BE(x, i * 8));
  assert.deepEqual(bytes('rootCVadcopFinal'), root);
  const proof = readFileSync(new URL('../../zisk-contracts/test/blake3_pilfflonk.bin', import.meta.url));
  assert.equal(createHash('sha256').update(proof).digest('hex'),
    '002e1c53bd52a05f733835d10d12b7506e215e33e9be08e4d5c210245596ad8b');
});
