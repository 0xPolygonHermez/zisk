import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {Keypair,PublicKey,SystemProgram,Transaction,ComputeBudgetProgram} from '@solana/web3.js';
import {decodeFixture,ix,checkedWire,readBuffer,HEADER,SIZE,CHUNK} from './protocol.mjs';
const fixture = JSON.parse(await readFile(new URL('../fixtures/blake3.json',import.meta.url),'utf8'));
const payer = await Keypair.fromSeed(new Uint8Array(32).fill(1));
const buffer = await Keypair.fromSeed(new Uint8Array(32).fill(2));
const program = new PublicKey(new Uint8Array(32).fill(41));
const expectedGuest='ed39a4bf37fadea5ad27fd5d35d04411ccc1aeb852a7a178634d41e7f253bb0b';
const claim = decodeFixture(fixture,expectedGuest);
const u32 = n => { const b=Buffer.alloc(4);b.writeUInt32LE(n);return b; };

test('the caller must select the expected guest independently of the proof file',()=>{
    const guest='ed39a4bf37fadea5ad27fd5d35d04411ccc1aeb852a7a178634d41e7f253bb0b';
    assert.throws(()=>decodeFixture(fixture),/guest/i);
    for(const bad of ['', '00', 'z'.repeat(64), '00'.repeat(32)]) {
        assert.throws(()=>decodeFixture(fixture,bad),/guest/i);
    }
    const decoded=decodeFixture(fixture,guest);
    assert.equal(decoded.guest.toString('hex'),guest);
    assert.throws(()=>decodeFixture({...fixture,programVK:'0x'+'00'.repeat(32)},guest),/guest/i);
});

test('full signed upload and verification transactions fit the packet limit',async()=>{
    const batches = [[SystemProgram.createAccount({fromPubkey:payer.publicKey,newAccountPubkey:buffer.publicKey,lamports:48198000n,space:SIZE,programId:program}),ix(program,buffer.publicKey,payer.publicKey,0,u32(6752))]];
    for(let offset=0;offset<claim.proof.length;offset+=CHUNK) batches.push([ix(program,buffer.publicKey,payer.publicKey,1,Buffer.concat([u32(offset),claim.proof.subarray(offset,offset+CHUNK)]))]);
    batches.push([ix(program,buffer.publicKey,payer.publicKey,2)]);
    batches.push([ix(program,buffer.publicKey,payer.publicKey,5,Buffer.concat([claim.guest,claim.outputs]))]);
    const sizes=[];
    for(let i=0;i<batches.length;i++) {
        const tx=new Transaction({feePayer:payer.publicKey,blockhash:PublicKey.default.toBase58(),lastValidBlockHeight:100n});
        tx.add(ComputeBudgetProgram.setComputeUnitLimit({units:1400000}),...batches[i]);
        const wire=await checkedWire(tx,i===0?[payer,buffer]:[payer]);sizes.push(wire.length);
        assert.equal(Transaction.from(wire).instructions.length,i===0?3:2);
    }
    assert.equal(batches.length,11);assert.ok(Math.max(...sizes)<=1232);
});

test('strict fixture and resume binding reject malformed or different data',()=>{
    for(const mutation of [{proofBytes:'0x00'},{programVK:'0xz'},{publicValues:fixture.publicValues+'00'},{rootCVadcopFinal:'0x'+'00'.repeat(32)}]) assert.throws(()=>decodeFixture({...fixture,...mutation},expectedGuest));
    const data=Buffer.alloc(SIZE);data.write('ZKPF0002');Buffer.from(payer.publicKey.toBytes()).copy(data,8);data.writeUInt32LE(900,40);claim.proof.copy(data,HEADER,0,900);
    assert.deepEqual(readBuffer({owner:program,data},program,payer.publicKey,claim.proof),{written:900,sealed:false});
    const changed=Buffer.from(claim.proof);changed[0]^=1;
    assert.throws(()=>readBuffer({owner:program,data},program,payer.publicKey,changed));
    assert.throws(()=>readBuffer({owner:program,data},program,buffer.publicKey,claim.proof));
    data[44]=1;assert.throws(()=>readBuffer({owner:program,data},program,payer.publicKey,claim.proof));
});
