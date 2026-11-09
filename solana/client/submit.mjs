import {readFile} from 'node:fs/promises';
import {parseArgs} from 'node:util';
import {Connection,Keypair,PublicKey,SystemProgram,Transaction,ComputeBudgetProgram} from '@solana/web3.js';
import {decodeFixture,ix,checkedWire,readBuffer,SIZE,CHUNK} from './protocol.mjs';

const {values:args}=parseArgs({options:{rpc:{type:'string'},program:{type:'string'},wallet:{type:'string'},proof:{type:'string'},guest:{type:'string'},buffer:{type:'string'},close:{type:'boolean'},help:{type:'boolean'}}});
if(args.help) {
    console.log('node client/submit.mjs --rpc URL --program ID --wallet keypair.json --proof export.json --guest EXPECTED_GUEST_HEX [--buffer ID]\nClose and reclaim rent: use --buffer ID --close instead of --proof and --guest.');
    process.exit(0);
}
if(!args.rpc||!args.program||!args.wallet||(!args.close&&(!args.proof||!args.guest))||(args.close&&(!args.buffer||args.proof||args.guest))) throw new Error('Specify --rpc, --program, --wallet and either --proof --guest or --buffer --close; see --help');
const claim=args.close?null:decodeFixture(JSON.parse(await readFile(args.proof,'utf8')),args.guest);
const secret=JSON.parse(await readFile(args.wallet,'utf8'));
if(!Array.isArray(secret)||secret.length!==64||secret.some(x=>!Number.isInteger(x)||x<0||x>255)) throw new Error('Wallet must contain a Solana CLI 64-byte keypair array');
const payer=await Keypair.fromSecretKey(Uint8Array.from(secret));
const connection=new Connection(args.rpc,'confirmed');
const program=new PublicKey(args.program);
if(!(await connection.getAccountInfo(program))?.executable) throw new Error('Verifier program is not executable');
const u32=n=>{const b=Buffer.alloc(4);b.writeUInt32LE(n);return b;};
async function send(instructions,signers=[payer]) {
    const lifetime=await connection.getLatestBlockhash();
    const tx=new Transaction({...lifetime,feePayer:payer.publicKey});
    tx.add(ComputeBudgetProgram.setComputeUnitLimit({units:1400000}),...instructions);
    const wire=await checkedWire(tx,signers);
    const signature=await connection.sendRawTransaction(wire,{skipPreflight:false,preflightCommitment:'confirmed'});
    console.log(`Sent ${signature} (${wire.length} bytes)`);
    const result=await connection.confirmTransaction({...lifetime,signature},'confirmed');
    if(result.value.err) throw new Error('Transaction failed',{cause:result.value.err});
}
let buffer=args.buffer?new PublicKey(args.buffer):null;
if(!buffer) {
    const key=await Keypair.generate();buffer=key.publicKey;
    // Initialization is atomic with account creation; only the authority signs later writes/close.
    console.log(`Buffer: ${buffer.toBase58()} (retain this address to resume or close)`);
    const rent=await connection.getMinimumBalanceForRentExemption(SIZE);
    console.log(`Refundable rent reserve: ${rent} lamports`);
    await send([SystemProgram.createAccount({fromPubkey:payer.publicKey,newAccountPubkey:buffer,lamports:rent,space:SIZE,programId:program}),ix(program,buffer,payer.publicKey,0,u32(6752))],[payer,key]);
}
let state=readBuffer(await connection.getAccountInfo(buffer),program,payer.publicKey,claim?.proof);
if(args.close) {
    await send([ix(program,buffer,payer.publicKey,4)]);
    console.log('Buffer closed; rent returned to authority.');
} else {
    if(!state.sealed) {
        for(let offset=state.written;offset<claim.proof.length;offset+=CHUNK) await send([ix(program,buffer,payer.publicKey,1,Buffer.concat([u32(offset),claim.proof.subarray(offset,offset+CHUNK)]))]);
        await send([ix(program,buffer,payer.publicKey,2)]);
    }
    await send([ix(program,buffer,payer.publicKey,5,Buffer.concat([claim.guest,claim.outputs]))]);
    console.log(`Verified statement recorded by ${program.toBase58()}; buffer ${buffer.toBase58()} is available for later consumption or close.`);
}
