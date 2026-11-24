// Constraint tests of the DotF8 air without buses: honest traces pass, forged ones fail. The
// evaluator checks polynomial constraints only, so the decode lookups and the range checks are
// checked here against an independent decoder.
const assert = require('node:assert/strict');
const path = require('node:path');
const {P, compile, checker} = require('./pil_test.cjs');

const N = 64;
const pil = compile(path.join(__dirname, 'dot_f8.pil'), 'dot_f8_test');
const air = name => pil.airGroups[0].airs.findIndex(a => a.name === name);
const sm = checker(pil, air('DotF8'));
const table = pil.airGroups[0].airs[air('DotF8DecodeTable')];

const mod = v => ((BigInt(v) % P) + P) % P;
function inv(v) {
    let [r, b, e] = [1n, mod(v), P - 2n];
    for (; e; e >>= 1n, b = b * b % P) if (e & 1n) r = r * b % P;
    return r;
}

// Independent decoder: the value of code c of format f, in units.
function decode(f, c) {
    if (f === 0) return c < 128 ? c : c - 256;
    if (f === 3) {
        const e = (c >> 1) & 3, m = c & 1, mag = e ? (2 + m) << (e - 1) : m;
        return c & 8 ? -mag : mag;
    }
    const e = (c >> 3) & 15, m = c & 7;
    let mag = e ? (8 + m) << (e - 1) : m;
    if ((f === 1 && e === 15 && m === 7) || (f === 2 && c === 0x80)) mag = 0;
    return c & 0x80 ? -mag : mag;
}
const scalar = b => b.length ? BigInt('0x' + Buffer.from(b).toString('hex')) : 0n;
const fixed = i => table.fixedCols[i].values.map(scalar);
const [F, C, V] = [0, 1, 2].map(fixed);
const entries = new Set(F.map((f, r) => `${f},${C[r]},${V[r]}`));
for (let f = 0; f < 4; f++)
    for (let c = 0; c < (f === 3 ? 16 : 256); c++) assert.ok(entries.has(`${f},${c},${mod(decode(f, c))}`));
assert.equal(entries.size, 3 * 256 + 16);
assert.equal(decode(1, 0x38), 512); // 1.0 in 2^-9 units
assert.equal(decode(1, 0x7e), 448 << 9);
assert.equal(decode(2, 0x7f), 240 << 10);
assert.equal(decode(3, 7), 12); // 6.0 in 2^-1 units

// i64 limbs of v: [l0, l1], [h0, h1], sign.
function limbs(v) {
    const u = BigInt.asUintN(64, v), hi = u >> 32n, sg = hi >> 31n, h31 = hi & 0x7fffffffn;
    return [[u & 0xffffn, (u >> 16n) & 0xffffn], [h31 & 0xffffn, h31 >> 16n], sg];
}

function rnd(seed) {
    let s = seed;
    return () => (s = (s * 1103515245 + 12345) % 2 ** 31) & 255;
}

