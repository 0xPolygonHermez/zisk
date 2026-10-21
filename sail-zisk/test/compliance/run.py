#!/usr/bin/env python3
"""Difftest the model on the RISC-V architectural tests, transpiled to ZisK.

Compiles every test of the chosen suites of riscv-arch-test
(riscv-test-suite/rv64i_m/<suite>/src/*.S) for ZisK's memory map, with
test/compliance/env, then runs each through `make difftest ELF=...`: ziskemu
and the model's C emulator run the transpiled program, and every step must
agree. Prints a line per test and a summary, and writes it to
build/compliance/results.txt. Exit status 1 if any test fails.

Usage: run.py [--arch-test DIR] [--gcc GCC] [-j N] [SUITE ...]   (`make compliance`)

SUITE is a directory of rv64i_m: I, M, A, C and B by default.
"""

import argparse
import concurrent.futures
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..'))
OUT = os.path.join(ROOT, 'build', 'compliance')

# The -march each suite is compiled with: the extension, and Zicsr, which
# the tests' framework uses.
MARCH = {
    'I': 'rv64i_zicsr',
    'M': 'rv64im_zicsr',
    'A': 'rv64ia_zicsr',
    'C': 'rv64imc_zicsr_zba_zbb_zcb',
    'Zicond': 'rv64i_zicsr_zicond',
    'Zifencei': 'rv64i_zicsr_zifencei',
    'B': 'rv64i_zicsr_zba_zbb_zbc_zbs',
}


# Tests of instructions ZisK does not run, and why: skipped, not failed.
SKIP = {
    'B-clmul-01': "the transpiler's default build has no Zbc (its zbc feature)",
    'B-clmulh-01': "the transpiler's default build has no Zbc (its zbc feature)",
    'B-clmulr-01': "the transpiler's default build has no Zbc (its zbc feature)",
    'Zicond-czero.eqz-01': "the transpiler's default build has no Zicond (its zicond_native feature)",
    'Zicond-czero.nez-01': "the transpiler's default build has no Zicond (its zicond_native feature)",
    'Zifencei-Fencei': 'it rewrites its own code, and ZisK runs code from a read-only ROM',
}


def compile_test(gcc, arch_test, suite, src):
    name = os.path.basename(src)[:-len('.S')]
    elf = os.path.join(OUT, suite, name + '.elf')
    os.makedirs(os.path.dirname(elf), exist_ok=True)
    cmd = [gcc, f'-march={MARCH[suite]}', '-mabi=lp64', '-static', '-mcmodel=medany',
           '-fvisibility=hidden', '-nostdlib', '-nostartfiles', '-g', '-Wa,-mno-relax',
           '-Wl,--no-relax', '-T', os.path.join(HERE, 'env', 'link.ld'),
           '-I', os.path.join(HERE, 'env'), '-I', os.path.join(arch_test, 'riscv-test-suite', 'env'),
           src, '-o', elf, '-DTEST_CASE_1=True', '-DXLEN=64']
    r = subprocess.run(cmd, capture_output=True, text=True)
    return name, elf, r.returncode == 0, r.stderr


def difftest(suite, name, elf):
    tag = f'{suite}-{name}'
    r = subprocess.run(['make', '--no-print-directory', 'difftest', f'ELF={elf}', f'NAME={tag}', 'INPUT='],
                       cwd=ROOT, capture_output=True, text=True)
    out = r.stdout + r.stderr
    m = re.search(r'OK: (\d+) steps', out)
    if r.returncode == 0 and m:
        return tag, True, f'{m.group(1)} steps'
    # The first line that says why.
    for pat in (r'^FAIL.*$', r'panicked at.*$', r'^.*(?:Error|error)[^\n]*$', r'^@@.*$'):
        f = re.search(pat, out, re.M)
        if f:
            return tag, False, f.group(0).strip()[:160]
    return tag, False, f'exit {r.returncode}'


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--arch-test', default=os.environ.get('ARCH_TEST', ''),
                    help='a riscv-arch-test checkout (ARCH_TEST)')
    ap.add_argument('--gcc', default=os.environ.get('RISCV_GCC', 'riscv-none-elf-gcc'),
                    help='a RISC-V GCC (RISCV_GCC)')
    ap.add_argument('-j', type=int, default=os.cpu_count() or 4)
    ap.add_argument('suites', nargs='*', default=['I', 'M', 'A', 'C', 'B'])
    args = ap.parse_args()
    if not args.arch_test or not os.path.isdir(os.path.join(args.arch_test, 'riscv-test-suite')):
        sys.exit('ARCH_TEST must name a riscv-arch-test checkout')
    if not shutil.which(args.gcc):
        sys.exit(f'cannot run {args.gcc}: RISCV_GCC must name a RISC-V GCC')

    jobs = []
    for suite in args.suites:
        d = os.path.join(args.arch_test, 'riscv-test-suite', 'rv64i_m', suite, 'src')
        for src in sorted(os.listdir(d)):
            if src.endswith('.S'):
                jobs.append((suite, os.path.join(d, src)))

    # One emulator runs every test: build it before they start.
    if subprocess.run(['make', '--no-print-directory', 'emu'], cwd=ROOT).returncode != 0:
        sys.exit('make emu failed')

    results = []
    with concurrent.futures.ThreadPoolExecutor(args.j) as pool:
        compiled = list(pool.map(lambda j: (j[0],) + compile_test(args.gcc, args.arch_test, *j), jobs))
        runs = []
        for suite, name, elf, ok, err in compiled:
            if f'{suite}-{name}' in SKIP:
                results.append((f'{suite}-{name}', None, 'skipped: ' + SKIP[f'{suite}-{name}']))
            elif ok:
                runs.append(pool.submit(difftest, suite, name, elf))
            else:
                results.append((f'{suite}-{name}', False, 'does not compile: ' + err.strip().split('\n')[0][:120]))
        for f in concurrent.futures.as_completed(runs):
            results.append(f.result())

    results.sort(key=lambda r: r[0])
    label = {True: 'PASS', False: 'FAIL', None: 'SKIP'}
    lines = [f'{label[ok]}  {tag:<32} {why}' for tag, ok, why in results]
    run = [r for r in results if r[1] is not None]
    passed = sum(ok for _, ok, _ in run)
    skipped = len(results) - len(run)
    by_suite = {}
    for tag, ok, _ in run:
        s = tag.split('-')[0]
        by_suite.setdefault(s, [0, 0])
        by_suite[s][0] += ok
        by_suite[s][1] += 1
    summary = f'{passed} of {len(run)} tests agree with ziskemu: ' + ', '.join(
        f'{s} {p}/{n}' for s, (p, n) in by_suite.items()) + (f'; {skipped} skipped' if skipped else '')
    text = '\n'.join(lines + ['', summary]) + '\n'
    os.makedirs(OUT, exist_ok=True)
    open(os.path.join(OUT, 'results.txt'), 'w').write(text)
    print(text, end='')
    sys.exit(0 if passed == len(run) else 1)


if __name__ == '__main__':
    main()
