const assert=require('node:assert/strict');
const path=require('node:path');
const {bits,compile,checker}=require('./pil_test.cjs');
const {arithmetic,pack}=require('./arithmetic_witness.cjs');
const pil=compile(path.join(__dirname,'dot.pil'),'dot');
const check=checker(pil,0);
function operation(a,b,mul) {
    const w=arithmetic(a,b,mul,0,0);
    let result=pack(w.fp_round_result),flags=pack(w.fp_round_flags);
    if(w.fp_nan) { result=0x7fc00000n;flags=w.fp_invalid*16n; }
    else if(w.fp_inf) {
        const sign=mul?w.fp_sign_xor:(w.fp_a_cls[3]?w.fp_a_sign:w.fp_b_sign);
        result=0x7f800000n|(sign<<31n);flags=0n;
    }
    w.result=bits(result,32);w.flags=bits(flags,5);
    return {w,result,flags};
}
const widen=[null,h=>BigInt(h)<<16n,h=>{
    h=BigInt(h);const s=(h>>15n)<<31n,e=(h>>10n)&31n,m=h&0x3ffn;
    if(e===0n&&m===0n)return s;
    if(e===0n){const k=BigInt(m.toString(2).length-1);return s|((103n+k)<<23n)|(((m-(1n<<k))<<(23n-k))&0x7fffffn);}
    return s|((e===31n?255n:e+112n)<<23n)|(m<<13n);
}];
// Sparse memory: every lane not written by a 16-bit vector holds a NaN pattern.
function memoryOf(vectors) {
    const words=new Map(),get=a=>words.get(a)??0x7fa17fa17fa17fa1n;
    for(const {base,stride,values} of vectors)values.forEach((h,i)=>{
        const a=base+2*stride*i,w=a-a%8,sh=BigInt(16*((a>>>1)&3));
        words.set(w,(get(w)&~(0xffffn<<sh))|(BigInt(h)<<sh));
    });
    return get;
}
// One vector's per-row operand columns; 16-bit lanes come from the sparse memory.
function operand(v,term,n,format,base,stride,memory) {
    let waddr,lane,word;
    if(format===0) {
        const b=term&~1;waddr=(base>>>3)+(term>>1);lane=2*(term&1);
        word=(v[b]??0n)|((v[b+1]??(b<n?0xdeadbeefn:0n))<<32n);
    } else {
        const a=base+2*stride*term;waddr=Math.floor(a/8);lane=(a>>>1)&3;word=n?memory(waddr*8):0n;
    }
    const lo=word&0xffffffffn,hi=word>>32n,half=lane&2?hi:lo,sel=(lane&1?half>>16n:half)&0xffffn;
    const op=format===0?half:widen[format](sel),e=(sel>>10n)&31n,m=sel&0x3ffn;
    const lead=Array(10).fill(0n);
    if(format===2&&e===0n&&m!==0n)lead[m.toString(2).length-1]=1n;
    return {word:[bits(lo,32),bits(hi,32)],waddr:bits(waddr,29),lane:bits(lane,2),sel:bits(sel,16),
        op:bits(op,32),lead,ez:BigInt(e===0n),eo:BigInt(e===31n),value:op};
}
function scenario(x,y,initial=0n,initialFlags=0n,inUse=1n,
        {format=0,xbase=0xa0002000,ybase=0xa0003000,sx=0,sy=0}={}) {
    assert.equal(x.length,y.length);const n=x.length;
    const memory=memoryOf([{base:xbase,stride:sx,values:x},{base:ybase,stride:sy,values:y}]);
    const xs=format?x.map(widen[format]):x,ys=format?y.map(widen[format]):y;
    const states=[];let acc=initial,sticky=initialFlags;
    for(let j=0;j<n;j++) {
        const mul=operation(xs[j],ys[j],1);states.push({a:xs[j],b:ys[j],mul:1,acc,sticky});sticky|=mul.flags;
        const add=operation(acc,mul.result,0);states.push({a:acc,b:mul.result,mul:0,acc,sticky});sticky|=add.flags;acc=add.result;
    }
    const final=acc,finalFlags=sticky,cache=new Map();
    const meta={in_use:inUse,descriptor:bits(0xa0001000n>>3n,29),x_half:bits(xbase/2,31),
        y_half:bits(ybase/2,31),x_stride:bits(sx,16),y_stride:bits(sy,16),bf16:BigInt(format===1),
        fp16:BigInt(format===2),step:bits(17,36),count:bits(n,13),initial:bits(initial,32),
        initial_flags:bits(initialFlags,5),returned:bits(final,32),returned_flags:bits(finalFlags,5),
        descriptor_end:bits((0xa0001000n>>3n)+4n,29),length_bound:bits(4096-n,13),length_zero:[1n]};
    for(const bit of meta.count)meta.length_zero.push(meta.length_zero.at(-1)*(1n-bit));
    function row(i) {
        assert.ok(i>=0&&i<2*Math.max(n,1));if(cache.has(i))return cache.get(i);
        const term=Math.floor(i/2);
        const state=states[i]||{a:0n,b:0n,mul:1-(i&1),acc:final,sticky:finalFlags};
        const op=operation(state.a,state.b,state.mul);
        const ox=operand(x,term,n,format,xbase,sx,memory),oy=operand(y,term,n,format,ybase,sy,memory);
        const vec=(p,o)=>({[p+'_waddr']:o.waddr,[p+'_lane']:o.lane,[p+'_sel']:o.sel,[p+'_op']:o.op,
            [p+'_lead']:o.lead,[p+'_ez']:o.ez,[p+'_eo']:o.eo});
        const remaining=bits(n-term-1+8192,14),last_zero=[1n];
        for(const bit of remaining.slice(0,13))last_zero.push(last_zero.at(-1)*(1n-bit));
        const r=check.witness({...op.w,...meta,start:BigInt(i===0),end:BigInt(i+1===2*Math.max(n,1)),
            index:bits(term,12),last_zero,remaining,acc:bits(state.acc,32),
            sticky:bits(state.sticky,5),x_word:ox.word,y_word:oy.word,...vec('x',ox),...vec('y',oy),
            load:BigInt((i&1)===0&&(format!==0||(term&1)===0))});
        cache.set(i,r);if(cache.size>4)cache.delete(cache.keys().next().value);return r;
    }
    return {row,final,finalFlags,n,size:2*Math.max(n,1),format};
}
let valid=0,rejected=0;
function verify(s,i) {
    const row=s.row(i),context={rowIndex:i,neighbors:{'-1':s.row(i-1)}};
    assert.ok(check.accepts(row,context),`row ${i}: ${check.failures(row,false,context).slice(0,8)}`);valid++;
    return {row,context};
}
const cases=[
    scenario([],[],0x7f800001n,8n),
    scenario([0x4b800000n,0x3f800000n,0xcb800000n],[0x3f800000n,0x3f800000n,0x3f800000n]),
    scenario([0x00010000n,0x00010000n],[0x37800000n,0x37000000n]),
    scenario([0x3f800000n,0xc0000000n,0x40400000n],[0x3f000000n,0x3e800000n,0xbe000000n]),
    scenario([0n],[0x7f800000n]),
    scenario([0x80000000n],[0x3f800000n],0x80000000n,8n),
    scenario([],[],0n,0n,0n),
];
assert.equal(cases[0].final,0x7f800001n);assert.equal(cases[0].finalFlags,8n);
assert.equal(cases[1].final,0n);assert.equal(cases[1].finalFlags,1n);
assert.equal(cases[2].final,1n);assert.equal(cases[2].finalFlags,3n);
assert.equal(cases[3].final,0xbec00000n);
function trace(calls) {
    const offsets=[];let used=0;
    for(const call of calls) { offsets.push(used);used+=call.size; }
    assert.ok(used<=16384);
    return {row(i) {
        i=(i+16384)%16384;
        if(i>=used)return cases[6].row(i%2);
        for(let c=calls.length-1;c>=0;c--)if(i>=offsets[c])return calls[c].row(i-offsets[c]);
    },offsets,used};
}
const maximum=scenario(Array(4096).fill(0x3f800000n),Array(4096).fill(0x3f800000n));
assert.equal(maximum.final,0x45800000n);assert.equal(maximum.finalFlags,0n);
const short=scenario(Array(128).fill(0x3f800000n),Array(128).fill(0x3f800000n));
const decode=scenario(Array(749).fill(0x3f800000n),Array(749).fill(0x3f800000n));
// 16-bit inputs: unaligned contiguous start, V-like stride 1024, broadcast, exceptional values.
const bfX=[0x3f80,0x4000,0xc040,0x3f00,0x0001,0x8000,0x7f81,0xff80,0x7fc1];
const bfY=[0x4040,0x3f80,0x3f80,0x4080,0x7f80,0x0000,0x3f80,0x3f80,0x3f80];
const bf16=scenario(bfX,bfY,0n,0n,1n,{format:1,xbase:0xa0002002,sx:1,ybase:0xa0004006,sy:1024});
assert.equal(bf16.final,0x7fc00000n);assert.equal(bf16.finalFlags,16n);
const fpX=[0x3c00,0x0001,0x03ff,0x8000,0x7bff,0x7c00,0xfc00,0x7d00];
const fpY=[0x3c00,0x0001,0x3c00,0x0000,0x7bff,0x3c00,0x3c00,0x3c00];
const fp16=scenario(fpX,fpY,0x3f800000n,8n,1n,{format:2,xbase:0xa0005000,sx:3,ybase:0xa0005ffe,sy:1});
assert.equal(fp16.final,0x7fc00000n);assert.equal(fp16.finalFlags,8n|16n|1n); // inf-inf and sNaN: NV; 65504^2 fits
const fpFinite=scenario(fpX.slice(0,4),fpY.slice(0,4),0n,0n,1n,{format:2,xbase:0xa0005000,sx:3,ybase:0xa0005ffe,sy:1});
// RN(RN(1 + 2^-48) + 1023*2^-24) ties to even at 1 + 2^-14; -0*0 adds nothing.
assert.equal(fpFinite.final,0x3f800200n);assert.equal(fpFinite.finalFlags,1n);
const broadcast=scenario(Array(5).fill(0x3c00),[0x3c00,0x4000,0x4200,0x4400,0x4500],0n,0n,1n,
    {format:2,xbase:0xa0006006,sx:0,ybase:0xa0007000,sy:1});
