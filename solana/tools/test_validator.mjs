// Real RPC/client coverage on an isolated, disposable Agave 2.2.21 ledger.
import assert from 'node:assert/strict';
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {readFile,writeFile,mkdtemp,mkdir} from 'node:fs/promises';
import {openSync,closeSync} from 'node:fs';
import {createServer} from 'node:net';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {Connection,Keypair,PublicKey,Transaction,TransactionInstruction,ComputeBudgetProgram,SystemProgram} from '@solana/web3.js';
import {decodeFixture,ix,checkedWire} from '../client/protocol.mjs';

const run=promisify(execFile);
const root=fileURLToPath(new URL('../',import.meta.url));
process.chdir(root);
const version=(await run('solana-test-validator',['--version'])).stdout.trim();
assert.match(version,/ 2\.2\.21 /,'use the pinned Agave 2.2.21 validator');
const features=JSON.parse(await readFile('benchmarks/validator-features.json','utf8'));
const guest='ed39a4bf37fadea5ad27fd5d35d04411ccc1aeb852a7a178634d41e7f253bb0b';
const claim=decodeFixture(JSON.parse(await readFile('fixtures/blake3.json','utf8')),guest);
const key=n=>new PublicKey(new Uint8Array(32).fill(n));
const verifier=key(41), consumer=key(42), laterConsumer=key(44);
const loader=new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111');
const programdata=(await PublicKey.findProgramAddress([verifier.toBytes()],loader))[0];
const payer=await Keypair.generate();
await mkdir('.cache',{recursive:true});
const directory=await mkdtemp('.cache/validator-');
const wallet=`${directory}/wallet.json`;
await writeFile(wallet,JSON.stringify(Array.from(payer.secretKey)),{mode:0o600});
async function freePorts() {
    for(let attempt=0;attempt<20;attempt++) {
        const port=20000+Math.floor(Math.random()*30000), servers=[];
        try {
            for(let offset=0;offset<4;offset++) {
                const server=createServer();servers.push(server);
                await new Promise((resolve,reject)=>server.once('error',reject).listen(port+offset,'127.0.0.1',resolve));
            }
            return port;
        } catch(error) {if(error.code!=='EADDRINUSE') throw error;}
        finally {await Promise.all(servers.map(server=>new Promise(resolve=>server.close(resolve))));}
    }
    throw new Error('No free local port range');
}
const port=await freePorts(), faucetPort=port+2, gossipPort=port+3;
const rpc=`http://127.0.0.1:${port}`;
const log=openSync(`${directory}/validator.log`,'w');
const validator=spawn('solana-test-validator',[
    '--quiet','--bind-address','127.0.0.1','--rpc-port',String(port),
    '--faucet-port',String(faucetPort),'--gossip-port',String(gossipPort),
    '--ledger',`${directory}/ledger`,'--mint',payer.publicKey.toBase58(),
    '--upgradeable-program',verifier.toBase58(),'target/deploy/zisk_solana_program.so',payer.publicKey.toBase58(),
    '--upgradeable-program',consumer.toBase58(),'target/deploy/zisk_consumer_example.so','none',
    '--upgradeable-program',laterConsumer.toBase58(),'target/deploy/zisk_consumer_example.so','none',
    ...features.inactive.flatMap(f=>['--deactivate-feature',f.id]),
],{stdio:['ignore',log,log],detached:true});
closeSync(log);
const stop=()=>{try {process.kill(-validator.pid,'SIGTERM');} catch(error) {if(error.code!=='ESRCH') throw error;}};
process.once('SIGINT',()=>{stop();process.exit(130);});
process.once('SIGTERM',()=>{stop();process.exit(143);});
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const connection=new Connection(rpc,'confirmed');
const measurements={};
let largest=0;
async function transaction(signature) {
    for(let i=0;i<50;i++) {
        const tx=await connection.getTransaction(signature,{commitment:'confirmed',maxSupportedTransactionVersion:0});
        if(tx) return tx;
        await pause(200);
    }
    throw new Error(`RPC never returned transaction ${signature}`);
}
async function send(label,instructions,expectedError=null,limit=1_400_000) {
    const lifetime=await connection.getLatestBlockhash();
    const tx=new Transaction({...lifetime,feePayer:payer.publicKey});
    tx.add(ComputeBudgetProgram.setComputeUnitLimit({units:limit}),...instructions);
    const wire=await checkedWire(tx,[payer]);
    largest=Math.max(largest,wire.length);
    const signature=await connection.sendRawTransaction(wire,{skipPreflight:expectedError!==null});
    const result=await connection.confirmTransaction({...lifetime,signature},'confirmed');
    assert.deepEqual(result.value.err,expectedError,label);
    const observed=await transaction(signature);
    measurements[label]=Number(observed.meta.computeUnitsConsumed);
    return observed;
}
async function consume(program,buffer,mode,outputs=claim.outputs) {
    const digest=createHash('sha256').update('ZisK:blake3-leaf:v1').update(claim.guest).update(outputs).digest();
    const receipt=(await PublicKey.findProgramAddress([Buffer.from('consumed'),digest],program))[0];
    return new TransactionInstruction({programId:program,keys:[
        {pubkey:payer.publicKey,isSigner:true,isWritable:true},
        {pubkey:receipt,isSigner:false,isWritable:true},
        {pubkey:buffer,isSigner:false,isWritable:false},
        {pubkey:verifier,isSigner:false,isWritable:false},
        {pubkey:programdata,isSigner:false,isWritable:false},
        {pubkey:SystemProgram.programId,isSigner:false,isWritable:false},
    ],data:Buffer.concat([Buffer.from([mode]),outputs])});
}
try {
    let ready=false;
    for(let i=0;i<150;i++) {
        if(validator.exitCode!==null) throw new Error(`validator exited; see ${directory}/validator.log`);
        try {if(Number(await connection.getSlot())>=3) {ready=true;break;}} catch {}
        await pause(200);
    }
    assert.ok(ready,`validator failed to start; see ${directory}/validator.log`);
    const localFeatures=JSON.parse((await run('solana',['feature','status','--url',rpc,'--display-all','--output','json'])).stdout);
    for(const f of features.inactive) assert.equal(localFeatures.features.find(x=>x.id===f.id)?.status,'inactive',f.id);
    const base=['client/submit.mjs','--rpc',rpc,'--program',verifier.toBase58(),'--wallet',wallet];
    const upload=await run(process.execPath,[...base,'--proof','fixtures/blake3.json','--guest',guest],{timeout:180_000});
    await writeFile(`${directory}/upload.log`,upload.stdout);
    const buffer=new PublicKey(upload.stdout.match(/Buffer: (\w+)/)[1]);
    const sent=[...upload.stdout.matchAll(/Sent (\w+) \((\d+) bytes\)/g)];
    assert.equal(sent.length,11);
    largest=Math.max(largest,...sent.map(m=>Number(m[2])));
    measurements.record_valid=Number((await transaction(sent.at(-1)[1])).meta.computeUnitsConsumed);
    assert.equal((await connection.getAccountInfo(buffer)).data[44],2);
    await send('verify_valid',[ix(verifier,buffer,payer.publicKey,3,Buffer.concat([claim.guest,claim.outputs]))]);
    const wrongGuest=Buffer.from(claim.guest);wrongGuest[0]^=1;
    await send('verify_wrong_guest',[ix(verifier,buffer,payer.publicKey,3,Buffer.concat([wrongGuest,claim.outputs]))],{InstructionError:[1n,{Custom:3n}]});
    const saved=await consume(laterConsumer,buffer,1);
    await send('consumer_upgradeable_rejected',[saved],{InstructionError:[1n,'InvalidAccountData']});
    await run('solana',['program','set-upgrade-authority',verifier.toBase58(),'--final','--keypair',wallet,'--url',rpc],{timeout:60_000});
    const changedOutputs=Buffer.from(claim.outputs);changedOutputs[0]^=1;
    await send('consumer_wrong_outputs',[await consume(laterConsumer,buffer,1,changedOutputs)],{InstructionError:[1n,'InvalidAccountData']},50_000);
    await send('consumer_cpi',[await consume(consumer,buffer,0)]);
    await send('consumer_recorded',[saved],null,50_000);
    const receipt=saved.keys[1].pubkey;
    const good=(await connection.getAccountInfo(receipt)).data;
    await send('consumer_replay',[saved],{InstructionError:[1n,'AccountAlreadyInitialized']},50_000);
    assert.deepEqual((await connection.getAccountInfo(receipt)).data,good);
    const close=await run(process.execPath,[...base,'--buffer',buffer.toBase58(),'--close'],{timeout:60_000});
    await writeFile(`${directory}/close.log`,close.stdout);
    assert.equal(await connection.getAccountInfo(buffer),null);
    assert.deepEqual((await connection.getAccountInfo(receipt)).data,good);
    const hashes={};
    for(const path of ['target/deploy/zisk_solana_program.so','target/deploy/zisk_consumer_example.so','benchmarks/validator-features.json']) {
        hashes[path]=createHash('sha256').update(await readFile(path)).digest('hex');
    }
    const report={validator:version,feature_snapshot:'benchmarks/validator-features.json',note:'Local validator with recorded Agave 2.2.21 feature gates; not full live-cluster equivalence.',measurements,largest_transaction_bytes:largest,hashes};
    await writeFile('.cache/validator.json',JSON.stringify(report,null,2)+'\n');
    console.log(JSON.stringify(report,null,2));
} finally {
    stop();
}
