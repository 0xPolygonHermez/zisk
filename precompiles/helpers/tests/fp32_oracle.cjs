// Build the bundled, unmodified SoftFloat oracles locally, then run the FP32 tests.
// Usage: node precompiles/helpers/tests/fp32_oracle.cjs
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '../../..');
const SOFT = path.join(ROOT, 'lib-float/c/SoftFloat-3e');
const SOURCE = path.join(SOFT, 'source');
const OUT = path.join(ROOT, 'target/fp32');
fs.mkdirSync(OUT, { recursive: true });

function run(cmd, args, env = process.env) {
    const result = spawnSync(cmd, args, { cwd: ROOT, env, stdio: 'inherit' });
    if (result.error) throw result.error;
    if (result.status !== 0) {
        console.error(`${cmd} failed (${result.signal ?? `exit ${result.status}`})`);
        process.exit(result.status || 1);
    }
}

const sources = [
    'f32_add.c', 'f32_mul.c', 's_addMagsF32.c', 's_subMagsF32.c',
    's_roundPackToF32.c', 's_normRoundPackToF32.c', 's_normSubnormalF32Sig.c',
    's_shiftRightJam32.c', 's_shiftRightJam64.c', 's_shortShiftRightJam64.c', 's_countLeadingZeros32.c',
    's_countLeadingZeros8.c', 'softfloat_state.c',
    '8086-SSE/softfloat_raiseFlags.c', '8086-SSE/s_propagateNaNF32UI.c',
];

function build(adapter, name, extra) {
    const oracle = path.join(OUT, name);
    run(process.env.CC || 'cc', [
        '-O2', '-std=c11', '-DSOFTFLOAT_FAST_INT64',
        '-DINLINE_LEVEL=0', '-I' + path.join(SOFT, 'build/Linux-x86_64-GCC'),
        '-I' + path.join(SOURCE, '8086-SSE'), '-I' + path.join(SOURCE, 'include'),
        path.join(__dirname, adapter), ...[...sources, ...extra].map((s) => path.join(SOURCE, s)),
        '-o', oracle,
    ]);
    return oracle;
}

const fp16 = ['f16_to_f32.c', 's_normSubnormalF16Sig.c', 's_countLeadingZeros16.c',
    '8086-SSE/s_f16UIToCommonNaN.c', '8086-SSE/s_commonNaNToF32UI.c'];
const env = {
    ...process.env,
    FP32_SOFTFLOAT_ORACLE: build('fp32_oracle.c', 'softfloat-oracle', []),
    FP16_SOFTFLOAT_ORACLE: build('fp16_oracle.c', 'softfloat-fp16-oracle', fp16),
};
run('cargo', ['test', '--locked', '-p', 'zisk-precomp-helpers',
    '--test', 'fp32', '--test', 'floatdot_formats', '--jobs', '2', '--', '--include-ignored'], env);