assert.equal(broadcast.final,0x41700000n); // 1+2+3+4+5
const bfLong=scenario(Array(1000).fill(0x3f80),Array(1000).fill(0x3f80),0n,0n,1n,
    {format:1,xbase:0xa0010000,sx:7,ybase:0xa0020002,sy:1024});
assert.equal(bfLong.final,0x447a0000n);
const bfEmpty=scenario([],[],0xff800001n,24n,1n,{format:1,xbase:0xfffffffe,sx:0xffff});
const sixteen=[bf16,fp16,fpFinite,broadcast,bfLong,bfEmpty];
const mixedCalls=[...cases.slice(0,6),maximum,cases[0],cases[1],short,decode,...sixteen];
const mixed=trace(mixedCalls);
for(const [calls,t] of [[mixedCalls,mixed],[[maximum,maximum],trace([maximum,maximum])],
    [Array(64).fill(short),trace(Array(64).fill(short))]]) {
    for(let c=0;c<calls.length;c++) {
        const call=calls[c],offset=t.offsets[c];
        for(const j of new Set([0,1,2,3,Math.floor(call.size/2),call.size-2,call.size-1]))
            if(j>=0&&j<call.size)verify(t,offset+j);
    }
    for(const i of [t.used,t.used+1,16382,16383])if(i<16384)verify(t,i);
}
// Exhaust every multiply/add transition and every padding boundary in the mixed trace.
for(let i=0;i<16384;i++)verify(mixed,i);
for(const i of [...Array(22).keys(),mixed.used,16383]) {
    const {row,context}=verify(mixed,i);
    for(const name of ['a','b','is_mul','result','flags','in_use','descriptor','x_half','y_half','step','count',
        'initial','initial_flags','returned','returned_flags','remaining','acc','sticky','x_word','y_word',
        'start','end','index','last_zero','bf16','fp16','x_stride','y_stride','load','x_waddr','x_lane',
        'x_sel','x_op','x_lead','x_ez','x_eo','y_waddr','y_lane','y_sel','y_op','y_lead','y_ez','y_eo']) {
        const symbol=check.symbols.find(s=>s.name.split('.').at(-1)===name);assert.ok(symbol,name);
        const length=symbol.lengths.reduce((a,b)=>a*b,1);
        for(let k=0;k<length;k++) {
            const forged=row.slice();forged[symbol.id+k]=2n;
            assert.ok(!check.accepts(forged,context),`malformed ${name}[${k}] row ${i}`);rejected++;
        }
    }
}
// Flip well-formed bits as well: premature starts/ends and shifted term indices must fail.
for(const i of [...Array(22).keys(),mixed.used,16383]) {
    const {row,context}=verify(mixed,i);
    for(const name of ['start','end','index']) {
        const symbol=check.symbols.find(s=>s.name.split('.').at(-1)===name);
        const forged=row.slice();forged[symbol.id]^=1n;
        assert.ok(!check.accepts(forged,context),`forged ${name} row ${i}`);rejected++;
    }
}
// Call metadata cannot change on an interior row.
for(const name of ['descriptor','x_half','y_half','step','count','initial','initial_flags','returned','returned_flags',
    'bf16','fp16','x_stride','y_stride']) {
    const {row,context}=verify(mixed,3),symbol=check.symbols.find(s=>s.name.split('.').at(-1)===name);
    const forged=row.slice();forged[symbol.id]^=1n;assert.ok(!check.accepts(forged,context),name);rejected++;
}
// Locally valid interior rows cannot occupy the global first/last boundary.
for(const [rowIndex,localIndex] of [[0,2],[16383,3]]) {
    const row=maximum.row(localIndex),context={rowIndex,neighbors:{'-1':maximum.row(localIndex-1)}};
    assert.ok(!check.accepts(row,context),`truncated call at ${rowIndex}`);rejected++;
}
// 16-bit rows: every derived operand column is pinned by the format, address and lane.
const symbol=name=>check.symbols.find(s=>s.name.split('.').at(-1)===name);
function forge(t,i,name,k,value) {
    const {row,context}=verify(t,i),sym=symbol(name),forged=row.slice();
    forged[sym.id+k]=value===undefined?forged[sym.id+k]^1n:value;
    return {forged,context};
}
const sixteenStart=mixed.offsets[mixedCalls.indexOf(bf16)];
const sixteenEnd=mixed.offsets[mixedCalls.indexOf(bfEmpty)]+2;
for(let i=sixteenStart;i<sixteenEnd;i++) {
    if(i>sixteenStart+40&&i<sixteenEnd-40&&i%97)continue;
    for(const name of ['bf16','fp16','x_op','y_op','x_sel','y_sel','x_lane','y_lane','x_waddr','y_waddr',
        'x_lead','y_lead','x_ez','x_eo','y_ez','y_eo','load']) {
        const len=symbol(name).lengths.reduce((a,b)=>a*b,1);
        for(let k=0;k<len;k++) {
            const {forged,context}=forge(mixed,i,name,k);
            assert.ok(!check.accepts(forged,context),`forged ${name}[${k}] row ${i}`);rejected++;
        }
    }
}
// Exclusive formats; FP32 forbids strides and sub-word alignment, even on a call's first row.
{
    const start=mixed.offsets[mixedCalls.indexOf(bf16)];
    const {forged,context}=forge(mixed,start,'fp16',0,1n);
    assert.ok(!check.accepts(forged,context),'bf16 and fp16');rejected++;
    for(const [name,k] of [['x_stride',0],['y_stride',15],['x_half',0],['x_half',1],['y_half',1]]) {
        const {forged,context}=forge(mixed,0,name,k);
        assert.ok(!check.accepts(forged,context),`FP32 ${name}[${k}]`);rejected++;
    }
}
// Exhaustive FP16 zero/subnormal/inf/NaN encodings and sampled normals and BF16 through the AIR.
let widened=0,seed=0x1234567;
const next=()=>(seed=(seed*1103515245+12345)&0x7fffffff)&0xffff;
for(const [format,values] of [[2,[...Array(2048).keys()].flatMap(m=>[m&0x3ff|(m&0x400)<<5,m&0x3ff|0x7c00|(m&0x400)<<5])],
    [2,Array.from({length:1024},next)],[1,Array.from({length:1024},next)]]) {
    for(const h of values) {
        const s=scenario([h],[0x3c00],0n,0n,1n,{format,xbase:0xa0000002,sx:1,ybase:0xa0000008,sy:1});
        const t=trace([s]);verify(t,0);widened++;
        const {forged,context}=forge(t,0,'x_op',widened%32);
        assert.ok(!check.accepts(forged,context),`widen ${format}:${h}`);rejected++;
    }
}
// A wrapping element has no 29-bit word address: x + 2*s*(n-1) + 2 > 2^32 is unsatisfiable.
{
    const wrap=scenario([0x3c00,0x3c00,0x3c00],[0x3c00,0x3c00,0x3c00],0n,0n,1n,
        {format:2,xbase:0xfffffff0,sx:4,ybase:0xa0000000,sy:1});
    const t=trace([wrap]);
    for(const i of [0,1,2,3])verify(t,i);
    const row=t.row(4),context={rowIndex:4,neighbors:{'-1':t.row(3)}};
    assert.ok(!check.accepts(row,context),'wrapped element row');rejected++;
}
// Unselected lanes are only memory-bound: a local change is accepted, so the bus must reject it.
{
    const i=mixed.offsets[mixedCalls.indexOf(bf16)];
    const {forged,context}=forge(mixed,i,'x_word',63);
    assert.ok(check.accepts(forged,context),'ignored lane is a bus obligation');
}
console.log(`FP16/BF16 operand rows: ${widened} widened encodings checked.`);
console.log(`floatdot PIL: ${valid} transition rows, ${rejected} malformed/forged rows rejected; no bus/VM proof.`);
