import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const [runner, elf, input, expected, key, snarkKey, hintsPath, directory] = process.argv.slice(2);
assert.equal(process.argv.length, 10,
  'usage: node check-hints.mjs GPU_RUNNER ELF INPUT EXPECTED KEY SNARK_KEY HINTS NEW_DIRECTORY');
fs.mkdirSync(directory);
const hash = data => createHash('sha256').update(data).digest('hex');
const hints = fs.readFileSync(hintsPath);
const inputs = [];
const koala = [];
for (let offset = 0; offset < hints.length;) {
  assert(offset + 8 <= hints.length, 'truncated hint header');
  const length = hints.readUInt32LE(offset);
  const code = hints.readUInt32LE(offset + 4);
  const start = offset + 8;
  offset = start + Math.ceil(length / 8) * 8;
  assert(offset <= hints.length, 'truncated hint payload');
  if (code === 0xf0000) inputs.push(hints.subarray(start, start + length));
  if (code === 0xa00) {
    assert.equal(length, 64, 'unexpected Koala hint length');
    koala.push(start);
  }
}
assert.deepEqual(Buffer.concat(inputs), fs.readFileSync(input), 'hints belong to another input');
assert(koala.length >= 2, 'need at least two Koala hints');
const first = koala[0];
const second = koala.slice(1).find(offset => !hints.subarray(first, first + 64).equals(hints.subarray(offset, offset + 64)));
assert.notEqual(second, undefined, 'need two different hint inputs');
const reordered = Buffer.from(hints);
hints.copy(reordered, first, second, second + 64);
hints.copy(reordered, second, first, first + 64);
const reorderedPath = path.join(directory, 'reordered-hints.bin');
fs.writeFileSync(reorderedPath, reordered);
const report = {
  driver_sha256: hash(fs.readFileSync(new URL(import.meta.url))),
  runner_sha256: hash(fs.readFileSync(runner)), elf_sha256: hash(fs.readFileSync(elf)),
  input_sha256: hash(fs.readFileSync(input)), hints_sha256: hash(hints),
  reordered_hints_sha256: hash(reordered), koala_calls: koala.length, runs: [],
};
for (const [name, stream] of [['valid', hintsPath], ['reordered', reorderedPath]]) {
  const deadline = Date.now() + 45000;
  for (;;) {
    const gpu = spawnSync('nvidia-smi', ['--query-gpu=memory.free', '--format=csv,noheader,nounits'], {encoding: 'utf8', timeout: 5000});
    assert.equal(gpu.status, 0, 'cannot check GPU availability');
    if (Number(gpu.stdout.trim().split('\n')[0]) >= 30000) break;
    assert(Date.now() < deadline, 'GPU memory was not released between tests');
    spawnSync('sleep', ['1']);
  }
  const output = path.join(directory, name);
  const result = spawnSync('timeout', ['--kill-after=5s', '600s', runner,
    elf, input, expected, key, snarkKey, output, 'plonk', '1', stream],
  {encoding: 'utf8', maxBuffer: 32 * 1024 * 1024});
  if (result.error) throw result.error;
  const log = result.stdout + result.stderr;
  fs.writeFileSync(path.join(directory, `${name}.log`), log);
  assert(!result.signal && ![124, 137].includes(result.status), `${name}: timeout/resource failure`);
  if (name === 'valid') {
    assert.equal(result.status, 0, `positive control failed: ${log.slice(-4000)}`);
    const proof = JSON.parse(fs.readFileSync(path.join(output, 'report.json')));
    assert.equal(proof.runs.length, 1);
    assert.equal(proof.runs[0].program_bound_verification, true);
    assert.equal(proof.runs[0].publics_match, true);
    assert.equal(proof.hints_sha256, report.hints_sha256);
  } else {
    assert.notEqual(result.status, 0, 'reordered hints were accepted');
    assert(!fs.existsSync(path.join(output, 'run-1.proof')), 'unexpected proof artifact');
    assert(/InvalidProof|Error generating witness|constraints.*fail/i.test(log),
      `missing proof-level rejection; inspect ${name}.log (an ASM failure alone is insufficient)`);
    assert(!/out of memory|CUDA error|memory allocation.*fail|WaitTimeout/i.test(log),
      'infrastructure failure is not evidence of proof rejection');
  }
  report.runs.push({name, exit_code: result.status, passed: true});
  fs.writeFileSync(path.join(directory, 'report.json'), JSON.stringify(report, null, 2));
  console.log(`hint regression: ${name} passed`);
}
