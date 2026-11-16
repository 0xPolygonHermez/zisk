// Test witness construction. Expected arithmetic outputs come from Berkeley SoftFloat.
const {bits}=require('./pil_test.cjs');
const pack=xs=>xs.reduce((n,b,i)=>n+(BigInt(b)<<BigInt(i)),0n);
function decode(value) {
    value=BigInt(value);const raw=bits(value,32),e=Number((value>>23n)&255n),f=value&0x7fffffn;
    const chain=(xs,fn)=>{const r=[1n];for(const x of xs)r.push(fn(r.at(-1),x));return r;};
    const cls=[e===0&&f===0n,e===0&&f!==0n,e>0&&e<255,e===255&&f===0n,
        e===255&&f>=0x400000n,e===255&&f>0n&&f<0x400000n].map(BigInt);
    return {sig:bits(f+(e?0x800000n:0n),24),sign:raw[31],exp:e,cls,
        ez:chain(raw.slice(23,31),(a,b)=>a*(1n-b)),eo:chain(raw.slice(23,31),(a,b)=>a*b),
        fz:chain(raw.slice(0,23),(a,b)=>a*(1n-b)),finite_sig:bits(e===255?0n:f+(e?0x800000n:0n),24),
        eff_exp:e===255?1:Math.max(e,1)};
}
function shift(value,amount) {
    value=BigInt(value);const stages=[],sticky=[];
    for(let j=0;j<6;j++) {
        const d=1<<j,old=bits(value,48);let any=0n;
        sticky.push(Array.from({length:33},(_,k)=>{if(k>d)return 0n;any|=old[k];return any;}));
        if(amount&d)value=(value>>BigInt(d))|BigInt((value&((1n<<BigInt(d))-1n))!==0n);
        stages.push(bits(value,48));
    }
    return {stages,sticky,value};
}
function normalize(value) {
    value=BigInt(value);const raw=bits(value,48),h=value===0n?0:value.toString(2).length-1;
    const prefix=Array(49).fill(0n),leading=Array(48).fill(0n);prefix[48]=1n;
    for(let i=47;i>=0;i--){leading[i]=raw[i]*prefix[i+1];prefix[i]=prefix[i+1]*(1n-raw[i]);}
    const norm=bits(value<<BigInt(47-h),48),tail=[];let any=0n;
    for(let i=0;i<22;i++){any|=norm[i];tail.push(any);}
    return {raw,norm,high_index:bits(h,6),prefix,leading,tail,sig:[any,...norm.slice(22)],h};
}
function roundPack(sig,exponent,sign) {
    const eb=exponent+512,distance=Math.max(0,-126-exponent),n=Math.min(63,distance);
    const shifted=shift(sig,n),tail=shifted.value&7n,hi=shifted.value>>3n;
    const rounded=hi+BigInt(tail>4n||(tail===4n&&(hi&1n)!==0n));
    const carry=Number(rounded>>24n),overflow=sig!==0n&&eb+carry>=640;
    const tiny=distance>0&&(exponent < -127||sig < 0x7fffffcn);
    const inexact=BigInt(tail!==0n),top_ones=[1n],sb=bits(sig,27);
    for(let k=2;k<27;k++)top_ones.push(top_ones.at(-1)*sb[k]);
    const mantissa=rounded>>BigInt(carry);
    const outExp=overflow?255:(mantissa>=0x800000n?Math.max(exponent,-126)+127+carry:0);
    const result=(BigInt(sign)<<31n)|(BigInt(outExp)<<23n)|(overflow?0n:mantissa&0x7fffffn);
    const flags=overflow?5:Number(inexact)|(tiny&&inexact?2:0);
    return {exp_lower:bits(eb-214,10),exp_upper:bits(767-eb,10),under_diff:bits(eb-386+1024,11),
        tiny_diff:bits(eb-385+1024,11),shift_amount:bits(distance,8),shift_bits:bits(n,6),
        stages:shifted.stages,sticky:shifted.sticky,rounded:bits(rounded,25),inexact,top_ones,
        overflow_diff:bits(eb+carry-640+1024,11),result:bits(result,32),flags:bits(flags,5)};
}
function arithmetic(a,b,is_mul,result,flags) {
    a=BigInt(a);b=BigInt(b);const da=decode(a),db=decode(b),w={a:bits(a,32),b:bits(b,32),is_mul,
        result:bits(result,32),flags:bits(flags,5)};
    for(const [name,d] of [['a',da],['b',db]]) for(const [key,value] of Object.entries(d))w[`fp_${name}_${key}`]=value;
    const absA=da.exp===255?0n:a&0x7fffffffn,absB=db.exp===255?0n:b&0x7fffffffn;
    const ge=absA>=absB,large=ge?da:db,small=ge?db:da,delta=large.eff_exp-small.eff_exp;
    const signXor=da.sign^db.sign,align=shift(pack(small.finite_sig)<<3n,Math.min(delta,63));
    const sum=signXor?(pack(large.finite_sig)<<3n)-align.value:(pack(large.finite_sig)<<3n)+align.value;
    const product=pack(da.finite_sig)*pack(db.finite_sig),norm=normalize(is_mul?product:sum);
    w.fp_compare=bits(absA-absB+(1n<<31n),32);w.fp_delta=bits(delta,8);
    w.fp_align_bits=bits(Math.min(delta,63),6);w.fp_align_stages=align.stages;w.fp_align_sticky=align.sticky;
    w.fp_sign_xor=signXor;w.fp_product=bits(product,48);w.fp_sum=bits(sum,28);
    for(const key of ['raw','norm','high_index','prefix','leading','tail','sig'])w[`fp_norm_${key}`]=norm[key];
    const exponent=is_mul?da.eff_exp+db.eff_exp+norm.h-300:large.eff_exp+norm.h-153;
    w.fp_exponent=bits(exponent+512,10);
    const sign=is_mul?signXor:(signXor&&sum===0n?0n:large.sign);w.fp_sign=sign;
    const rounded=roundPack(pack(norm.sig),exponent,sign);
    for(const [key,value] of Object.entries(rounded))w[`fp_round_${key}`]=value;
    const anyNan=da.cls[4]+da.cls[5]+db.cls[4]+db.cls[5] > 0n;
    const invalid=da.cls[5]+db.cls[5]>0n||(is_mul?
        da.cls[3]*db.cls[0]+db.cls[3]*da.cls[0]>0n:da.cls[3]*db.cls[3]*signXor>0n);
    w.fp_invalid=BigInt(invalid);w.fp_nan=BigInt(anyNan||invalid);
    w.fp_inf=BigInt(!w.fp_nan&&(da.cls[3]+db.cls[3]>0n));
    return w;
}
module.exports={arithmetic,decode,shift,normalize,roundPack,pack};
