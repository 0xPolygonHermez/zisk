const Register = require('./register.js');
const Context = require('../context.js');

module.exports = class SingleRegister extends Register {
    constructor(label, valueCol, inCol, setCol, inRom, setRom) {
        super(label, inCol, setCol, inRom, setRom);
        this.valueCol = valueCol;
        this.resetValue();
    }
    applyInToValue(inColValue) {
        return Context.fr.mul(inColValue, this.value);  
    }
    resetValue() {
        this.value = Context.fr.zero;
    }
    updateCols(row) {
        // check if not virtual register
        if (typeof this.valueCol !== 'function') {
            // console.log(` ===> ${this.label} = ${this.value} (${row})`);
            this.valueCol[row] = this.value;
        }
    }
    updateValue(value, row = false) {
        this.value = value[0];
        if (row !== false) {
            this.updateCols(row);
        }
    }
    getValue() {
        // check if virtual register
        if (typeof this.valueCol === 'function') {
            return this.valueCol();
        }

        return this.value;
    }
    dump() {        
        console.log(`${this.label}: ${this.value}`);
    }

}