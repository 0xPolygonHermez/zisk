const {assert} = require("chai");

class ContextImpl {
    static instance;
    static getInstance() {
        if (!ContextImpl.instance) {
            throw new Error('Context not initialized');
        }
        return ContextImpl.instance;
    }
    static setup(config) {
        if (ContextImpl.instance) {
            throw new Error('Context already initialized');
        }        
        return ContextImpl.instance = new ContextImpl(config);    
    }
    constructor(config = {}) {
        this.fr = config.fr;
        this.chunks = config.chunks || 8;
        this.zeroValue = new Array(this.chunks).fill(this.fr.zero);
        this.N = config.N;
        this.step = 0;
        this.row = 0;
        this.registers = config.registers;
        this.sourceRef = '';
    }
}

module.exports = class Context{
    static setup(config) {
        return ContextImpl.setup(config);
    }
    static get fr () {
        return ContextImpl.getInstance().fr;
    }
    static get zeroValue() {
        return [...ContextImpl.getInstance().zeroValue];
    }
    static get chunks() {
        return ContextImpl.getInstance().chunks;
    }
    static get N() {
        return ContextImpl.getInstance().N;
    }
    static get step() {
        return ContextImpl.getInstance().step;
    }
    static get row() {
        return ContextImpl.getInstance().row;
    }
    static getRegValue(regname) {
        return ContextImpl.getInstance().registers.getValue(regname);
    }
    static sourceRef() {
        return ContextImpl.getInstance().sourceRef;
    }
}