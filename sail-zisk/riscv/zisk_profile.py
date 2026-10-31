#!/usr/bin/env python3
"""Write sail-riscv's configuration for the RISC-V that ZisK runs.

Starts from the default configuration of the pinned sail-riscv — the
rv64d_v256_e64.json its CMake configure writes, which is what
`sail_riscv_sim --print-default-config` prints and what its Lean build
uses — and keeps only what ZisK's
transpiler supports in its default build (transpilers/riscv): RV64I, M, A,
Zba, Zbb, Zbs and Zbkb, Zicsr and Zifencei; machine mode only, and no
interrupts. F and D are out of scope; the compressed instructions, Zbc and
Zicond need transpiler features the default build leaves out (`compressed`
is off to match the EF RISC-V target standard, IALIGN = 32).

Memory is ZisK's map: the input (read-only), the ROM (read-only,
executable) and the RAM (read-write), and no PMP.

Usage: zisk_profile.py <default config.json> <out.json>
    (`make riscv-sim`, `make riscv-zisk-lean`)
"""

import json
import re
import sys

# The extensions ZisK runs. B is Zba, Zbb and Zbs.
PROFILE = {'M', 'A', 'B', 'Zbkb', 'Zicsr', 'Zifencei'}

# ZisK's architecture ID, ARCH_ID_ZISK in core/src/mem.rs.
ARCH_ID_ZISK = 0x0FFFEEEE


def main():
    base, out = sys.argv[1], sys.argv[2]
    config = json.loads(re.sub(r'(?m)^\s*//.*$', '', open(base).read()))
    for name, ext in config['extensions'].items():
        if isinstance(ext, dict) and 'supported' in ext:
            ext['supported'] = name in PROFILE
    # V has a support level rather than a flag.
    config['extensions']['V']['support_level'] = 'Disabled'
    # With neither F, V nor S, mstatus.FS and mstatus.VS are read-only zero.
    config['base']['mstatus']['fs_legal_states'] = 'ExtContext_Off'
    config['base']['mstatus']['vs_legal_states'] = 'ExtContext_Off'
    # marchid is ZisK's ID, as the BIOS sets its cell: the runtime reads it to
    # tell ZisK from QEMU (doc/riscv_contract.md, precondition C).
    config['platform']['archid'] = ARCH_ID_ZISK
    # No interrupt sources: nothing in a ZisK program can take one.
    platform = config['platform']
    for kind in platform['interrupts']['machine'].values():
        kind['supported'] = False
    platform['clint']['supported'] = False
    platform['simple_interrupt_generator']['supported'] = False
    # ZisK's memory map (contract § 2, pil/zisk.pil), and nothing else: an
    # access outside it, or against its permissions, is an access fault, a
    # trap, as it is a failure on ZisK. The attributes are the default's main
    # memory's but for the permissions; sail-riscv's main memory must be
    # writable, so the read-only regions are IO memory, which changes nothing
    # else of an access.
    memory = config['memory']
    main = next(r for r in memory['regions'] if r['attributes']['mem_type'] == 'MainMemory')

    def region(base, size, readable, writable, executable):
        r = json.loads(json.dumps(main))
        r['base']['value'], r['size']['value'] = hex(base), hex(size)
        r['attributes'].update(readable=readable, writable=writable, executable=executable,
                               mem_type='MainMemory' if writable else 'IOMemory')
        return r
    memory['regions'] = [
        region(0x1000_0000, 0x8000_0000, True, False, False),  # input, INPUT_ADDR
        region(0x9000_0000, 0x0800_0000, True, False, True),   # ROM, ROM_ADDR
        region(0xA000_0000, 0x2000_0000, True, True, False),   # RAM, RAM_ADDR
    ]
    # ZisK has no PMP.
    memory['pmp']['count'] = 0
    memory['pmp']['usable_count'] = 0
    with open(out, 'w') as f:
        json.dump(config, f, indent=2)
        f.write('\n')
    print(f'wrote {out}: RV64 {" ".join(sorted(PROFILE))}, M-mode only')


if __name__ == '__main__':
    main()
