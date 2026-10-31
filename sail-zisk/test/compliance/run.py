#!/usr/bin/env python3
"""Difftest the model on the RISC-V architectural tests, transpiled to ZisK.

Compiles every test of the chosen suites of riscv-arch-test
(riscv-test-suite/rv64i_m/<suite>/src/*.S) for ZisK's memory map, with
test/compliance/env, then runs each through `make difftest ELF=...`: ziskemu
and the model's C emulator run the transpiled program, and every step must
agree, the test's signature included. With --riscv-sim, sail-riscv's C
emulator also runs the RISC-V program itself, in ZisK's profile
(build/riscv/zisk_rv64.json, from riscv/zisk_profile.py), up to the halt (exit_cleanup), and its signature
must be the model's. With --zisk-riscv, ZisK's RISC-V machine
(lean/ZiskRiscvRun, `make riscv-run`) runs it as well, and must end as the
model does: exit, output and signature. Prints a line per test and a summary, and writes it to
build/compliance/results.txt. Exit status 1 if any test fails.

Usage: run.py [--arch-test DIR] [--gcc GCC] [--riscv-sim SIM] [-j N] [SUITE ...]
    (`make compliance`)

SUITE is a directory of rv64i_m: I, M, A, B and K's Zbkb tests by default,
ZisK's profile. C, Zicond and Zifencei are known, and skipped for want of
the transpiler features they need.
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
RISCV_CONFIG = os.path.join(ROOT, 'build', 'riscv', 'zisk_rv64.json')

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
    'K': 'rv64i_zicsr_zbkb',
}

# The tests of a suite that are in ZisK's profile, when not all of them are:
# of the scalar crypto suite, Zbkb's (the rest of Zbkb is in B).
ONLY = {
    'K': {'brev8-01', 'pack-01', 'packh-01', 'packw-01'},
}

# Suites of instructions ZisK does not run, and why: skipped, not failed.
SKIP_SUITE = {
    'C': "the transpiler's default build has no compressed instructions (its compressed feature)",
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


def symbols(nm, elf):
    """The ELF's symbols, by name."""
    out = subprocess.run([nm, elf], check=True, capture_output=True, text=True).stdout
    return {f[2]: int(f[0], 16) for f in (line.split() for line in out.splitlines()) if len(f) == 3}


def riscv_signature(sim, elf, stop, tag):
    """The test's signature as sail-riscv computes it, a 32-bit word a line,
    run in ZisK's profile up to `stop`; or why there is none."""
    sig = os.path.join(OUT, 'riscv', tag + '.sig')
    os.makedirs(os.path.dirname(sig), exist_ok=True)
    r = subprocess.run([sim, '--config', RISCV_CONFIG, '--stop-at-pc', hex(stop), '--inst-limit', '10000000',
                        '--test-signature', sig, elf], capture_output=True, text=True)
    if r.returncode != 0 or not os.path.exists(sig):
        return None, f'sail-riscv fails: {(r.stderr or r.stdout).strip().splitlines()[-1:]}'
    return [int(w, 16) for w in open(sig).read().split()], None


def ending(tag, halt):
    """None if the model's run of a test reached the test's halt, `halt`
    (exit_cleanup); else where it stopped instead. The end of the run does
    not tell: the exit syscall ends a ZisK run through `halt`, as an error
    with the exit code, just as an instruction ZisK does not run does."""
    trace = open(os.path.join(ROOT, 'build', 'difftest', tag, 'sail.trace')).read()
    if re.search(r'^\d+ 0x%016X ' % halt, trace, re.M):
        return None
    steps = [l for l in trace.splitlines() if re.match(r'\d+ ', l)]
    risc = [l.split()[1] for l in steps if int(l.split()[1], 16) >= 0x90000000]
    at = risc[-1] if risc else '?'
    return f'stops before the test\'s halt, after {len(steps)} steps, last in the program at {at}'


def zisk_riscv(tag, elf, begin, end):
    """None if ZisK's RISC-V machine ends the test as the model does; else why not."""
    r = subprocess.run(['make', '--no-print-directory', 'zisk-riscv', f'ELF={elf}', f'NAME={tag}',
                        f'DUMP={begin:x},{end:x}'], cwd=ROOT, capture_output=True, text=True)
    if r.returncode == 0:
        return None
    out = r.stdout + r.stderr
    return next((l.strip() for l in out.splitlines() if l.startswith(('FAIL', 'zisk_riscv_run', 'failure'))),
                f'exit {r.returncode}')[:160]