// Rows of one call: x and y as byte arrays, R rows, Bk rows per block (bz: block word 0). With
// rep (DOT_F8_REPEAT_Y), y is one block, read again for each block.
function call({fx, fy, R, Bk = R, bz = 0, ow = 0, rep = 0, seed = 1, x, y, step = 100, pw = 7, xw = 50, yw = 90}) {
    const r = rnd(seed), per = f => f === 3 ? 4 : 8; // bytes per row
    x = x ?? Array.from({length: per(fx) * R}, r);
    y = y ?? Array.from({length: per(fy) * (rep ? Math.min(Bk, R) : R)}, r);
    const codes = (f, b, i) => f === 3
        ? Array.from({length: 8}, (_, j) => (b[4 * i + (j >> 1)] >> (4 * (j & 1))) & 15)
        : b.slice(8 * i, 8 * i + 8);
    const word = (f, b, i) => {
        const at = f === 3 ? 8 * (i >> 1) : 8 * i, w = b.slice(at, at + 8);
        while (w.length < 8) w.push(0);
        return [0, 1].map(h => w.slice(4 * h, 4 * h + 4).reduce((a, v, k) => a + BigInt(v) * 256n ** BigInt(k), 0n));
    };
    const ex = fx === 3 ? 1 : 0, ey = fy === 3 ? 1 : 0, nz = R ? 1 : 0;
    let total = 0n;
    const blocks = [];
    const rows = [];
    let prev;
    for (let i = 0; i < Math.max(R, 1); i++) {
        const start = i === 0, par = i % 2;
        const bidx = start ? 0 : prev.be ? 0 : prev.bidx + 1;
        const yi = rep ? bidx : i, yr = rep && !start && bidx === 0 ? 1 : 0;
        const xc = nz ? codes(fx, x, i) : Array(8).fill(0), yc = nz ? codes(fy, y, yi) : Array(8).fill(0);
        const xv = xc.map(c => nz ? decode(fx, c) : 0), yv = yc.map(c => nz ? decode(fy, c) : 0);
        const prod = xv.reduce((a, v, j) => a + BigInt(v) * BigInt(yv[j]), 0n);
        const accb = start ? 0n : prev.be ? 0n : prev.s;
        const s = accb + prod, tot = start ? 0n : prev.t, t = tot + prod;
        const rem = BigInt(R - 1 - i), brem = BigInt(Bk - 1 - bidx);
        const ce = rem === 0n ? 1 : 0, be = nz && brem === 0n ? 1 : 0;
        const lx = 1 - ex * par, ly = 1 - ey * par;
        const oz = ow ? 0 : 1, w = be * (1 - oz);
        const ob = start ? ow : prev.ob + prev.be;
        const row = {
            in_use: 1, start: +start, end: nz ? ce : 1, active: nz, nz, ce: nz ? ce : 0, be, bz, oz, par, lx, ly, w,
            fx: [fx & 1, fx >> 1], fy: [fy & 1, fy >> 1], ex, ey, rep, yr, yb: yw,
            step, pw, R, r_inv: inv(R), idx: i, rem_inv: inv(rem), Bk, bidx, brem_inv: inv(brem),
            ow, ow_inv: inv(ow), ob,
            xw: xw + (ex ? i >> 1 : i), yw: yw + (ey ? yi >> 1 : yi),
            x_word: nz ? word(fx, x, i) : [0, 0], y_word: nz ? word(fy, y, yi) : [0, 0],
            xc, xv: xv.map(mod), yc, yv: yv.map(mod),
            accb: mod(accb), s: mod(s), tot: mod(tot), t: mod(t),
            bl: [0, 0], bh: [0, 0], bsg: 0,
        };
        if (w) [row.bl, row.bh, row.bsg] = limbs(s);
        if (be) blocks.push(s);
        Object.assign(row, {_s: s, _t: t, _bidx: bidx});
        prev = {be, bidx, s, t, ob};
        rows.push(row);
        total = t;
    }
    const [cl, ch, csg] = limbs(total);
    for (const row of rows) Object.assign(row, {cl: [...cl], ch: [...ch], csg});
    if (!R) {
        const row = rows[0];
        row.bz = Number(Bk === 0);
        row.r_inv = inv(Bk);
        [xw, yw, Bk, ow].forEach((words, j) => {
            const raw = 8n * BigInt(words);
            const units = (raw & 0xffffffffn) / (j === 2 && (ex || ey) ? 16n : 8n);
            row.xv[j] = raw >> 32n;
            (j < 2 ? row.cl : row.bl)[j % 2] = units & 0xffffn;
            (j < 2 ? row.ch : row.bh)[j % 2] = units >> 16n;
        });
    }
    return {rows, total, blocks, x, y};
}

function padding() {
    return {
        in_use: 0, start: 1, end: 1, active: 0, nz: 0, ce: 0, be: 0, bz: 0, oz: 1, par: 0, lx: 1, ly: 1, w: 0,
        fx: [0, 0], fy: [0, 0], ex: 0, ey: 0, rep: 0, yr: 0, yb: 0, step: 0, pw: 0, R: 0, r_inv: 0, idx: 0, rem_inv: P - 1n,
        Bk: 0, bidx: 0, brem_inv: 0, ow: 0, ow_inv: 0, ob: 0, xw: 0, yw: 0, x_word: [0, 0], y_word: [0, 0],
        xc: Array(8).fill(0), xv: Array(8).fill(0), yc: Array(8).fill(0), yv: Array(8).fill(0),
        accb: 0, s: 0, tot: 0, t: 0, cl: [0, 0], ch: [0, 0], csg: 0, bl: [0, 0], bh: [0, 0], bsg: 0,
    };
}

