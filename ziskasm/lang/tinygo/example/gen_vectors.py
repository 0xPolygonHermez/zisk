"""Generates vectors.go: the inputs and expected outputs of the C example guests in
ziskasm/lang/c/example, plus expected values those guests do not carry (BN254 add/mul from
py_ecc, modexp from pow, BLAKE2f from hashlib.blake2b), and the vectors of the U256, zisklib
and hint checks, from independent Python models (EVM semantics, py_ecc, field arithmetic).
Needs py_ecc.

    python3 gen_vectors.py ../../c/example > vectors.go && gofmt -w vectors.go
"""
import re, sys, hashlib
ex = sys.argv[1]
def arrays(fname):
    src = open(f"{ex}/{fname}").read()
    out = {}
    for m in re.finditer(r'(?:static\s+)?(?:const\s+)?(\w+)\s+(\w+)\s*(?:\[\d*\])?\s*=\s*\{\{?([^;]*?)\}?\};', src):
        typ, name, body = m.groups()
        vals = [int(v, 0) for v in re.findall(r'0x[0-9a-fA-F]+|\d+', body)]
        out[name] = bytes(vals)
    return out
k1 = arrays("ef_secp256k1_guest.c"); rc = arrays("zkvm_ecrecover_guest.c"); r1 = arrays("zkvm_secp256r1_guest.c")
bn = arrays("bn254_guest.c"); me = arrays("modexp_guest.c"); bl = arrays("bls_guest.c"); mk = arrays("bls_map_kzg_guest.c")
b2 = arrays("zkvm_blake2f_guest.c")

from py_ecc import bn128
P = bn128.field_modulus
def g1(b): 
    x, y = int.from_bytes(b[:32],'big'), int.from_bytes(b[32:],'big')
    return (bn128.FQ(x), bn128.FQ(y))
def enc(p): return int(p[0]).to_bytes(32,'big') + int(p[1]).to_bytes(32,'big')
bn_add = enc(bn128.add(g1(bn['add_p1']), g1(bn['add_p2'])))
bn_mul = enc(bn128.multiply(g1(bn['mul_p']), int.from_bytes(bn['mul_k'],'big')))
def mx(b,e,m): 
    mod = int.from_bytes(m,'big'); return (pow(int.from_bytes(b,'big'), int.from_bytes(e,'big'), mod) if mod else 0).to_bytes(len(m),'big')
h = b2['g_h']; m = b2['g_m'].ljust(128, b'\0'); t = b2['g_t'].ljust(16, b'\0')
assert len(h)==64
blake = hashlib.blake2b(b'abc').digest()

def go(name, b, typ=None):
    lines = []
    for i in range(0, len(b), 16):
        lines.append("\t" + ", ".join(f"0x{x:02x}" for x in b[i:i+16]) + ",")
    t = typ or f"[{len(b)}]byte"
    return f"var {name} = {t}{{\n" + "\n".join(lines) + "\n}\n"
V = []
add = lambda n, b, t=None: V.append(go(n, b, t))
add("k1Msg", k1['MSG']); add("k1Sig", k1['SIG']); add("k1Pub", k1['PUB'])
add("recMsg", rc['MSG']); add("recSig", rc['SIG']); add("recPub", rc['EXP'])
add("r1Msg", r1['MSG']); add("r1Sig", r1['SIG']); add("r1Pub", r1['PUB'])
add("bnAddP1", bn['add_p1']); add("bnAddP2", bn['add_p2']); add("bnAddSum", bn_add)
add("bnMulP", bn['mul_p']); add("bnMulK", bn['mul_k']); add("bnMulProduct", bn_mul)
add("bnPairsValid", bn['pairA']); add("bnPairsInvalid", bn['pairR'])
for v in ("v2","v3","v4"):
    add(f"mod{v.upper()}Base", me[f'{v}_b'], "[]byte"); add(f"mod{v.upper()}Exp", me[f'{v}_e'], "[]byte")
    add(f"mod{v.upper()}Mod", me[f'{v}_m'], "[]byte"); add(f"mod{v.upper()}Want", mx(me[f'{v}_b'], me[f'{v}_e'], me[f'{v}_m']), "[]byte")
