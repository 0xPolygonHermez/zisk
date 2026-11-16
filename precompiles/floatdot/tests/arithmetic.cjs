const assert=require('node:assert/strict');
const path=require('node:path');
const fs=require('node:fs');
const {spawnSync}=require('node:child_process');
const {root,P,compile,checker}=require('./pil_test.cjs');
const {arithmetic}=require('./arithmetic_witness.cjs');
const oraclePath=path.join(root,'target/fp32/softfloat-oracle');
assert.ok(fs.existsSync(oraclePath),'Build the reference oracle first: node precompiles/helpers/tests/fp32_oracle.cjs');
const pil=compile(path.join(__dirname,'arithmetic.pil'),'arithmetic');
const check=checker(pil,0);
const edges=[0,1,3,0x007fffff,0x00800000,0x3f000000,0x3f7fffff,0x3f800000,
    0x3f800001,0x7f7fffff,0x7f800000,0x7f800001,0x7fc00000];
const pairs=[];
for(const a of edges)for(const b of edges)for(let signs=0;signs<4;signs++)
    pairs.push([BigInt(a)|(BigInt(signs&1)<<31n),BigInt(b)|(BigInt(signs>>1)<<31n)]);
let seed=0x4d595df4d0f33173n;
for(let i=0;i<1000;i++) {
    seed^=seed<<13n;seed=BigInt.asUintN(64,seed);seed^=seed>>7n;seed^=seed<<17n;seed=BigInt.asUintN(64,seed);
    pairs.push([seed&0xffffffffn,seed>>32n]);
}
const oracle=spawnSync(oraclePath,[],{
    input:pairs.map(([a,b])=>`${a.toString(16)} ${b.toString(16)}\n`).join(''),encoding:'utf8',maxBuffer:16*1024*1024});
assert.equal(oracle.status,0,oracle.stderr||String(oracle.error));
const lines=oracle.stdout.trim().split('\n');assert.equal(lines.length,pairs.length);
const rustRows=process.env.FP32_WITNESS_DUMP?fs.readFileSync(process.env.FP32_WITNESS_DUMP,'utf8').trim().split('\n').map(line=>line.split(',').map(BigInt)):null;
if(rustRows)assert.equal(rustRows.length,pairs.length*2);
let valid=0,rejected=0;
for(let i=0;i<pairs.length;i++)for(const is_mul of [0,1]) {
    const [a,b]=pairs[i],expected=lines[i].split(/\s+/).map(s=>BigInt('0x'+s));
    const reference=check.witness(arithmetic(a,b,is_mul,expected[is_mul*2],expected[is_mul*2+1]));
    const row=rustRows?rustRows[i*2+is_mul]:reference;
    if(rustRows)assert.deepEqual(row,reference,`Rust witness row ${i*2+is_mul}`);
    assert.ok(check.accepts(row),`${is_mul?'mul':'add'} ${a.toString(16)} ${b.toString(16)}: ${check.failures(row).slice(0,5)}`);valid++;
    if((a===0x3f800001n&&b===0x3f7fffffn)||(a===0x00800000n&&b===0x3f7fffffn)||
       (a===0n&&b===0x7f800000n)) {
        for(const s of check.symbols) {
            const name=s.name.split('.').at(-1),input=['a','b','is_mul'].includes(name);
            const len=s.lengths.reduce((a,b)=>a*b,1);
            for(let k=0;k<len;k++)for(const bad of (input?[2n,P-1n]:[row[s.id+k]^1n,2n,P-1n])) {
                if(bad===row[s.id+k])continue;
                const forged=row.slice();forged[s.id+k]=bad;
                assert.ok(!check.accepts(forged),`forged ${name}[${k}]=${bad} accepted`);rejected++;
            }
        }
    }
}
assert.ok(rejected>500);
console.log(`FP32 arithmetic PIL: ${valid} SoftFloat-checked rows, ${rejected} forged witnesses rejected; no STARK/VM proof.`);
