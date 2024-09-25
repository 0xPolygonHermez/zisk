const {assert} = require('chai');
const Register = require('./register.js');
const Context = require('../context.js');

module.exports = class LargeRegister extends Register {
    constructor(label, valueCol, chunks, inCol, setCol, inRom, setRom) {
        super(label, inCol, setCol, inRom, setRom);
        this.valueCol = valueCol;
        this.chunks = chunks;
        this.resetValue();
    }
    applyInToValue(inColValue) {
        return this.value.map(x => Context.fr.mul(inColValue, x));        
    }
    resetValue() {
        this.value = new Array(this.chunks).fill(Context.fr.zero);
    }
    updateCols(row) {        
        // check if virtual register
        if (typeof this.valueCol === 'function') {
            return;
        }
        for (let index = 0; index < this.chunks; ++index) {
            this.valueCol[index][row] = this.value[index];
        }
    }
    updateValue(value, row = false) {
        assert(this.value.length === value.length);
        this.value = [...value];
        if (row !== false) {
            this.updateCols(row);
        }
    }
    dump() {        
        console.log(`${this.label}: ${this.value.map(x => x.toString(16)).join()}`);
    }
    getValue() {
        // check if virtual register
        if (typeof this.valueCol === 'function') {
            return this.valueCol();
        }
        return this.value;
    }
}
