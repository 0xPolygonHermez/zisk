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
    'useJmpAddrRel', 'useElseAddrRel', 'jmp', 'jmpn', 'jmpz', 'call', 'returnJmp', 'repeat',
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

            // col fixed CONST[NR];
            // col fixed COND_CONST, OFFSET;
            // col fixed IN_A, IN_B, IN_C, IN_ROTL_C, IN_D, IN_E, IN_SR, IN_FREE, IN_FREE0, 
            //           IN_CTX, IN_SP, IN_PC, IN_STEP, IN_RR, IN_RCX, IND, IND_RR,
            //           INC_STACK, JMP_ADDR, ELSE_ADDR, LINE;
            // col fixed FLAGS;

            this.addValue(BigInt(line), 'LINE');
            this.addValue(l.IN_A ? BigInt(IN_A) : 0n, 'l.IN_A');
            this.addValue(l.COND_CONST ? BigInt(COND_CONST) : 0n, 'COND_CONST');
            this.addValue(l.OFFSET ? BigInt(OFFSET) : 0n, 'OFFSET');
            this.addValue(l.IN_A ? BigInt(IN_A) : 0n, 'IN_A');
            this.addValue(l.IN_B ? BigInt(IN_B) : 0n, 'IN_B');
            this.addValue(l.IN_C ? BigInt(IN_C) : 0n, 'IN_C');
            this.addValue(l.IN_ROTL_C ? BigInt(IN_ROTL_C) : 0n, 'IN_ROTL_C');
            this.addValue(l.IN_D ? BigInt(IN_D) : 0n, 'IN_D');
            this.addValue(l.IN_E ? BigInt(IN_E) : 0n, 'IN_E');
            this.addValue(l.IN_SR ? BigInt(IN_SR) : 0n, 'IN_SR');
            this.addValue(l.IN_FREE ? BigInt(IN_FREE) : 0n, 'IN_FREE');
            this.addValue(l.IN_FREE0 ? BigInt(IN_FREE0) : 0n, 'IN_FREE0');
            this.addValue(l.IN_CTX ? BigInt(IN_CTX) : 0n, 'IN_CTX');
            this.addValue(l.IN_SP ? BigInt(IN_SP) : 0n, 'IN_SP');
            this.addValue(l.IN_PC ? BigInt(IN_PC) : 0n, 'IN_PC');
            this.addValue(l.IN_STEP ? BigInt(IN_STEP) : 0n, 'IN_STEP');
            this.addValue(l.IN_RR ? BigInt(IN_RR) : 0n, 'IN_RR');
            this.addValue(l.IN_RCX ? BigInt(IN_RCX) : 0n, 'IN_RCX');
            this.addValue(l.IND ? BigInt(IND) : 0n, 'IND');
            this.addValue(l.IND_RR ? BigInt(IND_RR) : 0n, 'IND_RR');
            this.addValue(l.INC_STACK ? BigInt(INC_STACK) : 0n, 'INC_STACK');
            this.addValue(l.JMP_ADDR ? BigInt(JMP_ADDR) : 0n, 'JMP_ADDR');
            this.addValue(l.ELSE_ADDR ? BigInt(ELSE_ADDR) : 0n, 'ELSE_ADDR');

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
