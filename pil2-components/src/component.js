const VALUE_TO_SCALAR_ARRAY = 0x400;
module.exports = class Component {
    constructor (config = {}) {
        this.fr = config.fr;
        this.inputs = [];
        this.lastMessage = false;
        this.enableVerifyExceptions = config.enableVerifyExceptions ?? true;
    }
    static get VALUE_TO_SCALAR_ARRAY() { return VALUE_TO_SCALAR_ARRAY; }
    init(parent) {
        this.parent = parent;
        this.registerHelperCalls();
    }
    finish() {
    }
    registerHelperCall(funcname, method) {
        this.parent.registerHelperCall(funcname, this, method);
    }
    registerHelperCalls() {
    }
    // eslint-disable-next-line no-unused-vars
    calculateVerify(verify, values) {
        return false;
    }
    verify(values) {
        return this.calculateVerify(true, values);
    }
    calculateFreeInput(values) {
        return this.calculateVerify(false, values);
    }
    onVerifyFails(msg) {
        this.lastMessage = msg;
        if (this.enableVerifyExceptions) {
            throw new Error(msg);
        }
        return false;
    }
    valuesToScalar(values, start, count, bits, safe = false) {
        // assumes first less signficant a0,a1,a2 ...
        let result = 0n;
        const factor = 2n**BigInt(bits);
        const lsi = start + count - 1;
        for (let index = 0; index < count; ++index) {
            result = result * factor + BigInt(values[lsi - index]);
        }
        return result;
    }
    valuesToScalars(values, lengths = [], bits, safe = false) {
        // TODO: use safe
        // assumes first less signficant a0,a1,a2 ...
        const result = [];
        let index = 0;
        for (const _len of lengths) {
            const flags = _len & 0xFFFC00;
            const len = _len & 0x3FF;
            if (len == 1) {
                result.push(BigInt(values[index++]));
                continue;
            }            

            // put chunks in an array
            if (flags & VALUE_TO_SCALAR_ARRAY) {
                result.push(values.slice(index, index + len).map(x => BigInt(x)));
                continue;
            }

            // default behaviour, put chunks in one scalar to operate with it.
            result.push(this.valuesToScalar(values, index, len, bits, safe));
            index = index + len;
        }
        return result;
    }
    isEqual(v1, v2) {
        if (Array.isArray(v1)) {
            return Array.isArray(v2) && v1.length === v2.length && v1.every((e, i) => e === v2[i]);
        }
        return v1 === v2;
    }
    setColArray(col, value, rowIndex) {
        for (let index = 0; index < value.length; ++index) {
            col[index][rowIndex] = value[index];
        }      
    }
}