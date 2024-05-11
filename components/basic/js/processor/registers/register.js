const Context = require('../context.js');
const {assert} = require('chai');

module.exports = class Register {
    constructor(label, inCol, setCol, inRom, setRom) {
        this.label = label;
        this.inCol = inCol;
        this.setCol = setCol;
        this.inRom = inRom;
        this.setRom = setRom;
    }
    reset(row) {
        this.resetValue();
        this.updateCols(row);
    }
    getInValue(row, romline) {        
        if (!romline[this.inRom]) {
            this.inCol[row] = Context.fr.zero;
            return false;
        }
        const inColValue = Context.fr.e(romline[this.inRom]);
        this.inCol[row] = inColValue;
        return this.applyInToValue(inColValue);
    }
    applySetValue(row, romline, value) {
        if (!romline[this.setRom]) {
            if (this.setCol !== false) this.setCol[row] = Context.fr.zero;
            // console.log(`\x1B[35m ==> ${this.label} = ${this.value} (${row})\x1B[0m`);
            this.updateCols(row);
            return;
        }
        assert(this.setCol !== false, `couldn't set value for register ${this.label}`);
        this.setCol[row] = Context.fr.one;

        // console.log(`\x1B[35m ==> ${this.label} = ${value} (${row})\x1B[0m`);
        this.updateValue(value, row);
    }
    applyInToValue(inColValue) {
        throw new Error('applyInToValue not implemented');
    }
    updateCols(row) {
        throw new Error('updateCols not implemented');
    }
    resetValue() {
        throw new Error('resetValue not implemented');
    }
    updateValue(value, row) {
        throw new Error('updateValue not implemented');
    }
}
