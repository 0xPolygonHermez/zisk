const Component = require('../../../src/component.js');
const DEFAULT_MEMORY_ID = 4;
module.exports = class Mem extends Component {
    constructor(config = {}) {
        super(config);
        this.inputChunks = config.inputChunks;
        this.emptyValue = new Array(this.inputChunks).fill(0n);
        this.mem = [];
    }
    getDefaultId() {
        return DEFAULT_MEMORY_ID;
    }

    // 0:id, 1:addr, 2:step 3:mWr 4:value[n]
    calculateVerify(verify, values) {
        const ic = this.inputChunks;
        const [id, addr, step, wr] = values.slice(0, 4);
        const value = values.slice(4);
        if (!verify && wr) {
            return false;
        }
        const _value = this.#calculate(addr);
        if (!verify) {
            // no verify => calculating .... return the value
            return _value;
        }

        // verify and read, value expected and assumed must be the same
        if (!wr && !this.isEqual(value, _value)) {
            return this.onVerifyFails(`Memory result doesn't match on address 0x${addr.toString(16)} `
                                        + `expected: ${_value.join()} vs: ${value.join()}`);
        }

        // verify = true && (read || write)
        this.proves(BigInt(id), addr, step, wr, value);
        return true;
    }
    #calculate(addr) {
        return this.mem[addr] ?? this.emptyValue;
    }
    proves(id, addr, step, wr, value) {
        if (wr) this.mem[addr] = value;
        this.inputs.push([id, addr, step, wr, value]);
    }
    // TO-DO: verify at end of execution that distance beetwen address no exceeds maximum
    execute(cols) {

        this.input.sort((a,b) => {
            if (a.addr == b.addr) {
                return a.step - b.step;
            } else {
                return a.address - b.address;
            }
        });

        const n = cols.addr.length;
        const count = this.input.length;

        let rowIndex = 0;
        let inputIndex = 0;
        while (inputIndex < count) {
            const [, addr, step, wr, value] = this.inputs[inputIndex];
            const nextAddr = (inputIndex + 1) < count ? this.inputs[inputIndex + 1] : false;
            const isDouble = nextAddr === addr && !this.inputs[inputIndex + 1].wr;
            cols.addr[rowIndex] = BigInt(addr);
            cols.step[0][rowIndex] = BigInt(step);
            cols.step[1][rowIndex] = BigInt(isDouble ? this.inputs[inputIndex + 1].step : step);
            cols.isDouble[rowIndex] = isDouble ? 1n : 0n;
            cols.sel[rowIndex] = 1n;
            cols.mWr[rowIndex] = wr ? 1n : 0n;
            cols.lastAccess[rowIndex - 1] = nextAddr === addr ? 0n : 1n;
            this.assignValues(cols.value, value);
            inputIndex = inputIndex + (isDouble ? 2 : 1);
            ++rowIndex;
        }
        const paddingAddress = BigInt(this.inputs.length > 0 ? this.inputs.slice(-1).addr + 1 : 1);
        while (rowIndex < n) {
            cols.addr[rowIndex] = paddingAddress;
            cols.step[0][rowIndex] = BigInt(rowIndex);
            cols.step[1][rowIndex] = BigInt(rowIndex);
            cols.sel[rowIndex] = 0n;
            cols.wr[rowIndex] = 0n;
            cols.isDouble[rowIndex] = 0n;
            this.assignValues(cols.value, this.emptyValue);
            ++rowIndex;
        }   
        cols.lastAccess[n - 1] = 1n;
    }
}
