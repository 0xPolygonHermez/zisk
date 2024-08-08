const fs = require('fs');
const { F1Field } = require("ffjavascript");

const Fr = new F1Field("0xFFFFFFFF00000001");

const INT32_MIN = -(2n ** 32n) + 1n;
const INT32_MAX = (2n ** 32n) - 1n;

const P256_MAX = (2n ** 256n) - 1n;
const MASK_32 = (2n ** 32n) - 1n;

//  doAssert, mOp, mWR, isMem, isStack, useCTX, useAddrRel,
//  useJmpAddrRel, useElseAddrRel, jmp, jmpn, jmpz, call, returnJmp, repeat,
//  setA, setB, setC, setCTX, setD, setE,
//  setPC, setRCX, setRR, setSP, setSR
const ROM_FLAGS = [
    'assert', 'mOp', 'mWR', 'isMem', 'isStack', 'useCTX', 'useAddrRel',
    'useJmpAddrRel', 'useElseAddrRel', 'JMP', 'JMPN', 'JMPZ', 'call', 'returnJmp', 'repeat',
    'setA', 'setB', 'setC', 'setCTX', 'setD', 'setE',
    'setPC', 'setRCX', 'setRR', 'setSP', 'setSR']

class Rom2Pil {
    constructor(romFile) {
        this.values = [];
        this.labels = [];
        this.previousFileName = false;
        this.loadRom(romFile);
    }
    loadRom(romFile) {
        // Init rom from file
        this.romFile = romFile;
        const rawdata = fs.readFileSync(romFile);
        const json = JSON.parse(rawdata);
        const rom = json.program;
        const JMP_ADDRESS_MAX = BigInt(rom.length - 1);
        const FLAGS_DIGITS = Math.ceil(ROM_FLAGS.length / 4);

        let data = [];
        let previousFileName = false;
        for (let line = 0; line < rom.length; line++) {
            const l = rom[line];

            // MAIN
            // =======
            // lookup_assumes(ROM_ID, [...CONST, condConst, inA, inB, inC, inROTL_C, inD, inE, inSR, inFREE, inFREE0,
            //                         inCTX, inSP, inPC, inSTEP, inRR, inRCX, ind, indRR,
            //                         romFlags, offset, incStack, jmpAddr, elseAddr, zkPC]);
            // ROM
            // =======
            // lookup_proves(ROM_ID, [...CONST, COND_CONST, IN_A, IN_B, IN_C, IN_ROTL_C, IN_D, IN_E, IN_SR, IN_FREE, IN_FREE0,
            //                        IN_CTX, IN_SP, IN_PC, IN_STEP, IN_RR, IN_RCX, IND, IND_RR,
            //                        FLAGS, OFFSET, INC_STACK, JMP_ADDR, ELSE_ADDR, LINE], mul);

            this.addValue(BigInt(line), 'LINE');
            this.addValue(l.condConst ? BigInt(l.condConst) : 0n, 'condConst');
            this.addValue(l.inA ? BigInt(l.inA) : 0n, 'inA');
            this.addValue(l.inB ? BigInt(l.inB) : 0n, 'inB');
            this.addValue(l.inC ? BigInt(l.inC) : 0n, 'inC');
            this.addValue(l.inROTL_C ? BigInt(l.inROTL_C) : 0n, 'inROTL_C');
            this.addValue(l.inD ? BigInt(l.inD) : 0n, 'inD');
            this.addValue(l.inE ? BigInt(l.inE) : 0n, 'inE');
            this.addValue(l.inSR ? BigInt(l.inSR) : 0n, 'inSR');
            this.addValue(l.inFREE ? BigInt(l.inFREE) : 0n, 'inFREE');
            this.addValue(l.inFREE0 ? BigInt(l.inFREE0) : 0n, 'inFREE0');
            this.addValue(l.inCTX ? BigInt(l.inCTX) : 0n, 'inCTX');
            this.addValue(l.inSP ? BigInt(l.inSP) : 0n, 'inSP');
            this.addValue(l.inPC ? BigInt(l.inPC) : 0n, 'inPC');
            this.addValue(l.inSTEP ? BigInt(l.inSTEP) : 0n, 'inSTEP');
            this.addValue(l.inRR ? BigInt(l.inRR) : 0n, 'inRR');
            this.addValue(l.inRCX ? BigInt(l.inRCX) : 0n, 'inRCX');
            this.addValue(l.ind ? BigInt(l.ind) : 0n, 'ind');
            this.addValue(l.indRR ? BigInt(l.indRR) : 0n, 'indRR');
            this.addValue(l.offset ? BigInt(l.offset) : 0n, 'offset');
            this.addValue(l.incStack ? BigInt(l.incStack) : 0n, 'incStack');
            this.addValue(l.jmpAddr ? BigInt(l.jmpAddr) : 0n, 'jmpAddr');
            this.addValue(l.elseAddr ? BigInt(l.elseAddr) : 0n, 'elseAddr');

            let flags = 0n;
            let factor = 1n;
            for (const flag of ROM_FLAGS) {
                const bit = typeof l[flag] === 'undefined' ? 0n : BigInt(l[flag]);
                if (bit !== 0n && bit !== 1n) {
                    throw new Error(`Invalid value for ${flag} ${bit} on ${l.fileName}:${l.line}`);
                }
                if (bit) this.addLabel(flag);
                flags += bit * factor;
                factor = factor * 2n;
            }
            this.addValue(flags);

            if (l.CONST) {
                const constValue = typeof l.CONST === 'undefined' ? 0n : BigInt(l.CONST);
                if (constValue && (constValue > INT32_MAX || constValue < INT32_MIN)) {
                    throw new Error(`Invalid value for CONST ${constValue} on ${l.fileName}:${l.line}`);
                }
                this.addValue(Fr.e(constValue), 'CONST');
                for (let index = 1; index < 8; ++index) {
                    this.addValue(Fr.zero);
                }
            } else {
                let constValue = typeof l.CONSTL === 'undefined' ? 0n : BigInt(l.CONSTL);
                if (constValue && (constValue > P256_MAX || constValue < 0n)) {
                    throw new Error(`Invalid value for CONSTL ${constValue} on ${l.fileName}:${l.line}`);
                }
                if (constValue) {
                    this.addLabel(`CONSTL: ${constValue}`);
                }
                for (let index = 0; index < 8; ++index) {
                    this.addValue(constValue & MASK_32);
                    constValue = constValue >> 32n;
                }
            }

            this.flushValues(l.fileName, l.line, l.lineStr, line == rom.length - 1);
            //            const extraLn = (previousFileName !== false && previousFileName !== l.fileName) ? '\n':'';
            //            previousFileName = l.fileName;

            //            // [LINE, ROM_FLAGS, CONST0, JMP_ADDRESS, IN_SIBLING_RKEY]);
            //            data.push({values: [line, '0x'+flags.toString(16).toUpperCase().padStart(FLAGS_DIGITS, '0'), CONST0, JMP_ADDRESS, IN_SIBLING_RKEY], source:l.fileName, line: l.line, linestr: l.lineStr.trimEnd(), extraLn, flags: activeFlags.join(', ')});
        }
/*        let widths = data.length ? data[0].values.map(x => 0) : [];
        let sourceWidth = 0;
        let lineWidth = 0;
        let lineStrWidth = 0;
        let flagsWidth = 0;
        for (let ldata of data) {
            ldata.values = ldata.values.map(x => typeof x === 'string' ? x : x.toString());
            widths = widths.map((w, i) => w > ldata.values[i].length ? w : ldata.values[i].length);
            sourceWidth = sourceWidth > ldata.source.length ? sourceWidth : ldata.source.length;
            const _line = ldata.line.toString();
            lineWidth = lineWidth > _line.length ? lineWidth : _line.length;
            lineStrWidth = lineStrWidth > ldata.linestr.length ? lineStrWidth : ldata.linestr.length;
            flagsWidth = flagsWidth > ldata.flags.length ? flagsWidth : ldata.flags.length;
        }*/
        let lines = '';
        /*
        const lineWidth = data.length.toString().length;
        for (let index = 0; index < data.length; ++index) {
            const ldata = data[index];
            if (ldata.extraLn) lines += '\n';
            const line = index.toString().padStart(lineWidth);
            lines += `\t[LINE[${line}], ROM_FLAGS[${line}], CONST0[${line}], JMP_ADDRESS[${line}], IN_SIBLING_RKEY[${line}]] = [` + ldata.values.map((x,i) => x.padStart(widths[i])).join() + ']; // ' + ldata.source.padEnd(sourceWidth) + ' ' + ldata.linestr + '\n';
        }
        */
        // const twidths = widths.reduce((t,x) => t+x, 0) + 10 + widths.length - 1;
        // for (let index = 0; index < data.length; ++index) {
        //     const ldata = data[index];
        //     if (ldata.extraLn) lines += '\n\t'+' '.repeat(twidths) + '// '+ ldata.source + '\n\n';
        //     lines += `\tsource(` + ldata.values.map((x,i) => x.padStart(widths[i])).join() + '); // #' + ldata.line.toString().padEnd(lineWidth) + ' ' + ldata.linestr.padEnd(lineStrWidth) + ' # flags = ' + ldata.flags + '\n';
        // }
        // console.log(lines);
    }
    addValue(value, label = false) {
        this.values.push(value);
        if (label !== false && value) {
            this.labels.push(label+': '+value);
        }
    }
    addLabel(label) {
        this.labels.push(label);
    }
    flushValues(fileName, line, lineStr, last = false) {
        const extraLn = (this.previousFileName !== false && this.previousFileName !== fileName) ? '\n':'';
        this.previousFileName = fileName;
        if (extraLn) console.log('');
        const sourceRef = `${fileName}:${line}`.padEnd(fileName.length > 20 ? fileName.length + 4 : 24);
        console.log(`\n\t// ${sourceRef} ${lineStr}`);
        const callStr = (last ? 'last':'add') + '_rom_line('+this.values[0]+',['+this.values.slice(1).join()+']);';
        console.log('\t'+callStr.padEnd(90)+' // '+this.labels.join(', '));
        this.values = [];
        this.labels = [];
    }
}

const rom2pil = new Rom2Pil(__dirname + '/../rom/rom.json');
