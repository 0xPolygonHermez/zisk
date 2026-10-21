#!/usr/bin/env python3
"""Write sail-riscv's configuration for the RISC-V that ZisK runs.

Starts from the default configuration of the pinned sail-riscv (what
`sail_riscv_sim --print-default-config` prints), and keeps only what ZisK's
transpiler supports in its default build (transpilers/riscv): RV64I, M, A,
Zba, Zbb, Zbs and Zbkb, Zicsr and Zifencei; machine mode only, and no
interrupts. F and D are out of scope; the compressed instructions, Zbc and
Zicond need transpiler features the default build leaves out (`compressed`
is off to match the EF RISC-V target standard, IALIGN = 32).

The memory map needs no change: the default's main memory,
0x8000_0000..0xFFFF_FFFF, holds ZisK's ROM (ROM_ADDR, 0x9000_0000) and RAM
(RAM_ADDR, 0xA000_0000), where the architectural tests live.

Usage: zisk_profile.py <sail_riscv_sim> <out.json>   (`make riscv-sim`)
"""

import json
import re
import subprocess
import sys

# The extensions ZisK runs. B is Zba, Zbb and Zbs.
PROFILE = {'M', 'A', 'B', 'Zbkb', 'Zicsr', 'Zifencei'}


def main():
    sim, out = sys.argv[1], sys.argv[2]
    text = subprocess.run([sim, '--print-default-config'], check=True, capture_output=True, text=True).stdout
    config = json.loads(re.sub(r'(?m)^\s*//.*$', '', text))
    for name, ext in config['extensions'].items():
        if isinstance(ext, dict) and 'supported' in ext:
            ext['supported'] = name in PROFILE
    # V has a support level rather than a flag.
    config['extensions']['V']['support_level'] = 'Disabled'
    # With neither F, V nor S, mstatus.FS and mstatus.VS are read-only zero.
    config['base']['mstatus']['fs_legal_states'] = 'ExtContext_Off'
    config['base']['mstatus']['vs_legal_states'] = 'ExtContext_Off'
    # No interrupt sources: nothing in a ZisK program can take one.
    platform = config['platform']
    for kind in platform['interrupts']['machine'].values():
        kind['supported'] = False
    platform['clint']['supported'] = False
    platform['simple_interrupt_generator']['supported'] = False
    with open(out, 'w') as f:
        json.dump(config, f, indent=2)
        f.write('\n')
    print(f'wrote {out}: RV64 {" ".join(sorted(PROFILE))}, M-mode only')


if __name__ == '__main__':
    main()
