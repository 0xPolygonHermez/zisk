// Evaluate rows emitted by the production Rust state machine against compiled PIL.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const {compile,checker}=require('./pil_test.cjs');
const file=process.env.FP32_DOT_WITNESS_DUMP;
assert.ok(file,'set FP32_DOT_WITNESS_DUMP to the production state-machine CSV');
const check=checker(compile(path.join(__dirname,'dot.pil'),'dot-rust'),0);
const rows=new Map();
for(const line of fs.readFileSync(file,'utf8').trim().split('\n')) {
    const [testCase,index,...values]=line.split(','),row=values.map(BigInt);
    assert.ok(row.length>=check.air.stageWidths[0],'production trace lost component columns');
    rows.set(`${testCase}:${index}`,row.slice(0,check.air.stageWidths[0]));
}
let valid=0,rejected=0;
const width=check.air.stageWidths[0];
for(const [key,row] of rows) {
    const [testCase,indexText]=key.split(':'),index=Number(indexText);
    const previous=rows.get(`${testCase}:${(index+16383)%16384}`);
    if(!previous)continue;
    const context={rowIndex:index,neighbors:{'-1':previous}};
    assert.ok(check.accepts(row,context),`${key}: ${check.failures(row,false,context).slice(0,8)}`);valid++;
    if(row[check.symbols.find(s=>s.name.split('.').at(-1)==='end').id]===1n)for(const name of ['returned','returned_flags']) {
        const symbol=check.symbols.find(s=>s.name.split('.').at(-1)===name);
        const n=symbol.lengths.reduce((a,b)=>a*b,1);
        for(let k=0;k<n;k++) {
            const forged=row.slice();forged[symbol.id+k]^=1n;
            assert.ok(!check.accepts(forged,context),`${key}: forged ${name}[${k}]`);rejected++;
        }
    }
    if(index===1)for(let k=0;k<width;k++) {
        if(row[k]===2n)continue;
        const forged=row.slice();forged[k]=2n;
        assert.ok(!check.accepts(forged,context),`${key}: malformed column ${k}`);rejected++;
    }
}
assert.ok(valid>=20,`only ${valid} rows had predecessors`);
assert.ok(rejected>5000,`only ${rejected} forged witnesses tested`);
console.log(`Production Rust dot PIL: ${valid} valid rows, ${rejected} forged witnesses rejected; no bus/VM proof.`);
