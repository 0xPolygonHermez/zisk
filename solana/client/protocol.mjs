import {TransactionInstruction} from '@solana/web3.js';
export const HEADER=77, SIZE=6829, CHUNK=900;
const ROOT="0xedf3a1f810d95764e99c0db7affc52918f3bbc7444f222891d1675897a63cb2d".replace(/^0x/, "").toLowerCase();
export function decodeFixture(value,expectedGuest) {
    const guestHex=typeof expectedGuest==='string'?expectedGuest.replace(/^0x/,''):'';
    if(!/^[0-9a-f]{64}$/i.test(guestHex)) throw new Error('Expected guest must be 32 bytes of hex (--guest)');
    const guest=Buffer.from(guestHex,'hex');
    const bytes=(key,size)=>{
        const hex=typeof value[key]==='string'?value[key].replace(/^0x/,''):'';
        if(hex.length!==size*2 || !/^[0-9a-f]+$/i.test(hex)) throw new Error(`Invalid ${key}: expected ${size} bytes of hex`);
        return Buffer.from(hex,'hex');
    };
    if(bytes('rootCVadcopFinal',32).toString('hex')!==ROOT) throw new Error('Proof uses a different recursion root/profile');
    if(!bytes('programVK',32).equals(guest)) throw new Error('Proof guest does not match the expected guest');
    return {guest,outputs:bytes('publicValues',512),proof:bytes('proofBytes',6752)};
}
export function ix(program,buffer,authority,tag,payload=Buffer.alloc(0)) {
    const valid=tag===0?payload.length===4&&payload.readUInt32LE()===6752:
        tag===1?payload.length>=5&&payload.length<=904&&payload.readUInt32LE()+payload.length-4<=6752:
        (tag===3||tag===5)?payload.length===544:(tag===2||tag===4)&&payload.length===0;
    if(!valid) throw new Error('Invalid verifier instruction');
    const keys=[{pubkey:buffer,isSigner:tag===0,isWritable:tag!==3}];
    if(tag!==3&&tag!==5) keys.push({pubkey:authority,isSigner:true,isWritable:tag===4});
    return new TransactionInstruction({programId:program,keys,data:Buffer.concat([Buffer.from([tag]),payload])});
}
export async function checkedWire(tx,signers) {
    await tx.sign(...signers);
    const wire=await tx.serialize();
    if(wire.length>1232) throw new Error(`Transaction exceeds packet limit: ${wire.length}`);
    return wire;
}
export function readBuffer(account,program,authority,proof) {
    if(!account || !account.owner.equals(program)) throw new Error('Buffer missing or owned by another program');
    const data=Buffer.from(account.data);
    if(data.length!==SIZE || data.subarray(0,8).toString()!=='ZKPF0002' || !data.subarray(8,40).equals(Buffer.from(authority.toBytes()))) throw new Error('Buffer version, size or authority mismatch');
    const written=data.readUInt32LE(40), sealed=data[44]!==0;
    if(written>6752 || data[44]>2 || (sealed&&written!==6752) || (data[44]!==2&&data.subarray(45,77).some(b=>b!==0))) throw new Error('Invalid buffer state');
    if(proof && !data.subarray(HEADER,HEADER+written).equals(proof.subarray(0,written))) throw new Error('Buffer contains a different proof');
    return {written,sealed};
}
