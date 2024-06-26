const {assert} = require('chai');
const Context = require('./context.js');
const Registers = require('./registers.js');
const F1Field = require('ffjavascript').F1Field;
const fs = require('fs');
const Memory = require('../../../mem/js/mem.js');
const Command = require('./command.js');
const Debug = require('./debug.js');

const FF_PRIME = 0xFFFFFFFF00000001n;

module.exports = class Processor
{
    constructor(cols, config = {}) {
        this.config = config;
        this.cols = cols;
        this.N = this.cols.zkPC.length;
        this.proofCtx = config.proofCtx;
        this.setupFr();
        this.setupChunks();
        this.registers = new Registers();
        this.context = Context.setup({  fr: this.fr,
                                        chunks: this.chunks,
                                        N: this.N,
                                        registers: this.registers});
        this.romToMainLinks = {};
        this.romConst = 'CONST';
        this.romConstl = 'CONSTL';
        this.components = {};
        this.command = new Command();
        this.setupRegisters();
        this.setupRomToMainLinks();
        this.registerComponents();
        this.registerHelpers();
        this.loadRom();
    }
    setupChunks() {
        this.chunks = 8;
        this.chunkBits = BigInt(32);
        this.chunkMask = (1n << this.chunkBits) - 1n;
    }
    setupFr() {
        this.fr = new F1Field(FF_PRIME);
        this.frFirst32BitsNegative = FF_PRIME - (2n ** 32n);
        this.frLast32BitsPositive = (2n ** 32n) - 1n;
        this.frZeroOne = [this.fr.zero, this.fr.one];
    }
    setupRegisters() {
        this.addr = 0;
        this.addrRel = 0;
        this.defineSingleRegisters();
        this.defineLargeRegisters();
    }
    loadRom() {
        this.romJson = JSON.parse(fs.readFileSync(this.config.romFile, 'utf8'));
        this.rom = this.romJson.program;
    }
    registerHelperCall(funcname, object, method) {
        return this.command.registerHelperCall(funcname, object, method);
    }
    defineSingleRegisters() {
        const cols = this.cols;
        this.defineSingleRegister('SP', cols.SP, cols.inSP, cols.setSP, 'inSP', 'setSP');
        this.defineSingleRegister('PC', cols.PC, cols.inPC, cols.setPC, 'inPC', 'setPC');
        this.defineSingleRegister('RR', cols.RR, cols.inRR, cols.setRR, 'inRR', 'setRR');
        this.defineSingleRegister('CTX', cols.CTX, cols.inCTX, cols.setCTX, 'inCTX', 'setCTX');
        this.defineSingleRegister('RCX', cols.RCX, cols.inRCX, cols.setRCX, 'inRCX', 'setRCX');
    }
    defineLargeRegisters() {
        const cols = this.cols;
        this.defineLargeRegister('A', cols.A, cols.inA, cols.setA, 'inA', 'setA');
        this.defineLargeRegister('B', cols.B, cols.inB, cols.setB, 'inB', 'setB');
        const C = this.defineLargeRegister('C', cols.C, cols.inC, cols.setC, 'inC', 'setC');
        this.defineLargeRegister('D', cols.D, cols.inD, cols.setD, 'inD', 'setD');
        this.defineLargeRegister('E', cols.E, cols.inE, cols.setE, 'inE', 'setE');
        this.defineLargeRegister('SR', cols.SR, cols.inSR, cols.setSR, 'inSR', 'setSR');
        const FREE = this.defineLargeReadOnlyRegister('FREE', cols.FREE, cols.inFREE, 'inFREE');

        // virtual registers
        this.defineSingleReadOnlyRegister('STEP', () => this.fr.e(this.row), cols.inSTEP, 'inSTEP');
        this.defineLargeReadOnlyRegister('FREE0', () => FREE.getValue()[0], cols.inFREE0, 'inFREE0');
        this.defineLargeReadOnlyRegister('ROTL_C', () => this.rotateLeft(C), cols.inROTL_C, 'inROTL_C');
    }
    defineCustomRegisterTransitions() {
        this.defineCustomRegisterTransition('SP', this.defaultTransitionSP);
    }
    defaultTransitionZkPC() {
    }
    defaultTransitionSP() {
    }
    defineLargeRegister(name, valueCol, inCol, setCol, inRomProp, setRomProp) {
        return this.registers.defineLarge(name, valueCol, this.chunks, inCol, setCol, inRomProp, setRomProp);
    }
    defineSingleRegister(name, valueCol, inCol, setCol, inRomProp, setRomProp) {
        return this.registers.defineSingle(name, valueCol, inCol, setCol, inRomProp, setRomProp);
    }
    defineLargeReadOnlyRegister(name, valueCol, inCol, inRomProp) {
        return this.registers.defineLarge(name, valueCol, this.chunks, inCol, false, inRomProp, false);
    }
    defineSingleReadOnlyRegister(name, valueCol, inCol, inRomProp) {
        return this.registers.defineSingle(name, valueCol, inCol, false, inRomProp, false);
    }
    rotateLeft(reg) {
        const chunkValues = reg.getValue();
        return [chunkValues[this.chunks - 1],...chunkValues.slice(0, this.chunks - 1)];
    }
    calculateRelativeAddress() {
        this.addrRel = 0;

        if (this.romline.ind) {
            this.addrRel += this.registers.getValue('E')[0];
        }

        if (this.romline.indRR) {
            this.addrRel += this.registers.getValue('RR');
        }

        if (typeof this.romline.maxInd !== 'undefined' && this.addrRel > this.romline.maxInd) {
            const index = this.romline.offset - this.romline.baseLabel + this.addrRel;
            throw new Error(`Address out of bounds accessing index ${index} but ${this.romline.offsetLabel}[${this.romline.sizeLabel}] ind:${this.addrRel}`);
        }
    }
    setupRomToMainLinks() {
        this.linkRomFlagToMainCol('isStack', this.cols.isStack);
        this.linkRomFlagToMainCol('isMem', this.cols.isMem);
        this.linkRomFlagToMainCol('mOp', this.cols.mOp);
        this.linkRomFlagToMainCol('mWR', this.cols.mWR);
        this.linkRomFlagToMainCol('memUseAddrRel', this.cols.memUseAddrRel);
        this.linkRomFlagToMainCol('useCTX', this.cols.useCTX);
        this.linkRomConstToMainCol('incStack', this.cols.incStack);
        this.linkRomConstToMainCol('ind', this.cols.ind);
        this.linkRomConstToMainCol('indRR', this.cols.indRR);
        this.linkRomConstToMainCol('offset', this.cols.offset);
        this.linkRomFlagToMainCol('doAssert', this.cols.doAssert);
        this.linkRomFlagToMainCol('assumeFREE', this.cols.assumeFREE);

        this.linkRomFlagToMainCol('JMP', this.cols.jmp);
        this.linkRomFlagToMainCol('JMPN', this.cols.jmpn);
        this.linkRomFlagToMainCol('JMPZ', this.cols.jmpz);
        this.linkRomFlagToMainCol('call', this.cols.call);
        this.linkRomFlagToMainCol('return', this.cols.returnJmp);

        this.linkRomFlagToMainCol('jmpUseAddrRel', this.cols.jmpUseAddrRel);
        this.linkRomFlagToMainCol('elseUseAddrRel', this.cols.elseUseAddrRel);
        this.linkRomFlagToMainCol('repeat', this.cols.repeat);

        this.linkRomConstToMainCol('condConst', this.cols.condConst);
        this.linkRomConstToMainCol('jmpAddr', this.cols.jmpAddr);
        this.linkRomConstToMainCol('elseAddr', this.cols.elseAddr);
    }
    linkRomFlagToMainCol(romFlag, col) {
        assert(typeof this.romToMainLinks[romFlag] === 'undefined');
        this.romToMainLinks[romFlag] = {binary: true, col, chunks: false};
    }
    linkRomConstToMainCol(romConst, col, chunks = false) {
        assert(typeof this.romToMainLinks[romConst] === 'undefined');
        this.romToMainLinks[romConst] = {binary: false, col, chunks};
    }
    updateRomToMainLinkedCols() {
        for (const romProp in this.romToMainLinks) {
            const link = this.romToMainLinks[romProp];
            // single binary links
            if (link.binary) {
                link.col[this.row] = this.romline[romProp] ? this.fr.one : this.fr.zero;
                continue;
            }
            // single non-binary links
            if (link.chunks === false) {
                link.col[this.row] = this.romline[romProp] ? this.fr.e(this.romline[romProp]) : this.fr.zero;
                continue;
            }
            // multi-chunk non-binary links
            for (let index = 0; index < link.chunks; ++index) {
                link.col[index][this.row] = this.romline[romProp][index] ? this.fr.e(this.romline[romProp][index]) : this.fr.zero;
            }
        }
    }
    calculateMemoryAddress() {
        this.addr = this.romline.offset ?? 0;

        if (this.romline.useCTX) {
            addr += Number(this.registers.getValue('CTX'))*0x40000;
        }

        if (this.romline.isStack) {
            addr += 0x10000;
        }

        if (this.romline.isMem) {
            addr += 0x20000;
        }

        if (this.romline.memUseAddrRel) {
            addr += this.addrRel;
        }
    }
    initRegs() {
        this.opValue = Context.zeroValue;
        // TODO initalize publics
        this.registers.reset(0);
        this.zkPC = 0;
    }
    execute(input) {
        const stepsN = this.N;
        this.initRegs();
        this.initComponents();
        for (let step = 0; step < stepsN; step++) {
            this.setStep(step);
            this.setRomLineAndZkPC();

            // selectors, component, mapping (lookup/permutation)

            this.evalPreCommands();
            this.calculateFreeInput();
            this.opValue = this.addInValues(this.getConstValue());
            this.calculateRelativeAddress();
            this.updateRomToMainLinkedCols();
            this.verifyComponents();
            this.manageFlowControl();
            this.applySetValues();
            // this.registers.dump();
            this.evalPostCommands();
        }
        this.finishComponents();
    }
    manageFlowControl() {
        // calculate all flow control values
        // TODO: call flag

        const condConst = this.romline.condConst ?? 0;

        const jmpAddr = this.romline.jmpAddr ?? 0;
        const elseAddr = this.romline.elseAddr ?? 0;

        const finalJmpAddr = jmpAddr + (this.romline.jmpUseAddrRel ? this.addrRel : 0);
        const finalElseAddr = elseAddr + (this.romline.elseUseAddrRel ? this.addrRel : 0);

        const insideRepeat = this.romline.repeat ? 1 : 0;

        const insideRepeatLoop = (insideRepeat && !Fr.isZero(this.RCX.getValue()));
        let nextZkPC = this.zkPC + (insideRepeatLoop ? 0:1);
        const op0cond = this.fr.e(this.opValue[0] + BigInt(condConst));

        let isNegative = 0;
        const op0Inv = this.fr.isZero(op0cond) ? this.fr.zero : this.fr.inv(op0cond);

        if (this.romline.JMPN) {
            if (op0cond >= this.frFirst32BitsNegative) {
                isNegative = 1;
                nextZkPC = finalJmpAddr;
            } else if (op0cond <= this.frLast32BitsPositive) {
                nextZkPC = elseAddr;
            } else {
                throw new Error(`On JMPN value ${op0cond} not a valid 32bit value ${Context.sourceRef}`);
            }
        } else {
            if (this.romline.JMPZ) {
                if (this.fr.isZero(op0cond)) {
                    nextZkPC = finalJmpAddr;
                } else {
                    nextZkPC = finalElseAddr;
                }
            } else if (this.romline.JMP) {
                nextZkPC = finalJmpAddr;
            } else if (this.romline.return) {
                nextZkPC = Number(this.RR.getValue());
            }
        }

        this.zkPC = nextZkPC;

        this.cols.isNeg[this.row] = this.frZeroOne[isNegative];
        this.cols.op0Inv[this.row] = op0Inv;
        this.cols.RCXInv[this.row] = insideRepeatLoop ? this.fr.inv(this.RCX.getValue()) : this.fr.zero;
    }
    initComponents() {
        for (const romFlag in this.components) {
            this.components[romFlag].helper.init(this);
        }
    }
    verifyComponents() {
        for (const romFlag in this.components) {
            if (!this.romline[romFlag]) continue;
            const componentInfo = this.components[romFlag];
            componentInfo.method.apply(this, [true, componentInfo.id, componentInfo.helper]);
        }
    }
    finishComponents() {
        for (const romFlag in this.components) {
            this.components[romFlag].helper.finish();
        }
    }
    evalPreCommands() {
    }
    evalPostCommands() {
    }
    addInValues(constValues) {
        return this.registers.addInValues(this.row, this.romline, constValues);
    }
    applySetValues() {
        this.registers.applySetValue(this.row, this.nextRow, this.romline, this.opValue);
    }
    convertConstlValue(value) {
        return this.scalarToFea(BigInt(value));
    }
    getConstValue() {
        let value = Context.zeroValue;
        const romConstlValue = this.romline[this.romConstl];
        if (romConstlValue) {
            value = this.convertConstlValue(romConstlValue);
        }
        else if (this.romline[this.romConst]) {
            value[0] = Context.fr.e(this.romline[this.romConst]);
        }

        for (let index = 0; index < this.chunks; ++index) {
            this.cols.CONST[index][this.row] = value[index];
        }
        return value;
    }
    calculateFreeInput() {
        let fi = 0n;

        if (this.romline.inFREE || this.romline.inFREE0) {
            if (!this.romline.freeInTag) {
                throw new Error(`Instruction with freeIn without freeInTag ${Context.sourceRef}`);
            }

            const freeInTag = this.romline.freeInTag;
            if (freeInTag.op !== '') {
                fi = this.command.evalCommand(freeInTag);
            } else {
                let nHits = 0;
                for (const romFlag in this.components) {
                    if (!this.romline[romFlag]) continue;
                    const componentInfo = this.components[romFlag];
                    const res = componentInfo.method.apply(this, [false, componentInfo.id,  componentInfo.helper]);
                    if (res === false) continue;
                    fi = res;
                    ++nHits;
                }
                if (nHits==0) {
                   throw new Error(`Empty freeIn without a valid instruction ${Context.sourceRef}`);
                } else if (nHits>1) {
                   throw new Error(`Only one instruction that requires freeIn is allowed ${Context.sourceRef}`);
                }
            }
        }
        if (!Array.isArray(fi)) {
            fi = this.scalarToFea(fi);
        }
        this.registers.setValue('FREE', fi, this.row);
    }
    registerComponents() {
        this.proofCtx.memory = new Memory({fr: this.fr, inputChunks: this.chunks});
        this.registerComponent(['mOp'], false, this.proofCtx.memory, this.mainToMemory);
    }
    registerComponent(romFlags, id, helper, method) {
        if (id === false) {
            id = helper.getDefaultId();
        }
        for (const romFlag of romFlags) {
            this.components[romFlag] = {id, helper, method};
        }
    }
    registerHelpers() {
        this.registerHelper(new Debug());
    }
    registerHelper(helper) {
        helper.init(this);
    }
    setStep(step) {
        this.row = step;        
        this.nextRow = (this.row + 1) % this.N;
        this.context.row = this.row;
        this.context.step = step;
    }
    setRomLineAndZkPC() {
        this.cols.zkPC[this.row] = this.fr.e(this.zkPC);
        this.romline = this.rom[this.zkPC] ?? false;
        this.context.sourceRef = `${this.romline.fileName}:${this.romline.line} (zkPC:${this.zkPC} row:${this.row})`;
        assert(this.romline !== false);
        // console.log(`\x1B[1;35m#${this.row.toString().padStart(8, '_')} ROM${this.zkPC.toString().padStart(6,'_')} ${this.romline.lineStr}\x1B[0m`);
    }

    mainToMemory(verify, helperId, helper) {
        if (verify) {
            return helper.verify([helperId, this.addr, this.row, this.romline.mWR ? 1n : 0n, ...this.opValue]);
        }
        return helper.calculateFreeInput([helperId, this.addr, this.row, this.romline.mWR ? 1n : 0n]);
    }
    scalarToFea(value) {
        let res = []
        let index = 0;
        while (value && index < this.chunks) {
            res.push(value & this.chunkMask);
            value = value >> this.chunkBits;
            ++index;
        }
        while (index < this.chunks) {
            res.push(0n);
            ++index;
        }
        assert(value === 0n);
        return res;
    }
    dumpRow(row, source) {
        const colnames = Object.keys(this.cols);
        let values = [];
        try {
            for (const colname of colnames) {
                const col = this.cols[colname];
                const len = col.length;
                let changes = false;
                let value = '';
                if (len <= 16) {                    
                    let avalues = [];
                    for (let index = 0; index < len; ++index) {
                        const value = col[index][row];
                        if (row > 0 && value !== col[index][row-1]) {
                            changes = true;
                            avalues.push(`\x1B[33m${value}\x1B[0m`);
                            continue;
                        }
                        avalues.push(value);
                    }
                    value = '['+avalues.join(',')+']';
                } else {
                    value = col[row];
                    if (row > 0 && value !== col[row-1]) {
                        changes = true;
                        value = `\x1B[33m${value}\x1B[0m`;
                    }
                }
                values.push((changes ? `\x1B[1;36m${colname}\x1B[0m: `:`${colname}: `) + value);
            }
        } catch(e) {
        }
        console.log(`\x1B[32m${source.trimStart()}\x1B[0m`);
        console.log(`ROW[${row}]={${values.join(' ')}}`);
    }
    // required filled when verifys
}