add("blakeH", h); add("blakeM", m); add("blakeT", t); add("blakeWant", blake)
for n in ("a1","b1","er1","a2","b2","er2","msm1","erm1","msm2","erm2","pairA","pairR"):
    add("bls" + n[0].upper() + n[1:], bl[n])
for n in ("u1","eg1","u2","eg2","kc","kz","ky","kyb","kp"):
    add("map" + n[0].upper() + n[1:] if n[0] in "ue" else "kzg" + n[1:].upper(), mk[n])

# ---- zkvm_u256.h / zkvm_u256_le.h: EVM semantics ----------------------------------------
M = 1 << 256
NEG1, MINNEG = M - 1, 1 << 255
def signed(x): return x - M if x >> 255 else x
def unsigned(x): return x % M
def sdiv(a, b):
    if b == 0: return 0
    q = abs(signed(a)) // abs(signed(b))
    return unsigned(-q if (signed(a) < 0) != (signed(b) < 0) else q)
def smod(a, b):
    if b == 0: return 0
    r = abs(signed(a)) % abs(signed(b))
    return unsigned(-r if signed(a) < 0 else r)
def sar(s, v): return unsigned((-1 if signed(v) < 0 else 0) if s >= 256 else signed(v) >> s)
def signextend(b, x):
    if b >= 31: return x
    bit = 8 * b + 7
    return unsigned(x | ~((1 << (bit + 1)) - 1)) if (x >> bit) & 1 else x & ((1 << (bit + 1)) - 1)