const strip = row => Object.fromEntries(Object.entries(row).filter(([k]) => !k.startsWith('_')));

// The failing constraint lines, the lookups not in the table, and the range checks that fail.
function check(rows) {
    rows = rows.concat(Array.from({length: N - rows.length}, padding));
    const w = rows.map(r => sm.witness(strip(r)));
    const failed = [];
    for (let i = 0; i < N; i++)
        failed.push(...sm.failures(w[i], {rowIndex: i, neighbors: {[-1]: w[(i + N - 1) % N]}}));
    let lookups = 0, ranges = 0;
    for (const r of rows) {
        if (r.active) for (let j = 0; j < 8; j++) {
            if (!entries.has(`${r.fx[0] + 2 * r.fx[1]},${r.xc[j]},${mod(r.xv[j])}`)) lookups++;
            if (!entries.has(`${r.fy[0] + 2 * r.fy[1]},${r.yc[j]},${mod(r.yv[j])}`)) lookups++;
        }
        const u16 = v => BigInt(v) >= 0n && BigInt(v) < 65536n;
        const u15 = v => BigInt(v) >= 0n && BigInt(v) < 32768n;
        const limbs_ok = (l, h) => u16(l[0]) && u16(l[1]) && u16(h[0]) && u15(h[1]);
        if (r.start && r.in_use && !limbs_ok(r.cl, r.ch)) ranges++;
        if ((r.w || (r.in_use && !r.active)) && !limbs_ok(r.bl, r.bh)) ranges++;
    }
    return {failed, lookups, ranges};
}
const ok = rows => {
    const c = check(rows);
    assert.deepEqual(c, {failed: [], lookups: 0, ranges: 0});
};
let rejected = 0;
const rejects = (rows, what) => {
    const c = check(rows);
    assert.ok(c.failed.length + c.lookups + c.ranges > 0, `accepted forgery: ${what}`);
    rejected++;
};

// Reference sums from the bytes, independent of the trace.
function reference(fx, x, fy, y, n, block, rep) {
    const val = (f, b, i) => decode(f, f === 3 ? (b[i >> 1] >> (4 * (i & 1))) & 15 : b[i]);
    const sums = [];
    for (let b = 0; b < n / block; b++) {
        let s = 0n;
        for (let i = b * block; i < (b + 1) * block; i++) s += BigInt(val(fx, x, i)) * BigInt(val(fy, y, rep ? i - b * block : i));
        sums.push(s);
    }
    return [sums.reduce((a, v) => a + v, 0n), sums];
}

const calls = [
    call({fx: 1, fy: 1, R: 4, Bk: 2, ow: 300, seed: 3}),
    call({fx: 3, fy: 1, R: 4, bz: 1, seed: 5}),
    call({fx: 0, fy: 0, R: 3, Bk: 1, ow: 400, seed: 7}),
    call({fx: 2, fy: 3, R: 2, bz: 1, ow: 500, seed: 11}),
    call({fx: 1, fy: 0, R: 0}),
    // Largest E4M3 magnitudes, -448 * 448 per product: a negative total, so the sign is set.
    call({fx: 1, fy: 1, R: 2, bz: 1, ow: 600, x: Array(16).fill(0xfe), y: Array(16).fill(0x7e)}),
    call({fx: 1, fy: 1, R: 4, Bk: 2, ow: 700, rep: 1, seed: 13}),
    call({fx: 3, fy: 3, R: 4, Bk: 2, ow: 800, rep: 1, seed: 17}),
    call({fx: 0, fy: 3, R: 6, Bk: 2, rep: 1, seed: 19}),
    call({fx: 2, fy: 0, R: 3, Bk: 1, ow: 900, rep: 1, seed: 23}),
    call({fx: 1, fy: 3, R: 2, Bk: 2, rep: 1, seed: 29}),
    call({fx: 3, fy: 0, R: 0, Bk: 2 ** 30 + 2, rep: 1, xw: 2 ** 32, yw: 2 ** 31, ow: 2 ** 29}),
];
for (const [k, c] of calls.entries()) {
    const r0 = c.rows[0], fx = r0.fx[0] + 2 * r0.fx[1], fy = r0.fy[0] + 2 * r0.fy[1], n = 8 * r0.R;
    if (n) {
        const [total, sums] = reference(fx, c.x, fy, c.y, n, 8 * r0.Bk, r0.rep);
        assert.equal(c.total, total, `call ${k} total`);
        assert.deepEqual(c.blocks, sums, `call ${k} blocks`);
    }
}
assert.equal(calls[5].total, -16n * BigInt(448 << 9) ** 2n);
assert.ok(calls.some(c => c.total < 0n) && calls.some(c => c.total > 0n));
const all = () => calls.flatMap(c => structuredClone(c.rows));
ok(all());

