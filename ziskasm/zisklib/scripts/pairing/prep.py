#!/usr/bin/env python3
"""prep.py <orig_dir> <work_dir>: source-level edits applied before zopt (batched Miller loop)."""
import sys, os, subprocess
O, W = sys.argv[1], sys.argv[2]
G = os.path.dirname(os.path.abspath(__file__))
for d in ('bn254', 'bls12_381', 'zkvm'):
    os.makedirs(f'{W}/{d}', exist_ok=True)
    for f in os.listdir(f'{O}/{d}'):
        s = open(f'{O}/{d}/{f}').read()
        # the first-version generators now live in scripts/legacy
        open(f'{W}/{d}/{f}', 'w').write(s.replace('scratchpad/gen_', 'scripts/legacy/gen_'))
def edit(p, old, new, count=1):
    s = open(p).read()
    assert s.count(old) == count, (p, old[:60], s.count(old))
    open(p, 'w').write(s.replace(old, new))
for c, P in (('bn254', 'PC_'), ('bls12_381', 'LPC_')):
    p = f'{W}/{c}/pairing.zisk'
    lp = P.lower()[:-1]
    edit(p, f'{lp}_finit_done:\n', f'{lp}_finit_done:\n\tcall zisklib_ml_batch_reset_{c}\n')
    edit(p, f'''	copyb(0, {P}MI) -> r12
	call zisklib_miller_loop_{c}
	copyb(0, {P}F) -> r10
	copyb(0, {P}MI) -> r11
	copyb(0, {P}F) -> r12
	call zisklib_mul_fp12_{c}
''', f'''	call zisklib_ml_batch_push_{c}
''')
    edit(p, f'''	; res = final_exp(f)
	copyb(0, {P}F) -> r10
''', f'''	; f = product of the batched Miller loops ; res = final_exp(f)
	copyb(0, {P}F) -> r10
	call zisklib_ml_batch_finish_{c}
	copyb(0, {P}F) -> r10
''')
    ml = f'{W}/{c}/miller_loop.zisk'
    txt = subprocess.run([sys.executable, f'{G}/mlbatch.py', c], capture_output=True, text=True, check=True).stdout
    open(ml, 'a').write('\n' + txt)
sys.path.insert(0, G)
import smul
for c in ('bn254', 'bls12_381'):
    p = f'{W}/{c}/curve.zisk'
    t = smul.patch(open(p).read(), c)
    open(p, 'w').write(t)
import sgx
p = f'{W}/bls12_381/subgroup.zisk'
t = sgx.patch(open(p).read())
open(p, 'w').write(t)
p = f'{W}/bls12_381/kzg.zisk'
edit(p, '''	copyb(0, BLS_K_M1) -> r12
	call zisklib_miller_loop_bls12_381
	copyb(0, BLS_K_PROOF) -> r10
	copyb(0, BLS_K_TMZ) -> r11
	copyb(0, BLS_K_M2) -> r12
	call zisklib_miller_loop_bls12_381
	copyb(0, BLS_K_M1) -> r10
	copyb(0, BLS_K_M2) -> r11
	copyb(0, BLS_K_F) -> r12
	call zisklib_mul_fp12_bls12_381
''', '''	call zisklib_ml_batch_reset_bls12_381
	copyb(0, BLS_K_CMY) -> r10
	copyb(0, BLS_K_NEGG2) -> r11
	call zisklib_ml_batch_push_bls12_381
	copyb(0, BLS_K_PROOF) -> r10
	copyb(0, BLS_K_TMZ) -> r11
	call zisklib_ml_batch_push_bls12_381
	copyb(0, BLS_K_F) -> r10
	call zisklib_ml_batch_finish_bls12_381
''')
