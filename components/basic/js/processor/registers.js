const {assert} = require('chai');
const Context = require('./context.js');
const LargeRegister = require('./registers/large.js');
const SingleRegister = require('./registers/single.js');

module.exports = class Registers {
    constructor() {
        this.registers = {};
    }
    checkDefine(name, valueCol, inCol, setCol, inRomProp, setRomProp, chunks = false) {
        assert(typeof this.registers[name] === 'undefined', `register ${name} already defined`);
        assert(typeof valueCol !== 'undefined', `valueCol of register ${name} not defined`);
        if (chunks !== false) {
            assert(Array.isArray(valueCol), `valueCol of register ${name} is not array`);
            assert(valueCol.length === chunks, `chuncks of valueCol of register ${name} not match (len:${valueCol.length},chunks:${chunks})`);
        }
        assert(typeof inCol !== 'undefined', `inCol of register ${name} not defined`);
        assert(typeof setCol !== 'undefined', `setCol of register ${name} not defined`);
        assert(typeof inRomProp !== 'undefined', `inRomProp of register ${name} not defined`);
        assert(typeof setRomProp !== 'undefined', `setRomProp of register ${name} not defined`);
    }
    defineLarge(name, valueCol, chunks, inCol, setCol, inRomProp, setRomProp) {
        this.checkDefine(name, valueCol, inCol, setCol, inRomProp, setRomProp);
        return this.registers[name] = new LargeRegister(name, valueCol,  chunks, inCol, setCol, inRomProp, setRomProp);
    }
    defineSingle(name, valueCol, inCol, setCol, inRomProp, setRomProp) {
        this.checkDefine(name, valueCol, inCol, setCol, inRomProp, setRomProp);
        return this.registers[name] = new SingleRegister(name, valueCol, inCol, setCol, inRomProp, setRomProp);
    }
    addInValues(row, romline, value) {
        for (const regname in this.registers) {
            const register = this.registers[regname];
            const regValue = register.getInValue(row, romline);
            if (regValue === false) continue;
            if (!Array.isArray(regValue)) {
                value[0] = Context.fr.add(value[0], regValue);
                continue;
            }
            for (let index = 0; index < regValue.length; ++index) {
                value[index] = Context.fr.add(value[index], regValue[index]);
            }
        }
        return value;
    }
    applySetValue(setRow, regRow, romline, value) {
        for (const regname in this.registers) {
            this.registers[regname].applySetValue(setRow, regRow, romline, value);
        }
    }
    dump() {
        for (const regname in this.registers) {
            this.registers[regname].dump();
        }
    }
    reset(row) {
        for (const regname in this.registers) {
            this.registers[regname].reset(row);
        }
    }
    getValue(name) {
        return this.registers[name].getValue();
    }
    setValue(name, value, row = false) {
        this.registers[name].updateValue(value, row);
    }
};

