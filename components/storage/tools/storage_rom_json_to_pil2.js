const fs = require('fs');
const { F1Field } = require("ffjavascript");

const Fr = new F1Field("0xFFFFFFFF00000001");

const INT_MIN = -(2n ** 32n) + 1n;
const INT_MAX = (2n ** 32n) - 1n;

const ROM_FLAGS = ['hash', 'hashType', 'latchGet', 'latchSet', 'climbRkey', 'climbSiblingRkey', 'climbBitN',
        'jmp', 'jmpz', 'jmpnz', 'setHASH_LEFT', 'setHASH_RIGHT', 'setLEVEL', 'setNEW_ROOT', 'setOLD_ROOT',
        'setRKEY', 'setRKEY_BIT', 'setSIBLING_RKEY', 'setSIBLING_VALUE_HASH', 'setVALUE_HIGH', 'setVALUE_LOW',
        'inFREE', 'inNEW_ROOT', 'inOLD_ROOT', 'inRKEY', 'inRKEY_BIT', 'inSIBLING_VALUE_HASH',
        'inVALUE_LOW', 'inVALUE_HIGH', 'inROTL_VH', 'inLEVEL'];

function loadStorageRom(StorageRomFile) {
    // Init rom from file
    const rawdata = fs.readFileSync(StorageRomFile);
    const json = JSON.parse(rawdata);
    const rom = json.program;
    const JMP_ADDRESS_MAX = BigInt(rom.length - 1);
    const FLAGS_DIGITS = Math.ceil(ROM_FLAGS.length / 4);

    let data = [];
    let previousFileName = false;
    for (let line = 0; line < rom.length; line++) {
        const l = rom[line];

        const CONST0 = typeof l.CONST === 'undefined' ? 0n : BigInt(l.CONST);
        if (CONST0 && (CONST0 > INT_MAX || CONST0 < INT_MIN)) {
            throw new Error(`Invalid value for CONST ${CONST0} on ${l.fileName}:${l.line}`);
        }
       
        const JMP_ADDRESS = typeof l.JMP_ADDRESS === 'undefined' ? 0n : BigInt(l.JMP_ADDRESS);
        if (JMP_ADDRESS && (JMP_ADDRESS > JMP_ADDRESS_MAX || JMP_ADDRESS < 0n)) {
            throw new Error(`Invalid value for JMP_ADDRESS ${JMP_ADDRESS} on ${l.fileName}:${l.line}`);
        }

        const IN_SIBLING_RKEY = typeof l.IN_SIBLING_RKEY === 'undefined' ? 0n : BigInt(l.IN_SIBLING_RKEY);
        if (IN_SIBLING_RKEY && (IN_SIBLING_RKEY > INT_MAX || IN_SIBLING_RKEY < INT_MIN)) {
            throw new Error(`Invalid value for IN_SIBLING_RKEY ${CONST0} on ${l.fileName}:${l.line}`);
        }
        
        let flags = 0n;
        let activeFlags = [];
        let factor = 1n;
        for (const flag of ROM_FLAGS) {
            const bit = typeof l[flag] === 'undefined' ? 0n : BigInt(l[flag]);
            if (bit !== 0n && bit !== 1n) {
                throw new Error(`Invalid value for ${flag} ${bit} on ${l.fileName}:${l.line}`);
            }
            if (bit) activeFlags.push(flag);
            flags += bit * factor;
            factor = factor * 2n;
        }
        const extraLn = (previousFileName !== false && previousFileName !== l.fileName) ? '\n':'';
        previousFileName = l.fileName;
    
        // [LINE, ROM_FLAGS, CONST0, JMP_ADDRESS, IN_SIBLING_RKEY]);
        data.push({values: [line, '0x'+flags.toString(16).toUpperCase().padStart(FLAGS_DIGITS, '0'), CONST0, JMP_ADDRESS, IN_SIBLING_RKEY], source:l.fileName, line: l.line, linestr: l.lineStr.trimEnd(), extraLn, flags: activeFlags.join(', ')});
    }
    let widths = data.length ? data[0].values.map(x => 0) : [];
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
    }
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
    const twidths = widths.reduce((t,x) => t+x, 0) + 10 + widths.length - 1;
    for (let index = 0; index < data.length; ++index) {
        const ldata = data[index];
        if (ldata.extraLn) lines += '\n\t'+' '.repeat(twidths) + '// '+ ldata.source + '\n\n';
        lines += `\tsource(` + ldata.values.map((x,i) => x.padStart(widths[i])).join() + '); // #' + ldata.line.toString().padEnd(lineWidth) + ' ' + ldata.linestr.padEnd(lineStrWidth) + ' # flags = ' + ldata.flags + '\n';
    }
    console.log(lines);
}

loadStorageRom(__dirname + '/storage_rom.json');