const forge = (what, f) => {
    const rows = all();
    f(rows);
    rejects(rows, what);
};
// Row 0..3: E4M3 x E4M3, 2 blocks to out. Rows 4..7: E2M1 x E4M3. Rows 8..10: int8, Bk = 1.
forge('wrong decoded value', r => { r[1].xv[3] = mod(BigInt(r[1].xv[3]) + 1n); });
forge('wrong code for the word', r => { r[1].xc[0] = (r[1].xc[0] + 1) % 256; });
forge('total off by one', r => { for (const x of r.slice(0, 4)) x.cl[0] = BigInt(x.cl[0]) ^ 1n; });
forge('negative total as unsigned', r => {
    const u = BigInt.asUintN(64, calls[5].total) - 2n ** 64n + P; // p + v, a valid u64 below 2^64
    const lo = u & 0xffffffffn, hi = u >> 32n;
    for (const x of r.slice(14, 16)) Object.assign(x, {cl: [lo & 0xffffn, lo >> 16n], ch: [hi & 0xffffn, hi >> 16n], csg: 0});
});
forge('block sum wrong', r => { r[1].bl[1] = BigInt(r[1].bl[1]) ^ 4n; });
forge('call ends early', r => { r[2].end = 1; r[2].ce = 1; });
forge('block does not divide', r => { for (const x of r.slice(8, 11)) x.Bk = 2; });
forge('E2M1 skips a word', r => { r[5].xw += 1; });
forge('E2M1 wrong half', r => { r[5].par = 0; r[5].lx = 1; });
forge('write with out = 0', r => { r[7].w = 1; });
forge('block sum not reset', r => { r[2].accb = r[1].s; r[2].s = mod(BigInt(r[1].s) + (r[2]._s)); });
forge('padding row active', r => { r.push(Object.assign(padding(), {active: 1})); });
// Rows 16..19: rep E4M3 x E4M3, Bk = 2. Rows 20..23: rep E2M1 x E2M1, Bk = 2. Rows 18 and 22
// start the second block, so they read y from yb again.
forge('rep: y not reset, reads on', r => { r[18].yr = 0; r[18].yw = r[17].yw + 1; });
forge('rep: restart away from yb', r => { r[18].yw = r[16].yw + 1; });
forge('rep: E2M1 y not reset', r => { r[22].yr = 0; r[22].yw = r[21].yw + 1; });
forge('rep: yb not the y address', r => { for (const x of r.slice(16, 20)) x.yb += 1; });
forge('rep: flag dropped mid-call', r => { for (const x of r.slice(18, 20)) { x.rep = 0; x.yr = 0; } });
forge('rep: flag set on a plain call', r => { for (const x of r.slice(0, 4)) x.rep = 1; });
forge('rep: block word 0', r => { for (const x of r.slice(16, 20)) x.bz = 1; });
forge('rep: restart without a block start', r => { r[17].yr = 1; r[17].yw = r[16].yw; });
const odd = call({fx: 1, fy: 3, R: 2, Bk: 1, rep: 1, seed: 31}); // E2M1 y of 8 values: half a word
rejects(all().concat(odd.rows), 'rep: E2M1 y block of 8 values');
console.log(`dot_f8 constraints: ${calls.length} calls honest, ${rejected} forgeries rejected, decode table ${entries.size} entries`);
