#!/usr/bin/env python3
"""prep.py <orig_dir> <work_dir>: source-level edits applied before zopt (batched Miller
loop, unrolled scalar multiplications, direct precompile operands)."""
import sys, os, subprocess
O, W = sys.argv[1], sys.argv[2]
G = os.path.dirname(os.path.abspath(__file__))
for d in ('bn254', 'bls12_381', 'zkvm'):
    os.makedirs(f'{W}/{d}', exist_ok=True)
    for f in os.listdir(f'{O}/{d}'):
        s = open(f'{O}/{d}/{f}').read()
        # the first-version generators now live in scripts/legacy; the symbol
        # redirects became zkvmcalls
        s = s.replace('scratchpad/gen_', 'scripts/legacy/gen_')
        s = s.replace('; redirect\n; targets of', '; zkvmcall\n; targets of')
        s = s.replace('redirect targets of zkvm_bn254_*', 'zkvmcall targets of zkvm_bn254_*')
        open(f'{W}/{d}/{f}', 'w').write(s)
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
# the curve_add precompiles take their two pointers directly (a = p1, updated in
# place; b = p2) instead of a 2-pointer header
for c, P in (('bn254', 'BN_'), ('bls12_381', 'BLS_')):
    p = f'{W}/{c}/curve.zisk'
    pre = f'{c}_curve_add'
    edit(p, f'''	copyb(0, {P}ADD_HDR) -> r5
	copyb(0, {P}ADD_RES) -> r6
	copyb(r6, 8[a + 0]) -> r6
	copyb(r5, r6) -> 8[a + 0]
	copyb(0, {P}ADD_P2) -> r7
	copyb(r7, 8[a + 0]) -> r7
	copyb(r5, r7) -> 8[a + 8]
	{pre}(0, r5) -> r14
''', f'''	copyb(0, {P}ADD_RES) -> r5
	copyb(r5, 8[a + 0]) -> r5
	copyb(0, {P}ADD_P2) -> r7
	copyb(r7, 8[a + 0]) -> r7
	{pre}(r5, r7) -> r14
''')
    edit(p, f'u64 {P}ADD_HDR[2]  = 0, 0\n', '')
    edit(p, f'u64 {P}SM_HDR[2]   = 0, 0\n', '')
edit(f'{W}/bn254/curve.zisk', '; (header [&p1,&p2], p1+=p2 in place)', '; (a=&p1, b=&p2, p1+=p2 in place)')
edit(f'{W}/bls12_381/curve.zisk', 'Uses bls12_381_curve_add (header [&p1,&p2] each', 'Uses bls12_381_curve_add (a=&p1, b=&p2, each')
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
