// Evaluate actual compiled components, including malformed and forged witnesses.
const assert=require('node:assert/strict');
const path=require('node:path');
const {P,bits,compile,checker}=require('./pil_test.cjs');
const pil=compile(path.join(__dirname,'components.pil'),'components');
let valid = 0, rejected = 0;
for (const [airId,air] of pil.airGroups[0].airs.entries()) {
    const {symbols,witness,accepts}=checker(pil,airId);
    const inputs = {Multiply:["a","b"],Add:["a","b","subtract"],Round:["sig"],Shift:["input","amount"],Decode:["raw"],Normalize:["raw"],RoundPack:["sig","exponent","sign"]}[air.name];
    const inputColumns = new Set();
    for(const s of symbols) if(inputs.includes(s.name.split(".").at(-1))) {
        for(let i=0;i<s.lengths.reduce((a,b)=>a*b,1);i++) inputColumns.add(s.id+i);
    }
    function check(values, mutate=false) {
        const row=witness(values);
        assert.ok(accepts(row), `${air.name} valid witness rejected`); valid++;
        if(mutate) for(let i=0;i<row.length;i++) {
            // Valid input changes may leave a rounded output unchanged. Input/memory binding
            // belongs to the eventual calling AIR; here reject malformed bits and forged outputs.
            for(const bad of (inputColumns.has(i)?[2n,P-1n]:[row[i]^1n,2n,P-1n])) {
                if(bad===row[i]) continue;
                const forged=row.slice(); forged[i]=bad;
                assert.ok(!accepts(forged),`${air.name} forged column ${i}=${bad} accepted`); rejected++;
            }
        }
    }
    if(air.name==='Multiply') {
        for(const a of [0n,1n,0x800000n,0xffffffn]) for(const b of [0n,1n,0x800001n,0xffffffn])
            check({a:bits(a,24),b:bits(b,24),product:bits(a*b,48)},a===0xffffffn&&b===0xffffffn);
    } else if(air.name==='Add') {
        for(const a of [0n,1n,(1n<<27n)-1n]) for(const b of [0n,1n,(1n<<27n)-1n])
            for(const subtract of [0n,1n]) if(!subtract || a>=b)
                check({a:bits(a,27),b:bits(b,27),subtract,result:bits(subtract?a-b:a+b,28)},a===((1n<<27n)-1n)&&b===1n);
    } else if(air.name==='Round') {
        for(const hi of [0n,1n,2n,(1n<<23n)-1n,(1n<<24n)-1n]) for(let tail=0n;tail<8n;tail++) {
            const sig=(hi<<3n)|tail;
            const rounded=hi+BigInt(tail>4n||(tail===4n&&(hi&1n)!==0n));
            check({sig:bits(sig,27),rounded:bits(rounded,25),inexact:BigInt(tail!==0n)},hi===1n||hi===((1n<<24n)-1n));
        }
    } else if(air.name==='Shift') {
        for(const input of [0n,1n,0x800000000000n,0xffffffffffffn,0x123456789abcn]) for(let n=0;n<64;n++) {
            let value=input; const stages=[],sticky=[];
            for(let j=0;j<6;j++) {
                const distance=1<<j; const original=bits(value,48); let any=0n;
                sticky.push(Array.from({length:33},(_,k)=>{
                    if(k>distance)return 0n;
                    any|=original[k]; return any;
                }));
                if(n&distance) value=(value>>BigInt(distance))|BigInt((value&((1n<<BigInt(distance))-1n))!==0n);
                stages.push(bits(value,48));
            }
            check({input:bits(input,48),amount:bits(n,6),stages,sticky},input===0x123456789abcn&&n===37);
        }
    } else if(air.name==='Decode') {
        const rawValues=[0,1,0x007fffff,0x00800000,0x3f800000,0x7f7fffff,0x7f800000,0x7f800001,0x7fc00000,0x7fffffff];
        for(const rawValue of rawValues) for(const sign of [0,1]) {
            const raw=bits(BigInt(rawValue)|(BigInt(sign)<<31n),32);
            const exponent=(rawValue>>>23)&255, fraction=rawValue&0x7fffff;
            const sig=bits(fraction+(exponent?0x800000:0),24);
            const chain=(xs,initial,fn)=>{const res=[initial]; for(const x of xs)res.push(fn(res.at(-1),x)); return res;};
            const exp_zero=chain(raw.slice(23,31),1n,(a,b)=>a*(1n-b));
            const exp_ones=chain(raw.slice(23,31),1n,(a,b)=>a*b);
            const frac_zero=chain(raw.slice(0,23),1n,(a,b)=>a*(1n-b));
            const classes=[exponent===0&&fraction===0,exponent===0&&fraction!==0,
                exponent>0&&exponent<255,exponent===255&&fraction===0,
                exponent===255&&fraction>=0x400000,exponent===255&&fraction>0&&fraction<0x400000].map(BigInt);
            check({raw,sig,sign,exponent,classes,exp_zero,exp_ones,frac_zero},true);
        }
    } else if(air.name==='Normalize') {
        for(const value of [0n,1n,2n,3n,0xffffffn,0x1000000n,0x7ffffffn,0x8000000n,0x800000000000n,0xffffffffffffn,0x123456789abcn]) {
            const raw=bits(value,48), h=value===0n?0:value.toString(2).length-1;
            const normalized=bits(value<<BigInt(47-h),48);
            const prefix=Array(49).fill(0n),leading=Array(48).fill(0n); prefix[48]=1n;
            for(let i=47;i>=0;i--) { leading[i]=raw[i]*prefix[i+1]; prefix[i]=prefix[i+1]*(1n-raw[i]); }
            check({raw,normalized,high_index:bits(h,6),prefix,leading},true);
        }
    } else if(air.name==='RoundPack') {
        for(const e of [-298,-150,-149,-148,-128,-127,-126,-125,-1,0,126,127,128,255])
        for(const sig of [0n,0x4000000n,0x4000001n,0x4000004n,0x400000cn,0x7fffff8n,0x7fffffcn,0x7ffffffn])
        for(const sign of [0n,1n]) {
            const eb=e+512, under=Number(e < -126), distance=under?-126-e:0;
            const n=Math.min(63,distance); let value=sig; const stages=[],sticky=[];
            for(let j=0;j<6;j++) {
                const d=1<<j, original=bits(value,48); let any=0n;
                sticky.push(Array.from({length:33},(_,k)=>{if(k>d)return 0n;any|=original[k];return any;}));
                if(n&d)value=(value>>BigInt(d))|BigInt((value&((1n<<BigInt(d))-1n))!==0n);
                stages.push(bits(value,48));
            }
            const tail=value&7n, hi=value>>3n;
            const rounded=hi+BigInt(tail>4n||(tail===4n&&(hi&1n)!==0n));
            const carry=Number(rounded>>24n), overflow=sig!==0n&&eb+carry>=640;
            const tiny=under && (e < -127 || sig < 0x7fffffcn);
            const inexact=BigInt(tail!==0n);
            const top_ones=[1n], sb=bits(sig,27);
            for(let k=2;k<27;k++)top_ones.push(top_ones.at(-1)*sb[k]);
            const effectiveExponent=Math.max(e,-126);
            let mantissa=rounded>>BigInt(carry);
            const outExp=overflow?255:(mantissa >= 0x800000n ? effectiveExponent+127+carry:0);
            const result=(sign<<31n)|(BigInt(outExp)<<23n)|(overflow?0n:mantissa&0x7fffffn);
            const f=BigInt(overflow?5:(Number(inexact)|(tiny&&inexact?2:0)));
            check({sig:sb,exponent:bits(eb,10),sign,exp_lower:bits(eb-214,10),exp_upper:bits(767-eb,10),
                under_diff:bits(eb-386+1024,11),tiny_diff:bits(eb-385+1024,11),
                shift_amount:bits(distance,8),shift_bits:bits(n,6),stages,sticky,rounded:bits(rounded,25),
                inexact,top_ones,overflow_diff:bits(eb+carry-640+1024,11),result:bits(result,32),flags:bits(f,5)},
                sign===1n&&((e===-127&&sig===0x7fffff8n)||(e===127&&sig===0x7ffffffn)));
        }
    } else throw new Error('unexpected air '+air.name);
}
assert.ok(valid>300 && rejected>500);
console.log(`PIL components: ${valid} valid rows; ${rejected} forged witnesses rejected (no STARK proof).`);