OPS = {
    "div": lambda a, b: [a // b if b else 0], "mod": lambda a, b: [a % b if b else 0],
    "divmod": lambda a, b: [a // b if b else 0, a % b if b else 0],
    "sdiv": lambda a, b: [sdiv(a, b)], "smod": lambda a, b: [smod(a, b)],
    "sdivmod": lambda a, b: [sdiv(a, b), smod(a, b)],
    "add": lambda a, b: [(a + b) % M], "sub": lambda a, b: [(a - b) % M],
    "mul": lambda a, b: [(a * b) % M],
    "addmod": lambda a, b, n: [(a + b) % n if n else 0],
    "mulmod": lambda a, b, n: [(a * b) % n if n else 0],
    "exp": lambda a, b: [pow(a, b, M)],
    "lt": lambda a, b: [int(a < b)], "gt": lambda a, b: [int(a > b)],
    "slt": lambda a, b: [int(signed(a) < signed(b))], "sgt": lambda a, b: [int(signed(a) > signed(b))],
    "eq": lambda a, b: [int(a == b)], "iszero": lambda a: [int(a == 0)],
    "and": lambda a, b: [a & b], "or": lambda a, b: [a | b], "xor": lambda a, b: [a ^ b],
    "not": lambda a: [NEG1 ^ a],
    "byte": lambda i, a: [(a >> (8 * (31 - i))) & 0xFF if i < 32 else 0],
    "shl": lambda s, v: [(v << s) % M if s < 256 else 0],
    "shr": lambda s, v: [v >> s if s < 256 else 0],
    "sar": lambda s, v: [sar(s, v)], "signextend": lambda b, x: [signextend(b, x)],
}
A = (1 << 255) + 0x0123456789abcdef_fedcba9876543210_0f1e2d3c4b5a6978  # negative as signed
B, C = 7, (1 << 128) + 3
CASES = [
    ("div", A, B), ("div", A, 0), ("mod", A, B), ("mod", A, 0), ("divmod", A, C), ("divmod", A, 0),
    ("sdiv", A, B), ("sdiv", MINNEG, NEG1), ("sdiv", A, 0), ("smod", A, B), ("smod", B, M - 2),
    ("sdivmod", A, B), ("add", NEG1, 2), ("add", A, C), ("sub", 2, NEG1), ("sub", B, A),
    ("mul", A, C), ("mul", NEG1, NEG1), ("addmod", NEG1, NEG1, C), ("addmod", A, B, 0),
    ("mulmod", NEG1, NEG1, C), ("mulmod", A, B, 0), ("exp", 3, 200), ("exp", 2, 256), ("exp", A, C),
    ("lt", A, B), ("lt", B, A), ("gt", A, B), ("slt", A, B), ("slt", B, A), ("sgt", A, B),
    ("eq", A, A), ("eq", A, B), ("iszero", 0), ("iszero", A), ("and", A, C), ("or", A, C),
    ("xor", A, C), ("not", A), ("byte", 0, A), ("byte", 31, A), ("byte", 32, A),
    ("shl", 4, A), ("shl", 256, A), ("shr", 4, A), ("shr", 255, A), ("sar", 4, A), ("sar", 256, A),
    ("signextend", 0, 0x80), ("signextend", 0, 0x7f), ("signextend", 1, 0x8000), ("signextend", 31, A),
]
assert {c[0] for c in CASES} == set(OPS)
hx = lambda v: '"%064x"' % v
rows = []
for op, *args in CASES:
    want = OPS[op](*args)
    rows.append('\t{"%s", []string{%s}, []string{%s}},' % (op, ", ".join(map(hx, args)), ", ".join(map(hx, want))))
V.append("// u256Cases holds every U256 operation with big-endian hex operands and results.\n"
         "var u256Cases = []u256Case{\n" + "\n".join(rows) + "\n}\n")

# ---- little-endian u64 limbs ----------------------------------------------------------
def limbs(v, n): return [(v >> (64 * i)) & (2**64 - 1) for i in range(n)]
def lit(ws, typ=None):
    return (typ or f"[{len(ws)}]uint64") + "{" + ", ".join(f"0x{w:x}" for w in ws) + "}"
def lv(name, ws, typ=None): V.append(f"var {name} = {lit(ws, typ)}\n")
be = lambda b: int.from_bytes(b, "big")
def be_limbs(b, size, n): return sum((limbs(be(b[i:i + size]), n) for i in range(0, len(b), size)), [])

# ---- zkvm_zisklib.h ---------------------------------------------------------------------
zA = 0xfedcba9876543210_0123456789abcdef_1111111111111111_2222222222222223
zB = 0x0000000000000000_00000000deadbeef_0000000000000001_00000000cafef00d
zM = (1 << 255) - 19
lv("zA", limbs(zA, 4)); lv("zB", limbs(zB, 4)); lv("zM", limbs(zM, 4)); lv("zAEven", limbs(zA - 1, 4))
lv("zInv256", limbs(pow(zA, -1, M), 4))
lv("zAddWrap", limbs((NEG1 + 2) % M, 4)); lv("zSubWrap", limbs((1 - 2) % M, 4))
lv("zMulOver", limbs(((1 << 200) * (1 << 100)) % M, 4)); lv("z2p200", limbs(1 << 200, 4)); lv("z2p100", limbs(1 << 100, 4))
lv("zQ", limbs(zA // zB, 4)); lv("zR", limbs(zA % zB, 4))
lv("zReduce", limbs(zA % zM, 4)); lv("zAddMod", limbs((zA + zB) % zM, 4)); lv("zMulMod", limbs(zA * zB % zM, 4))
lv("zInvMod", limbs(pow(zA, -1, zM), 4)); lv("zPowMod", limbs(pow(zA, zB, zM), 4))
lv("zPow3x100", limbs(3**100, 4)); lv("zPow3x200", limbs(3**200 % M, 4))
pk_l = lambda b: be_limbs(b, 32, 4)
lv("zK1Pub", pk_l(k1['PUB'])); lv("zK1Z", limbs(be(k1['MSG']), 4))
lv("zK1R", limbs(be(k1['SIG'][:32]), 4)); lv("zK1S", limbs(be(k1['SIG'][32:]), 4))
lv("zRecZ", limbs(be(rc['MSG']), 4)); lv("zRecR", limbs(be(rc['SIG'][:32]), 4))
lv("zRecS", limbs(be(rc['SIG'][32:]), 4)); lv("zRecPub", pk_l(rc['EXP']))
lv("zR1Pub", pk_l(r1['PUB'])); lv("zR1Z", limbs(be(r1['MSG']), 4))
lv("zR1R", limbs(be(r1['SIG'][:32]), 4)); lv("zR1S", limbs(be(r1['SIG'][32:]), 4))

# BIP-340 Schnorr, signed here with a fixed key and zero aux randomness.
from py_ecc.secp256k1 import secp256k1 as k256
NK = k256.N
def tagged(tag, msg):
    t = hashlib.sha256(tag.encode()).digest()
    return hashlib.sha256(t + t + msg).digest()
d0 = 0x1b2c3d4e5f60718293a4b5c6d7e8f90112233445566778899aabbccddeeff00
Pk = k256.multiply(k256.G, d0); d = d0 if Pk[1] % 2 == 0 else NK - d0
smsg = b"zisk schnorr"
t = (d ^ be(tagged("BIP0340/aux", bytes(32)))).to_bytes(32, "big")
px = Pk[0].to_bytes(32, "big")
k0 = be(tagged("BIP0340/nonce", t + px + smsg)) % NK
Rk = k256.multiply(k256.G, k0); kk = k0 if Rk[1] % 2 == 0 else NK - k0
e = be(tagged("BIP0340/challenge", Rk[0].to_bytes(32, "big") + px + smsg)) % NK
ss = (kk + e * d) % NK
lhs = k256.multiply(k256.G, ss); rhs = k256.add(Rk if Rk[1] % 2 == 0 else (Rk[0], k256.P - Rk[1]),
                                            k256.multiply((Pk[0], Pk[1] if Pk[1] % 2 == 0 else k256.P - Pk[1]), e))
assert lhs == rhs, "BIP-340 self-check"
lv("zSchnorrPk", limbs(Pk[0], 4)); lv("zSchnorrR", limbs(Rk[0], 4)); lv("zSchnorrS", limbs(ss, 4))
V.append(f'var zSchnorrMsg = []byte("{smsg.decode()}")\n')

def bn_pairs(b):
    g1, g2 = [], []
    for i in range(0, len(b), 192):
        p = b[i:i + 192]
        g1.append(be_limbs(p[:64], 32, 4))
        xi, xr, yi, yr = (p[64 + 32 * j:96 + 32 * j] for j in range(4))
        g2.append(be_limbs(xr + xi + yr + yi, 32, 4))
    return g1, g2
def bls_pairs(b):
    g1, g2 = [], []
    for i in range(0, len(b), 288):
        p = b[i:i + 288]
        g1.append(be_limbs(p[:96], 48, 6)); g2.append(be_limbs(p[96:], 48, 6))
    return g1, g2
def pairs_v(name, ps, n):
    V.append(f"var {name} = [][{n}]uint64{{\n" + "".join(f"\t{lit(p, '')},\n" for p in ps) + "}\n")
for name, (g1, g2) in [("zBnValid", bn_pairs(bn['pairA'])), ("zBnInvalid", bn_pairs(bn['pairR']))]:
    pairs_v(name + "G1", g1, 8); pairs_v(name + "G2", g2, 16)
for name, (g1, g2) in [("zBlsValid", bls_pairs(bl['pairA'])), ("zBlsInvalid", bls_pairs(bl['pairR']))]:
    pairs_v(name + "G1", g1, 12); pairs_v(name + "G2", g2, 24)
lv("zMapU1", be_limbs(mk['u1'], 48, 6)); lv("zMapG1", be_limbs(mk['eg1'], 48, 6))
lv("zMapU2", be_limbs(mk['u2'], 48, 6)); lv("zMapG2", be_limbs(mk['eg2'], 48, 6))

import py_ecc.optimized_bls12_381 as ob
from py_ecc.bls.hash_to_curve import hash_to_G2
from py_ecc.bls import G2Basic
hmsg, hdst = b"abc", b"QUUX-V01-CS02-with-BLS12381G2_XMD:SHA-256_SSWU_RO_"
hx_, hy_ = ob.normalize(hash_to_G2(hmsg, hdst, hashlib.sha256))
lv("zHashG2", sum((limbs(int(c), 6) for c in (*hx_.coeffs, *hy_.coeffs)), []))
V.append(f'var zHashMsg, zHashDst = []byte("{hmsg.decode()}"), []byte("{hdst.decode()}")\n')
bls_sk, bls_msg = 0x2a5f, b"zisk bls"
add("zBlsPk", G2Basic.SkToPk(bls_sk)); add("zBlsSig", G2Basic.Sign(bls_sk, bls_msg))
assert G2Basic.Verify(G2Basic.SkToPk(bls_sk), bls_msg, G2Basic.Sign(bls_sk, bls_msg))
V.append(f'var zBlsMsg = []byte("{bls_msg.decode()}")\n')
mb, me_, mm = (1 << 300) + 12345, 65537, (1 << 319) + (1 << 200) + 1
lv("zModexpBase", limbs(mb, 5), "[]uint64"); lv("zModexpExp", [me_], "[]uint64")
lv("zModexpMod", limbs(mm, 5), "[]uint64"); lv("zModexpWant", limbs(pow(mb, me_, mm), 8), "[]uint64")

# ---- zkvm_fcall.h hints -----------------------------------------------------------------
PK1 = 2**256 - 2**32 - 977
NK1 = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
NR1 = 0xFFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551
PBN = bn128.field_modulus
PBL = 0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab
PED = 2**255 - 19
hX = 0x0badc0ffee0ddf00d_123456789abcdef_0fedcba987654321_1122334455667788 % PK1
lv("hX", limbs(hX, 4))
lv("hK1FpInv", limbs(pow(hX, -1, PK1), 4)); lv("hK1FnInv", limbs(pow(hX, -1, NK1), 4))
lv("hR1FnInv", limbs(pow(hX, -1, NR1), 4)); lv("hBnFpInv", limbs(pow(hX, -1, PBN), 4))
s0 = 0x3141592653589793238462643383279502884197169399375105820974944592 % PK1
odd_root = s0 if s0 % 2 else PK1 - s0
lv("hK1Square", limbs(s0 * s0 % PK1, 4)); lv("hK1OddRoot", limbs(odd_root, 4))
lv("hK1NonResidueRoot", limbs(pow(9, (PK1 + 1) // 4, PK1), 4))  # the root of 3 * NQR (= 3)
# GLV: the decomposition of ziskos's glv.rs (Babai rounding on its short basis).
A1, MB1 = 0x3086D221A7D46BCDE86C90E49284EB15, 0xE4437ED6010E88286F547FA90ABFE4C3
A2, B2 = 0x114CA50F7A8E2F3F657C1108D9D44CFD8, 0x3086D221A7D46BCDE86C90E49284EB15
LAMBDA = 0x5363AD4CC05C30E0A5261C028812645A122E22EA20816678DF02967C1B23BD72
rnd = lambda n, d: (2 * n + d) // (2 * d)
gk = hX % NK1
c1, c2 = rnd(B2 * gk, NK1), rnd(MB1 * gk, NK1)
k1v, k2v = gk - c1 * A1 - c2 * A2, c1 * MB1 - c2 * B2
assert (k1v + k2v * LAMBDA) % NK1 == gk and abs(k1v) < 2**128 and abs(k2v) < 2**128
lv("hGlvK", limbs(gk, 4)); lv("hGlvK1", limbs(abs(k1v), 4)); lv("hGlvK2", limbs(abs(k2v), 4))
V.append(f"var hGlvNeg1, hGlvNeg2 = {str(k1v < 0).lower()}, {str(k2v < 0).lower()}\n")

def f2mul(a, b, p): return ((a[0] * b[0] - a[1] * b[1]) % p, (a[0] * b[1] + a[1] * b[0]) % p)
def f2sub(a, b, p): return ((a[0] - b[0]) % p, (a[1] - b[1]) % p)
def f2inv(a, p):
    n = pow(a[0] * a[0] + a[1] * a[1], -1, p)
    return (a[0] * n % p, -a[1] * n % p)
def f2l(a, n): return limbs(a[0], n) + limbs(a[1], n)
def lines(name, p, n):
    x1, y1 = (0x1234 + 5 * n, 0x5678), (0x9abc, 0xdef0 + n)
    x2, y2 = (0x1111, 0x2222), (0x3333, 0x4444)
    lam = f2mul(f2sub(y2, y1, p), f2inv(f2sub(x2, x1, p), p), p)
    mu = f2sub(y1, f2mul(lam, x1, p), p)
    lt = f2mul(f2mul((3, 0), f2mul(x1, x1, p), p), f2inv(f2mul((2, 0), y1, p), p), p)
    mt = f2sub(y1, f2mul(lt, x1, p), p)
    for suffix, v in [("X1", x1), ("Y1", y1), ("X2", x2), ("Y2", y2), ("AddLambda", lam),
                      ("AddMu", mu), ("DblLambda", lt), ("DblMu", mt)]:
        lv(name + suffix, f2l(v, n))
hF2 = (hX % PBN, 0x777)
lv("hBnFp2", f2l(hF2, 4)); lv("hBnFp2Inv", f2l(f2inv(hF2, PBN), 4)); lines("hBn", PBN, 4)
hXL = (hX << 100) % PBL
lv("hBlsX", limbs(hXL, 6)); lv("hBlsFpInv", limbs(pow(hXL, -1, PBL), 6))
sq = s0 * s0 % PBL
lv("hBlsSquare", limbs(sq, 6)); lv("hBlsRoot", limbs(pow(sq, (PBL + 1) // 4, PBL), 6))
hF2L = (hXL, 0x999)
lv("hBlsFp2", f2l(hF2L, 6)); lv("hBlsFp2Inv", f2l(f2inv(hF2L, PBL), 6)); lines("hBls", PBL, 6)
r2 = (s0 % PBL, 0x1234567)
lv("hBlsFp2Square", f2l(f2mul(r2, r2, PBL), 6))
lv("hBlsFp2Root", f2l(r2, 6)); lv("hBlsFp2NegRoot", f2l(((-r2[0]) % PBL, (-r2[1]) % PBL), 6))
bd = (1 << 64) + 11
lv("hBinDecompX", limbs(bd, 2), "[]uint64")
V.append("var hBinDecompBits = []byte{" + ", ".join(bin(bd)[2:]) + "}\n")
lv("hMsbA", limbs(1 << 130, 4)); lv("hMsbB", limbs(1 << 70, 4))
lv("hMsb384X", limbs(1 << 300, 6)); lv("hMsb384Y", limbs(5, 6))
lv("hDivQ", limbs(zA // zB, 4)); lv("hDivR", limbs(zA % zB, 4))
lv("hInv", limbs(pow(zA, -1, M), 4)); lv("hInvMod", limbs(pow(zA, -1, zM), 4))
lv("hSix", limbs(6, 4)); lv("hNine", limbs(9, 4))
ba, bb = (1 << 300) + 987654321, (1 << 100) + 17
def padded(v):
    w = [] if v == 0 else limbs(v, (v.bit_length() + 63) // 64)
    return w + [0] * (max(4, -(-len(w) // 4) * 4) - len(w))
lv("hBigA", limbs(ba, 5), "[]uint64"); lv("hBigB", limbs(bb, 2), "[]uint64")
lv("hBigQ", padded(ba // bb), "[]uint64"); lv("hBigR", padded(ba % bb), "[]uint64")
def ed_sqrt_even(w):
    c = pow(w, (PED + 3) // 8, PED)
    if c * c % PED != w:
        c = c * pow(2, (PED - 1) // 4, PED) % PED
    assert c * c % PED == w
    return PED - c if c & 1 else c
lv("hEdNine", limbs(9, 4)); lv("hEdOne", limbs(1, 4)); lv("hEdTwo", limbs(2, 4))
lv("hEdRoot9", limbs(ed_sqrt_even(9), 4)); lv("hEdRoot4", limbs(ed_sqrt_even(4), 4))

print("// Code generated by gen_vectors.py from the vectors of ziskasm/lang/c/example and the\n"
      "// models in gen_vectors.py. DO NOT EDIT.\n\npackage main\n")
print("\n".join(V))
