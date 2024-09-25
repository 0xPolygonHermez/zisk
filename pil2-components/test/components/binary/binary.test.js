const assert = require('assert');
const F1Field = require("ffjavascript").F1Field;
const util = require('util');

const data = require('./binary.data.js');

const CDIR = __dirname + "/../../../components";
const Binary = require(CDIR + "/binary/binary.js");

function toChunks(value, count, chunkBits) {
    let res = [];
    let _value = BigInt(value);
    const _chunkBits = BigInt(chunkBits);
    const mask = 2n**_chunkBits - 1n;
    for (let index = 0; index < count; ++index) {
        res.push(_value & mask);
        _value = _value >> _chunkBits;
    }
    return res;
}
describe("verify component", async function () {
    this.timeout(10000000);
    const fr = new F1Field("0xFFFFFFFF00000001");
    it("verify method", async () => {
        const binary = new Binary({fr});
        let index = 0n;
        for (const input of data.ok) {
            const _input = [index++, BigInt(input.operation), ...toChunks('0x'+input.a, 8, 32), ...toChunks('0x'+input.b, 8, 32), 
                      ...toChunks('0x'+input.c, 8, 32), 0n, BigInt(input.carry)];
            binary.verify(_input);
        }
    });
    it("calculateFreeInput method", async () => {
        const binary = new Binary({fr});
        let index = 0n;
        for (const input of data.ok) {
            const _input = [index++, BigInt(input.operation), ...toChunks('0x'+input.a, 8, 32), ...toChunks('0x'+input.b, 8, 32), 
                      ...toChunks('0x'+input.c, 8, 32), 0n, BigInt(input.carry)];            
            assert.equal(binary.calculateFreeInput(_input), BigInt('0x'+input.c));
        }
    });
    it("execute method", async () => {
        const binary = new Binary({fr});
        let id = 0n;
        
        const ONE_HSB = (1n << BigInt(binary.bits - 8));
        for (const input of data.ok) {
            let pols = { freeInA: new Array(binary.bpc), freeInB: new Array(binary.bpc), freeInC: new Array(binary.bpc), carry: new Array(binary.bpc+1)};
            for (const name in pols) {
                pols[name] = pols[name].fill(0n).map(x => new Array(binary.clocks));
            }
            const operation = BigInt(input.operation);
            binary.execute(pols, 0, id++, operation, BigInt('0x'+input.a), BigInt('0x'+input.b), 0n);
            let a = 0n;
            let b = 0n;
            let c = 0n;
            for (let clock = binary.clocks - 1; clock >= 0; --clock) {
                for (let index = binary.bpc - 1; index >= 0; --index) {
                    a = a * 256n + pols.freeInA[index][clock];
                    b = b * 256n + pols.freeInB[index][clock];
                    c = c * 256n + pols.freeInC[index][clock];
                }
            }
            // console.log([operation, Binary.EQ, Binary.LT, Binary.SLT, Binary.LT4, c]);
            if (operation === Binary.EQ || operation === Binary.LT || operation === Binary.SLT || operation === Binary.LT4) {
                if (c === ONE_HSB) c = 1n;
            }
            assert.equal(a, BigInt('0x'+input.a), `A not match on ${id-1n}`);
            assert.equal(b, BigInt('0x'+input.b), `B not match on ${id-1n}`);
            assert.equal(c, BigInt('0x'+input.c), `C not match on ${id-1n}`);
            assert.equal(pols.carry[0][0], 0n, `CARRY_IN not match on ${id-1n}`);
            assert.equal(pols.carry[binary.bpc][binary.clocks-1], BigInt(input.carry),`CARRY_OUT not match on ${id-1n}`);
        }
    });
});
/*
describe("test plookup operations", async function () {

    this.timeout(10000000);
    const Fr = new F1Field("0xFFFFFFFF00000001");
    let pil;

    const N = 2**22;
    let constPols, cmPols;
    async function preparePilFromString() {
        // pil = await compile(Fr, "pil/binary.pil", null, {defines: { N }});
        pil = await compile(Fr, ['include "pil/binary.pil";',
            'namespace Main(2**22);',
            'pol commit A[8],B[8],C[8],binOpcode,carry,bin,range;',
            'bin {',
            '   binOpcode,',
            '   A[0], A[1], A[2], A[3], A[4], A[5], A[6], A[7],',
            '   B[0], B[1], B[2], B[3], B[4], B[5], B[6], B[7],',
            '   C[0], C[1], C[2], C[3], C[4], C[5], C[6], C[7],',
            '   carry',
            '} is',
            'Binary.resultBinOp {',
            '   Binary.lOpcode,',
            '   Binary.a[0], Binary.a[1], Binary.a[2], Binary.a[3], Binary.a[4], Binary.a[5], Binary.a[6], Binary.a[7],',
            '   Binary.b[0], Binary.b[1], Binary.b[2], Binary.b[3], Binary.b[4], Binary.b[5], Binary.b[6], Binary.b[7],',
            '   Binary.c[0], Binary.c[1], Binary.c[2], Binary.c[3], Binary.c[4], Binary.c[5], Binary.c[6], Binary.c[7],',
            '   Binary.lCout',
            '};',
            'range {',
            '   A[0], A[1], A[2], A[3], A[4], A[5], A[6], A[7],',
            '   %GL_L, %GL_H, %GL_L, %GL_H, %GL_L, %GL_H, %GL_L, %GL_H,',
            '   8,1',
            '} is',
            'Binary.resultValidRange {',
            '   Binary.a[0], Binary.a[1], Binary.a[2], Binary.a[3], Binary.a[4], Binary.a[5], Binary.a[6], Binary.a[7],',
            '   Binary.b[0], Binary.b[1], Binary.b[2], Binary.b[3], Binary.b[4], Binary.b[5], Binary.b[6], Binary.b[7],',
            '   Binary.lOpcode, Binary.lCout',
            '};'].join("\n"), null, {compileFromString: true, defines: { N }});
        await buildConstants();
    }
    async function preparePil() {
        pil = await compile(Fr, "pil/binary.pil", null, {defines: { N }});
        await buildConstants();
    }
    async function buildConstants() {
        constPols = newConstantPolsArray(pil);
        await smGlobal.buildConstants(constPols.Global);
        await smBinary.buildConstants(constPols.Binary);

        for (let i=0; i<constPols.$$array.length; i++) {
            for (let j=0; j<N; j++) {
                if (typeof constPols.$$array[i][j] !== "bigint") {
                    throw new Error(`Polynomial not fited ${constPols.$$defArray[i].name} at ${j}` )
                }
            }
        }
    }

    function smMainExecute (cmPols, input) {
        // fill main inputs
        const MASK32 = (2n ** 32n - 1n);
        for (let index = 0; index < input.length; ++index) {
            for (let k = 0; k < 8; ++k) {
                const bits = BigInt(32 * k);
                cmPols.Main.A[k][index] = (BigInt('0x'+input[index].a) >> bits) & MASK32;
                cmPols.Main.B[k][index] = (BigInt('0x'+input[index].b) >> bits) & MASK32;
                cmPols.Main.C[k][index] = (BigInt('0x'+input[index].c) >> bits) & MASK32;
            }
            cmPols.Main.bin[index] = input[index].type == 1 ? 1n : 0n;
            cmPols.Main.range[index] = input[index].type == 2 ? 1n : 0n;
            cmPols.Main.carry[index] = BigInt(input[index].carry ?? 0n)
            cmPols.Main.binOpcode[index] = BigInt(input[index].opcode)
        }

        const N = cmPols.Main.bin.length;
        for (let index = input.length; index < N; ++index) {
            for (let k = 0; k < 8; ++k) {
                cmPols.Main.A[k][index] = 0n;
                cmPols.Main.B[k][index] = 0n;
                cmPols.Main.C[k][index] = 0n;
            }
            cmPols.Main.bin[index] = 0n;
            cmPols.Main.range[index] = 0n;
            cmPols.Main.carry[index] = 0n;
            cmPols.Main.binOpcode[index] = 0n;
        }
    }

    function setup() {
        return new Binary();
    }
    it("It should verify the binary operations pil", async () => {
        // generateZkasmLt4Test(input.filter(x => x.opcode == 8));
        setup();
        EXIT_HERE;
        await preparePilFromString();
        cmPols = newCommitPolsArray(pil);

        await smBinary.execute(cmPols.Binary, input);
        smMainExecute(cmPols, input);

        for (let i=0; i<cmPols.$$array.length; i++) {
            for (let j=0; j<N; j++) {
                if (typeof cmPols.$$array[i][j] !== 'bigint') {
                    throw new Error(`Polynomial not fited ${cmPols.$$defArray[i].name} at ${j}` )
                }
            }
        }
        // Verify
        const res = await verifyPil(Fr, pil, cmPols, constPols ,{continueOnError: true});

        if (res.length != 0) {
            console.log("Pil does not pass");
            for (let i = 0; i < res.length; i++) {
                console.log(res[i]);
            }
            assert(0);
        }
    });

    function includes(res, value) {
        const index = res.indexOf(value);
        assert(index !== -1, "not found "+value);
        res.splice(index, 1);
    }

    function generateZkasmLt4Test(inputs) {

        for (const _input of inputs) {
            console.log(['    0x'+_input.a.padStart(64,'0').toUpperCase().match(/.{1,16}/g).join('_')+'n => A',
                         '    0x'+_input.b.padStart(64,'0').toUpperCase().match(/.{1,16}/g).join('_')+'n => B',
                         `    ${_input.carry} :LT4,${_input.carry?'JMPNC':'JMPC'}(OpBinLt4__CarryTestFail)`,
                         `    $ => A :LT4,${_input.carry?'JMPNC':'JMPC'}(OpBinLt4__CarryTestFail)`,
                         `    ${_input.carry} :ASSERT`, ''].join("\n"));
        }
    }

    it("It should fail tests", async () => {
        EXIT_HERE;
        await preparePilFromString();

        cmPols = newCommitPolsArray(pil);

        await smBinary.execute(cmPols.Binary, error_input);
        smMainExecute(cmPols, error_input);

        let res = await verifyPil(Fr, pil, cmPols, constPols, { continueOnError: true })
        res = res.map(x => x.split('/').slice(-1)[0]);
        for (let i = 0; i < res.length; i++) {
            console.log(res[i]);
        }
        expect(res.length).to.not.eq(0);

        const plookupLine1 = pil.plookupIdentities[0].line;
        const prefix1 = 'binary.pil:'+plookupLine1+':  plookup not found ';
        const plookupLine2 = pil.plookupIdentities[1].line;
        const prefix2 = 'binary.pil:'+plookupLine2+':  plookup not found ';
        const suffix = '';

        // P_LAST, P_OPCODE, Global.BYTE_2A, Global.BYTE, P_CIN, P_C, P_FLAGS
        includes(res, prefix1 + 'w=16 values: 1:0,1,255,255,0,1,0' + suffix);
        includes(res, prefix2 + 'w=15 values: 1:1,0,15,15,1,15,0' + suffix);
        includes(res, prefix2 + 'w=47 values: 1:1,2,0,0,1,0,3' + suffix);
        includes(res, prefix2 + 'w=63 values: 1:1,2,0,0,0,1,2' + suffix);
        includes(res, prefix2 + 'w=79 values: 1:1,3,128,0,0,0,3' + suffix);
        includes(res, prefix2 + 'w=95 values: 1:1,3,0,255,1,1,2' + suffix);
        includes(res, prefix2 + 'w=111 values: 1:1,4,0,0,0,0,3' + suffix);
        includes(res, prefix2 + 'w=127 values: 1:1,4,0,0,1,1,2' + suffix);
        includes(res, prefix2 + 'w=128 values: 1:0,4,255,255,0,1,0' + suffix);
        includes(res, prefix2 + 'w=159 values: 1:1,5,15,15,1,14,1' + suffix);
        includes(res, prefix2 + 'w=175 values: 1:1,6,176,180,0,164,0' + suffix);
        includes(res, prefix2 + 'w=191 values: 1:1,7,15,240,0,239,0' + suffix);

        // P_C 0 vs 1 ==> 1,8,255,0,0,0,10 OK ==> (P_CIN=0 && !(A < B) ==> P_C = 0)
        includes(res, prefix2 + 'w=255 values: 1:1,8,255,0,0,1,10' + suffix);

        // P_C 0 vs 1 ==> 1:1,8,0,0,0,0,10 OK ==> (P_CIN=0 && !(A < B) ==> P_C = 0)
        includes(res, prefix2 + 'w=367 values: 1:1,8,0,0,0,1,10' + suffix);

        const pPrefix1 = '(string):4:  permutation not found ';
        const pPrefix2 = '(string):4:  permutation failed. Remaining ';

        //  lOpcode, a[0..7], b[0..7], c[0..7], lCout

        // c[0]
        includes(res, pPrefix1 + 'w=2 values: 1:2,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1');
        includes(res, pPrefix2 +   '1 values: 1:2,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,1');

        // c[0]
        includes(res, pPrefix1 + 'w=3 values: 1:2,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0');
        includes(res, pPrefix2 +   '1 values: 1:2,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0');

        // c[0]
        includes(res, pPrefix1 + 'w=4 values: 1:3,0,0,0,0,0,0,0,2147483648,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0');
        includes(res, pPrefix2 +   '1 values: 1:3,0,0,0,0,0,0,0,2147483648,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,1');

        // c[0]
        includes(res, pPrefix1 + 'w=5 values: 1:3,0,0,0,0,0,0,0,0,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,1,0,0,0,0,0,0,0,0');
        includes(res, pPrefix2 +   '1 values: 1:3,0,0,0,0,0,0,0,0,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,0,0,0,0,0,0,0,0,0');

        // c[0]
        includes(res, pPrefix1 + 'w=6 values: 1:4,65280,0,0,0,0,0,0,0,65280,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1');
        includes(res, pPrefix2 +   '1 values: 1:4,65280,0,0,0,0,0,0,0,65280,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,1');

        // c[0]
        includes(res, pPrefix1 + 'w=7 values: 1:4,65280,0,0,0,0,0,0,0,255,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0');
        includes(res, pPrefix2 +   '1 values: 1:4,65280,0,0,0,0,0,0,0,255,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0');

        // c[0] 0xFF00 EQ 0xFFF00 = 0x100
        includes(res, pPrefix1 + 'w=8 values: 1:4,65280,0,0,0,0,0,0,0,1048320,0,0,0,0,0,0,0,256,0,0,0,0,0,0,0,0');
        includes(res, pPrefix2 +   '1 values: 1:4,65280,0,0,0,0,0,0,0,1048320,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0');

        // c[7] 0 vs 0x10000000 (268435456)
        includes(res, pPrefix1 + 'w=12 values: 1:2,255,0,0,0,0,0,0,0,65280,0,0,0,0,0,0,0,1,0,0,0,0,0,0,268435456,1');
        includes(res, pPrefix2 +    '1 values: 1:2,255,0,0,0,0,0,0,0,65280,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,1');

        // LT4 c[0] 0 vs 1
        includes(res, pPrefix1 + 'w=13 values: 1:8,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,1,4294967295,1,4294967295,1,4294967295,1,4294967295,0,0,0,0,0,0,0,0,1');
        includes(res, pPrefix2 +    '1 values: 1:8,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,1,4294967295,1,4294967295,1,4294967295,1,4294967295,0,0,0,0,0,0,0,0,0');

        // LT4 c[7] 0 vs 0x10000000 (268435456)
        includes(res, pPrefix1 + 'w=14 values: 1:8,0,4294967295,0,4294967295,0,4294967295,0,4294967295,1,4294967295,1,4294967295,1,4294967295,1,4294967295,1,0,0,0,0,0,0,268435456,1');
        includes(res, pPrefix2 +    '1 values: 1:8,0,4294967295,0,4294967295,0,4294967295,0,4294967295,1,4294967295,1,4294967295,1,4294967295,1,4294967295,1,0,0,0,0,0,0,0,1');

        // LT4 carry 1 vs 0
        includes(res, pPrefix1 + 'w=15 values: 1:8,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,1,4294967295,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0');
        includes(res, pPrefix2 +    '1 values: 1:8,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,1,4294967295,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0');

        // LT4 carry 1 vs 0
        includes(res, pPrefix1 + 'w=17 values: 1:8,0,0,0,0,0,0,4294967295,4293918719,0,0,0,0,0,0,4294967295,4294901759,1,0,0,0,0,0,0,0,0');
        includes(res, pPrefix2 +    '1 values: 1:8,0,0,0,0,0,0,4294967295,4293918719,0,0,0,0,0,0,4294967295,4294901759,0,0,0,0,0,0,0,0,0');

        // LT4 C[0] 1 vs 0 , carry 1 vs 0
        includes(res, pPrefix1 + 'w=18 values: 1:8,4294967295,4293918719,0,0,0,0,4294967295,4293918719,4294967295,4294901759,0,0,0,0,4294967295,4294901759,1,0,0,0,0,0,0,0,1');
        includes(res, pPrefix2 +    '1 values: 1:8,4294967295,4293918719,0,0,0,0,4294967295,4293918719,4294967295,4294901759,0,0,0,0,4294967295,4294901759,0,0,0,0,0,0,0,0,0');

        // LT4 carry 1 vs 0
        includes(res, pPrefix1 + 'w=19 values: 1:8,4294967295,4293918719,4294967295,4293918719,0,0,4294967295,4293918719,4294967295,4294901759,4294967295,4294901759,0,0,4294967295,4294901759,1,0,0,0,0,0,0,0,0');
        includes(res, pPrefix2 +    '1 values: 1:8,4294967295,4293918719,4294967295,4293918719,0,0,4294967295,4293918719,4294967295,4294901759,4294967295,4294901759,0,0,4294967295,4294901759,0,0,0,0,0,0,0,0,0');

        // LT4 carry 1 vs 0
        includes(res, pPrefix1 + 'w=20 values: 1:8,4294967295,4293918719,0,0,4294967295,4293918719,4294967295,4293918719,4294967295,4294901759,0,0,4294967295,4294901759,4294967295,4294901759,1,0,0,0,0,0,0,0,0');
        includes(res, pPrefix2 +    '1 values: 1:8,4294967295,4293918719,0,0,4294967295,4293918719,4294967295,4293918719,4294967295,4294901759,0,0,4294967295,4294901759,4294967295,4294901759,0,0,0,0,0,0,0,0,0');

        // LT4 C[0] 0 vs 1/0 , C[7] 0 vs 0/0x10000000, carry 0 vs 1/1 (NOTE: 268435456 = 0x10000000)
        includes(res, pPrefix1 + 'w=21 values: 1:8,0,0,4294967295,4293918719,4294967295,4293918719,4294967295,4293918719,0,0,4294967295,4294901759,4294967295,4294901759,4294967295,4294901759,1,0,0,0,0,0,0,0,1');
        includes(res, pPrefix1 + 'w=25 values: 1:8,0,0,4294967295,4293918719,4294967295,4293918719,4294967295,4293918719,0,0,4294967295,4294901759,4294967295,4294901759,4294967295,4294901759,0,0,0,0,0,0,0,268435456,1');
        includes(res, pPrefix2 +    '2 values: 1:8,0,0,4294967295,4293918719,4294967295,4293918719,4294967295,4293918719,0,0,4294967295,4294901759,4294967295,4294901759,4294967295,4294901759,0,0,0,0,0,0,0,0,0');

        // LT4 C[0] 1 vs 0 , carry 1 vs 0
        includes(res, pPrefix1 + 'w=22 values: 1:8,4294967295,4293918719,4294967295,4293918719,4294967295,4293918719,0,0,4294967295,4294901759,4294967295,4294901759,4294967295,4294901759,0,0,1,0,0,0,0,0,0,0,1');
        includes(res, pPrefix2 +    '1 values: 1:8,4294967295,4293918719,4294967295,4293918719,4294967295,4293918719,0,0,4294967295,4294901759,4294967295,4294901759,4294967295,4294901759,0,0,0,0,0,0,0,0,0,0,0');

        // LT4 C[0] 1 vs 0/0, C[7] 0 vs 0/0x10000000 (NOTE: 268435456 = 0x10000000)
        includes(res, pPrefix1 + 'w=16 values: 1:8,4294967295,4293918719,4294967295,4293918719,4294967295,4293918719,4294967295,4293918719,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294901759,0,0,0,0,0,0,0,0,1');
        includes(res, pPrefix1 + 'w=24 values: 1:8,4294967295,4293918719,4294967295,4293918719,4294967295,4293918719,4294967295,4293918719,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294901759,0,0,0,0,0,0,0,268435456,1');
        includes(res, pPrefix2 +    '2 values: 1:8,4294967295,4293918719,4294967295,4293918719,4294967295,4293918719,4294967295,4293918719,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294967295,4294901759,1,0,0,0,0,0,0,0,1');


// binary.pil:150:  plookup not found w=255 values: 1:1,8,255,0,0,1,10
// binary.pil:150:  plookup not found w=367 values: 1:1,8,0,0,0,1,10

        for (let i = 0; i < res.length; i++) {
            if (i === 0) {
                console.log('######################## NON EXPECTED ERRORS ######################');
            }
            console.log(res[i]);
        }
        expect(res.length).to.eq(0);

    })

});*/
