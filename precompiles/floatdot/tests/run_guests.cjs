// Build the legacy FP32 and mixed FloatDot guests (precompile and software variants),
// then run the ignored emulator guest and replay tests against them.
// Usage: node precompiles/floatdot/tests/run_guests.cjs
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '../../..');
const OUT = path.join(ROOT, 'target/fp32/artifacts');
const TARGET = 'riscv64ima-zisk-zkvm-elf';
fs.mkdirSync(OUT, { recursive: true });

function run(args, env = process.env) {
    const result = spawnSync('cargo', args, { cwd: ROOT, env, stdio: 'inherit' });
    if (result.error) throw result.error;
    if (result.status !== 0) {
        console.error(`cargo ${args[0]} failed (${result.signal ?? `exit ${result.status}`})`);
        process.exit(result.status || 1);
    }
}

const env = {
    ...process.env,
    RUSTFLAGS: '-C target-feature=+unaligned-scalar-mem,+zba --cfg zisk_guest -C link-arg=-T'
        + path.join(ROOT, 'ziskbuild/zisk_linker_script.ld'),
};
for (const [crate, prefix] of [['fp32-dot-guest', ''], ['floatdot-guest', 'floatdot-']]) {
    for (const [variant, features] of [['precompile', []], ['software', ['--features', 'software']]]) {
        run(['+zisk', 'build', '--locked', '--release', '--manifest-path', 'examples/Cargo.toml',
            '--target', TARGET, '-p', crate, '--jobs', '2', ...features], env);
        fs.copyFileSync(path.join(ROOT, 'examples/target', TARGET, 'release', crate),
            path.join(OUT, `${prefix}${variant}.elf`));
    }
}
run(['test', '--locked', '-p', 'ziskemu', '--test', 'floatdot_guest', '--test', 'floatdot_replay',
    '--jobs', '2', '--', '--ignored', '--nocapture', '--test-threads=1']);