def difftest(suite, name, elf, nm, sim, zrv):
    tag = f'{suite}-{name}'
    sym = symbols(nm, elf)
    begin, end = sym['begin_signature'], sym['end_signature']
    r = subprocess.run(['make', '--no-print-directory', 'difftest', f'ELF={elf}', f'NAME={tag}', 'INPUT=',
                        f'DUMP={begin:x},{end:x}'],
                       cwd=ROOT, capture_output=True, text=True)
    out = r.stdout + r.stderr
    m = re.search(r'OK: (\d+) steps', out)
    if r.returncode == 0 and m:
        # Agreeing is not enough: a test both emulators abandon at the same
        # instruction agrees too.
        why = ending(tag, sym['exit_cleanup'])
        if why:
            return tag, False, why
        if not sim:
            return tag, True, f'{m.group(1)} steps'
        trace = open(os.path.join(ROOT, 'build', 'difftest', tag, 'sail.trace')).read()
        zisk = [int(w, 16) for w in re.findall(r'^dump \S+ (0x[0-9A-F]+)$', trace, re.M)]
        riscv, why = riscv_signature(sim, elf, sym['exit_cleanup'], tag)
        if riscv is None:
            return tag, False, why
        if riscv != zisk:
            k = next((i for i, (x, y) in enumerate(zip(riscv, zisk)) if x != y), min(len(riscv), len(zisk)))
            return tag, False, (f'signature differs from sail-riscv\'s at word {k} of {len(zisk)}'
                                f' ({begin + 4 * k:#x})')
        if zrv:
            why = zisk_riscv(tag, elf, begin, end)
            if why:
                return tag, False, f'ZisK\'s RISC-V machine: {why}'
            return tag, True, f'{m.group(1)} steps; signature = sail-riscv\'s = ZisK\'s RISC-V machine\'s ({len(zisk)} words)'
        return tag, True, f'{m.group(1)} steps; signature = sail-riscv\'s ({len(zisk)} words)'
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
    ap.add_argument('--riscv-sim', default=os.environ.get('RISCV_SIM', ''),
                    help="sail-riscv's sail_riscv_sim, to compare signatures with (RISCV_SIM)")
    ap.add_argument('--zisk-riscv', action='store_true',
                    help="also run each test on ZisK's RISC-V machine (needs make riscv-run, and --riscv-sim)")
    ap.add_argument('-j', type=int, default=os.cpu_count() or 4)
    ap.add_argument('suites', nargs='*', default=['I', 'M', 'A', 'B', 'K'])
    args = ap.parse_args()
    if not args.arch_test or not os.path.isdir(os.path.join(args.arch_test, 'riscv-test-suite')):
        sys.exit('ARCH_TEST must name a riscv-arch-test checkout')
    if not shutil.which(args.gcc):
        sys.exit(f'cannot run {args.gcc}: RISCV_GCC must name a RISC-V GCC')
    # binutils' nm, beside the GCC
    nm = re.sub(r'gcc$', 'nm', args.gcc)
    if args.riscv_sim and not shutil.which(args.riscv_sim):
        sys.exit(f'cannot run {args.riscv_sim}: RISCV_SIM must name sail-riscv\'s sail_riscv_sim')

    jobs = []
    for suite in args.suites:
        d = os.path.join(args.arch_test, 'riscv-test-suite', 'rv64i_m', suite, 'src')
        for src in sorted(os.listdir(d)):
            if src.endswith('.S') and (suite not in ONLY or src[:-len('.S')] in ONLY[suite]):
                jobs.append((suite, os.path.join(d, src)))

    # One emulator runs every test: build it before they start.
    if subprocess.run(['make', '--no-print-directory', 'emu'], cwd=ROOT).returncode != 0:
        sys.exit('make emu failed')

    results = []
    with concurrent.futures.ThreadPoolExecutor(args.j) as pool:
        compiled = list(pool.map(lambda j: (j[0],) + compile_test(args.gcc, args.arch_test, *j), jobs))
        runs = []
        for suite, name, elf, ok, err in compiled:
            if suite in SKIP_SUITE:
                results.append((f'{suite}-{name}', None, 'skipped: ' + SKIP_SUITE[suite]))
            elif f'{suite}-{name}' in SKIP:
                results.append((f'{suite}-{name}', None, 'skipped: ' + SKIP[f'{suite}-{name}']))
            elif ok:
                runs.append(pool.submit(difftest, suite, name, elf, nm, args.riscv_sim, args.zisk_riscv))
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
    against = ('ziskemu, sail-riscv and ZisK\'s RISC-V machine' if args.riscv_sim and args.zisk_riscv
               else 'ziskemu and sail-riscv' if args.riscv_sim else 'ziskemu')
    summary = f'{passed} of {len(run)} tests agree with {against}: ' + ', '.join(
        f'{s} {p}/{n}' for s, (p, n) in by_suite.items()) + (f'; {skipped} skipped' if skipped else '')
    text = '\n'.join(lines + ['', summary]) + '\n'
    os.makedirs(OUT, exist_ok=True)
    open(os.path.join(OUT, 'results.txt'), 'w').write(text)
    print(text, end='')
    sys.exit(0 if passed == len(run) else 1)


if __name__ == '__main__':
    main()
