// Test-only evaluator for compiled polynomial equations, not a STARK verifier.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const {createRequire}=require('node:module');
const {execFileSync}=require('node:child_process');
const root=path.resolve(__dirname,'../../..');
const modules=path.join(root,'target/fp32/pil-tools/node_modules');
assert.ok(fs.existsSync(path.join(modules,'pil2-compiler/package.json')),
    'Install constraint-test tooling from the repository root: npm install --prefix target/fp32/pil-tools --no-save git+https://github.com/0xPolygonHermez/pil2-compiler.git#503862ca92a7493623bca1bc331cdbfb9398e435');
const protobuf=createRequire(path.join(modules,'pil2-compiler/package.json'))('protobufjs');
const P=0xffffffff00000001n;
// Constraints on later-stage columns (the std lookup and range-check sums) are skipped.
const SKIP=Symbol('later stage');
const scalar=b=>b.length?BigInt('0x'+Buffer.from(b).toString('hex')):0n;
const bits=(n,width)=>Array.from({length:width},(_,i)=>(BigInt(n)>>BigInt(i))&1n);
function compile(source,name) {
    const out=path.join(root,'target/fp32',name+'.pilout');
    fs.rmSync(out,{force:true});
    execFileSync(path.join(modules,'.bin/pil2com'),[source,'-o',out,'-I',[path.join(root,'precompiles'),path.join(root,'pil'),process.env.PIL_STD].filter(Boolean).join(',')],{stdio:'inherit'});
    const schema=protobuf.loadSync(path.join(modules,'pil2-compiler/src/pilout.proto')).lookupType('pilout.PilOut');
    const pil=schema.decode(fs.readFileSync(out));
    assert.equal(scalar(pil.baseField),P);
    return pil;
}
function checker(pil,airId,airGroupId=0) {
    const air=pil.airGroups[airGroupId].airs[airId];
    const symbols=pil.symbols.filter(s=>s.type===3&&s.stage===1&&s.airGroupId===airGroupId&&s.airId===airId);
    function witness(values) {
        const row=Array(air.stageWidths[0]).fill(undefined);
        for(const symbol of symbols) {
            const name=symbol.name.split('.').at(-1);
            assert.ok(Object.hasOwn(values,name),`missing ${symbol.name}`);
            const value=Array.isArray(values[name])?values[name].flat(Infinity):[values[name]];
            const len=symbol.lengths.reduce((a,b)=>a*b,1);
            assert.equal(value.length,len,`width ${symbol.name}`);
            for(let i=0;i<len;i++)row[symbol.id+i]=BigInt(value[i]);
        }
        assert.ok(row.every(v=>v!==undefined));
        return row;
    }
    function failures(row,firstOnly=false,context={}) {
        const cache=[];
        function operand(o) {
            if(o.constant)return scalar(o.constant.value);
            if(o.expression)return expression(o.expression.idx);
            if(o.fixedCol) {
                assert.ok(Number.isInteger(context.rowIndex));
                const values=air.fixedCols[o.fixedCol.idx].values;
                const index=(context.rowIndex+o.fixedCol.rowOffset+values.length)%values.length;
                return scalar(values[index]);
            }
            if(o.witnessCol) {
                if(o.witnessCol.stage!==1)throw SKIP;
                const offset=o.witnessCol.rowOffset;
                const target=offset===0?row:context.neighbors?.[offset];
                assert.ok(target,`missing neighbor ${offset}`);
                return target[o.witnessCol.colIdx];
            }
            if(o.airGroupValue||o.airValue||o.challenge)throw SKIP;
            throw new Error('unsupported operand '+JSON.stringify(o));
        }
        function expression(i) {
            if(cache[i]!==undefined)return cache[i];
            const e=air.expressions[i];let v;
            if(e.add)v=operand(e.add.lhs)+operand(e.add.rhs);
            else if(e.sub)v=operand(e.sub.lhs)-operand(e.sub.rhs);
            else if(e.mul)v=operand(e.mul.lhs)*operand(e.mul.rhs);
            else if(e.neg)v=-operand(e.neg.value);
            else throw new Error('unsupported expression');
            return cache[i]=(v%P+P)%P;
        }
        const failed=[];
        for(const c of air.constraints) {
            assert.ok(c.everyRow,'component constraint must apply to every row');
            let v;
            try { v=expression(c.everyRow.expressionIdx.idx); } catch(e) { if(e===SKIP)continue; throw e; }
            if(v!==0n) {
                failed.push(c.everyRow.debugLine); if(firstOnly)break;
            }
        }
        return failed;
    }
    return {air,symbols,witness,failures,accepts:(row,context)=>failures(row,true,context).length===0};
}
module.exports={root,P,bits,compile,checker};
